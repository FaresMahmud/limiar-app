//! Motor de cálculo do limiar (LD50 / PWT) pelo método up-and-down de Dixon (1980).
//!
//! Implementa o procedimento de decodificação descrito no artigo (seção
//! "THE UP-AND-DOWN METHOD", steps 1-5, p. 453-455 do PDF em
//! `docs/referencia/dixon1980.pdf`) usando a Tabela 7 transcrita em
//! [`crate::dixon_table`].
//!
//! Fórmula (em espaço log10):  estimativa_log = log10(xf) + k · d
//!   - `xf` = força do último filamento/dose testado (valor real, ex.: gramas);
//!   - `k`  = valor tabelado (Dixon, Table 7), função da configuração O/X;
//!   - `d`  = intervalo médio entre níveis, em log10 (vem do cadastro de filamentos).
//! Limiar (valor real):  LIMIAR = 10 ^ (log10(xf) + k · d)
//! Ver `docs/DOMINIO.md`.
//!
//! CONVENÇÃO O/X (consistente em todo o projeto):
//!   - `Resposta::NaoRespondeu` = **O** (animal não retirou a pata / "alive").
//!   - `Resposta::Respondeu`    = **X** (animal retirou a pata / "dead").
//!
//! Regra up-and-down: sobe o nível após **O**, desce após **X**.

use crate::dixon_table::{buscar, INCREMENTO_TERCEIRO_DECIMAL};

/// Resposta de um animal a uma aplicação (um teste da série).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resposta {
    /// O — não retirou a pata (sem resposta).
    NaoRespondeu,
    /// X — retirou a pata (respondeu).
    Respondeu,
}

impl Resposta {
    fn respondeu(self) -> bool {
        matches!(self, Resposta::Respondeu)
    }
}

/// Erros possíveis do cálculo — sempre retornados como `Result`, nunca `panic`.
/// (O software antigo travava com sequências fora da tabela; aqui devolvemos uma
/// mensagem clara em vez de quebrar.)
#[derive(Debug, Clone, PartialEq)]
pub enum DixonError {
    /// Nenhuma resposta informada.
    SerieVazia,
    /// Quantidade de doses diferente da quantidade de respostas.
    TamanhosIncompativeis { respostas: usize, doses: usize },
    /// Uma dose <= 0 (log10 indefinido).
    DoseInvalida { indice: usize, valor: f64 },
    /// `d` não é um número finito e positivo.
    DInvalido { valor: f64 },
    /// Todas as respostas iguais: não há reversão, logo não há "segunda parte" e a
    /// Tabela 7 não define estimativa. (Ex.: animal respondeu a todos os filamentos.)
    SerieSemReversao,
    /// N nominal fora do que a Tabela 7 cobre (2..=6). Para N > 6, ver Dixon (7)
    /// (tabelas adicionais) — não implementado neste motor. LIMITAÇÃO CONHECIDA.
    NNominalForaDaTabela { n_nominal: usize },
    /// Configuração não encontrada na tabela (não deveria ocorrer com a tabela
    /// completa; indica bug de decodificação se acontecer).
    ConfiguracaoNaoEncontrada { n_nominal: usize, second_part: String },
}

impl std::fmt::Display for DixonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DixonError::SerieVazia => write!(f, "Série de testes vazia."),
            DixonError::TamanhosIncompativeis { respostas, doses } => write!(
                f,
                "Número de respostas ({respostas}) difere do número de doses ({doses})."
            ),
            DixonError::DoseInvalida { indice, valor } => write!(
                f,
                "Dose inválida no índice {indice} ({valor}); deve ser > 0 para log10."
            ),
            DixonError::DInvalido { valor } => {
                write!(f, "Intervalo d inválido ({valor}); deve ser finito e > 0.")
            }
            DixonError::SerieSemReversao => write!(
                f,
                "Série sem reversão (todas as respostas iguais): a Tabela 7 de Dixon \
                 não define um limiar para este caso."
            ),
            DixonError::NNominalForaDaTabela { n_nominal } => write!(
                f,
                "N nominal = {n_nominal} fora do intervalo coberto pela Tabela 7 (2 a 6). \
                 Para N > 6, ver tabelas adicionais em Dixon (1980), ref. (7)."
            ),
            DixonError::ConfiguracaoNaoEncontrada { n_nominal, second_part } => write!(
                f,
                "Configuração não encontrada na Tabela 7 (N={n_nominal}, segunda parte \
                 \"{second_part}\")."
            ),
        }
    }
}

impl std::error::Error for DixonError {}

/// Resultado detalhado da estimativa, com os valores intermediários para
/// rastreabilidade científica (permite auditar como o limiar foi obtido).
#[derive(Debug, Clone, PartialEq)]
pub struct Estimativa {
    /// Limiar em valor real (mesma unidade das doses, ex.: gramas): 10^(log10(xf)+k·d).
    pub limiar: f64,
    /// Estimativa em espaço log10 (log10(xf) + k·d) — é o "0.852" do exemplo do artigo.
    pub estimativa_log: f64,
    /// Valor de k efetivamente aplicado (já com sinal e eventual incremento).
    pub k: f64,
    /// log10 da última dose testada (o `xf` do artigo).
    pub xf_log10: f64,
    /// `d` usado (intervalo médio em log10).
    pub d: f64,
    /// N nominal apurado.
    pub n_nominal: usize,
    /// Rótulo canônico da segunda parte usado na consulta (começa com 'X').
    pub second_part: String,
    /// Índice de coluna usado (0=O, 1=OO, 2=OOO, 3=OOOO).
    pub coluna: usize,
    /// `true` se a série começou com X e o sinal de k foi invertido (entrada "pelo pé").
    pub sinal_invertido: bool,
    /// `true` se o incremento de +0,001 (células "+1") foi aplicado.
    pub incremento_aplicado: bool,
    /// Erro padrão do LD50 (fração de σ) do bloco N usado.
    pub erro_padrao_sigma: f64,
}

/// Estima o limiar (LD50/PWT) a partir da série de respostas up-and-down.
///
/// # Parâmetros
/// - `respostas`: sequência **cronológica** de O/X (do primeiro ao último teste).
/// - `doses`: força real de cada nível testado (ex.: gramas), na mesma ordem
///   cronológica das respostas. `doses.last()` é o `xf` (último nível testado).
/// - `d`: intervalo médio entre níveis em **log10** (calculado a partir do conjunto
///   de filamentos cadastrado — ver `docs/DOMINIO.md`).
///
/// # Retorno
/// [`Estimativa`] com o limiar e os valores intermediários, ou [`DixonError`].
pub fn estimar_limiar(
    respostas: &[Resposta],
    doses: &[f64],
    d: f64,
) -> Result<Estimativa, DixonError> {
    // ---- Validações de entrada ----
    if respostas.is_empty() {
        return Err(DixonError::SerieVazia);
    }
    if respostas.len() != doses.len() {
        return Err(DixonError::TamanhosIncompativeis {
            respostas: respostas.len(),
            doses: doses.len(),
        });
    }
    if !d.is_finite() || d <= 0.0 {
        return Err(DixonError::DInvalido { valor: d });
    }
    for (i, &dose) in doses.iter().enumerate() {
        if !dose.is_finite() || dose <= 0.0 {
            return Err(DixonError::DoseInvalida { indice: i, valor: dose });
        }
    }

    // ---- Step 3: N' e N nominal ----
    // Primeira parte = corrida inicial de respostas iguais (m elementos).
    let lider = respostas[0];
    let m = respostas.iter().take_while(|&&x| x == lider).count();

    // Todas iguais => sem segunda parte => sem reversão => sem estimativa.
    if m == respostas.len() {
        return Err(DixonError::SerieSemReversao);
    }

    // Segunda parte = tudo após a primeira parte. N nominal = |segunda parte| + 1.
    // (Equivale a N = N' - (m - 1) do artigo.)
    let cauda = &respostas[m..];
    let n_nominal = cauda.len() + 1;
    if !(2..=6).contains(&n_nominal) {
        return Err(DixonError::NNominalForaDaTabela { n_nominal });
    }

    // ---- Entrada "pelo pé": se a série começa com X, troca O<->X e nega k. ----
    let sinal_invertido = lider.respondeu();
    let sinal = if sinal_invertido { -1.0 } else { 1.0 };

    // Rótulo canônico da segunda parte (sempre começa com 'X').
    // Se `swap`, invertemos cada resposta (equivale a entrar pela base da tabela).
    let swap = sinal_invertido;
    let second_part: String = cauda
        .iter()
        .map(|&x| {
            let bit = x.respondeu() ^ swap; // XOR: inverte quando swap
            if bit { 'X' } else { 'O' }
        })
        .collect();

    // ---- Step 4/5: coluna = min(m, 4). Para m > 4, mantém a última coluna (OOOO). ----
    let coluna = m.min(4) - 1; // 0..=3

    let (linha, erro_padrao_sigma) = buscar(n_nominal, &second_part)
        .ok_or_else(|| DixonError::ConfiguracaoNaoEncontrada {
            n_nominal,
            second_part: second_part.clone(),
        })?;

    // Valor-base de k na coluna apurada.
    let mut k_base = linha.k[coluna];

    // Incremento "+1" (+0,001): apenas nas 5 células marcadas, na última coluna,
    // e só quando m > 4 (condição validada com o pesquisador). Ver dixon_table.rs.
    let incremento_aplicado = coluna == 3 && linha.increment_last_col && m > 4;
    if incremento_aplicado {
        k_base += INCREMENTO_TERCEIRO_DECIMAL;
    }

    let k = sinal * k_base;

    // ---- Fórmula ----
    let xf = *doses.last().expect("doses não-vazio já validado");
    let xf_log10 = xf.log10();
    let estimativa_log = xf_log10 + k * d;
    let limiar = 10f64.powf(estimativa_log);

    Ok(Estimativa {
        limiar,
        estimativa_log,
        k,
        xf_log10,
        d,
        n_nominal,
        second_part,
        coluna,
        sinal_invertido,
        incremento_aplicado,
        erro_padrao_sigma,
    })
}

// =============================================================================
// TESTES
// =============================================================================
#[cfg(test)]
mod tests {
    use super::Resposta::{NaoRespondeu as O, Respondeu as X};
    use super::*;

    const TOL: f64 = 0.001;

    /// TESTE OBRIGATÓRIO — Exemplo resolvido do artigo (Figure 6, p. 455).
    ///
    /// "For this series OXXOXO the estimate of LD50 is 0.602 + 0.831(0.301) = 0.852."
    /// Doses reais (concentrações %): 8, 16, 8, 4, 8, 4  (log10: 0.903, 1.204, 0.602...).
    /// xf = última dose = 4% ; d = 0.301 ; k esperado = 0.831 ; estimativa log = 0.852.
    #[test]
    fn figura6_exemplo_do_artigo() {
        let respostas = [O, X, X, O, X, O]; // OXXOXO (cronológico)
        let doses = [8.0, 16.0, 8.0, 4.0, 8.0, 4.0];
        let d = 0.301;

        let e = estimar_limiar(&respostas, &doses, d).expect("deve calcular");

        assert_eq!(e.n_nominal, 6, "N nominal");
        assert_eq!(e.second_part, "XXOXO", "segunda parte canônica");
        assert_eq!(e.coluna, 0, "coluna O (primeira parte = 1)");
        assert!(!e.sinal_invertido, "série começa com O, sem inversão");
        assert!((e.k - 0.831).abs() < TOL, "k = {} (esperado 0.831)", e.k);
        // O "0.852" do artigo é a estimativa em espaço log10:
        assert!(
            (e.estimativa_log - 0.852).abs() < TOL,
            "estimativa_log = {} (esperado 0.852)",
            e.estimativa_log
        );
        // Limiar em valor real = 10^0.852 ≈ 7.11%.
        assert!((e.limiar - 10f64.powf(0.852)).abs() < 0.01, "limiar = {}", e.limiar);
    }

    /// Entrada "pelo pé": a série espelhada (O<->X) de Figure 6 deve dar k = -0.831.
    /// OXXOXO espelhado = XOOXOX. Valida a inversão de sinal (sintético, derivado
    /// do exemplo do artigo por simetria O/X).
    #[test]
    fn entrada_pelo_pe_inverte_sinal() {
        let respostas = [X, O, O, X, O, X]; // XOOXOX = espelho de OXXOXO
        let doses = [4.0, 2.0, 4.0, 8.0, 4.0, 8.0]; // qualquer; testamos k e sinal
        let d = 0.301;

        let e = estimar_limiar(&respostas, &doses, d).expect("deve calcular");

        assert!(e.sinal_invertido, "série começa com X → entrada pelo pé");
        assert_eq!(e.second_part, "XXOXO", "mesma segunda parte canônica");
        assert!((e.k + 0.831).abs() < TOL, "k = {} (esperado -0.831)", e.k);
    }

    /// Consulta direta de valores da tabela via decodificação (N=3, N=4).
    #[test]
    fn valores_basicos_da_tabela() {
        // Série O,X  -> primeira parte "O" (m=1), segunda parte "X", N=2, col O.
        let e = estimar_limiar(&[O, X], &[1.0, 2.0], 0.3).unwrap();
        assert_eq!(e.n_nominal, 2);
        assert_eq!(e.second_part, "X");
        assert!((e.k - (-0.500)).abs() < TOL, "k N2/X/O = {}", e.k);

        // Série O,X,O -> "O" + "XO", N=3, col O, k(XO,O)=0.842.
        let e = estimar_limiar(&[O, X, O], &[1.0, 2.0, 1.0], 0.3).unwrap();
        assert_eq!(e.n_nominal, 3);
        assert_eq!(e.second_part, "XO");
        assert!((e.k - 0.842).abs() < TOL, "k N3/XO/O = {}", e.k);

        // Primeira parte com 2 iguais -> coluna OO. Série O,O,X -> "OO"+"X", N=2, col OO.
        let e = estimar_limiar(&[O, O, X], &[1.0, 2.0, 4.0], 0.3).unwrap();
        assert_eq!(e.n_nominal, 2);
        assert_eq!(e.coluna, 1, "coluna OO");
        assert!((e.k - (-0.388)).abs() < TOL, "k N2/X/OO = {}", e.k);
    }

    /// Caso patológico que travava o software antigo: 4+ respostas iguais no início.
    /// OOOOO XXXX: m=5 (>4) → coluna OOOO capada + incremento +0,001 na célula "+1".
    /// N=5, segunda parte "XXXX", k base 1.000 → 1.001.
    #[test]
    fn corrida_inicial_longa_usa_incremento() {
        let respostas = [O, O, O, O, O, X, X, X, X];
        let doses = [1.0, 2.0, 4.0, 8.0, 16.0, 8.0, 4.0, 2.0, 1.0];
        let d = 0.301;

        let e = estimar_limiar(&respostas, &doses, d).expect("não deve travar");
        assert_eq!(e.n_nominal, 5);
        assert_eq!(e.second_part, "XXXX");
        assert_eq!(e.coluna, 3, "coluna OOOO (m capado em 4)");
        assert!(e.incremento_aplicado, "m>4 deve aplicar +0,001");
        assert!((e.k - 1.001).abs() < TOL, "k = {} (esperado 1.001)", e.k);
    }

    /// Com m == 4 exatamente, NÃO aplica o incremento (usa valor-base 1.000).
    #[test]
    fn quatro_iguais_nao_aplica_incremento() {
        let respostas = [O, O, O, O, X, X, X, X]; // m=4
        let doses = [1.0, 2.0, 4.0, 8.0, 4.0, 2.0, 1.0, 0.5];
        let e = estimar_limiar(&respostas, &doses, 0.301).unwrap();
        assert_eq!(e.coluna, 3);
        assert!(!e.incremento_aplicado, "m==4 não aplica incremento");
        assert!((e.k - 1.000).abs() < TOL, "k = {} (esperado 1.000)", e.k);
    }

    /// Erros claros (não panic) para entradas fora do domínio.
    #[test]
    fn erros_bem_definidos() {
        assert_eq!(estimar_limiar(&[], &[], 0.3), Err(DixonError::SerieVazia));

        // Todas iguais → sem reversão.
        assert_eq!(
            estimar_limiar(&[X, X, X], &[1.0, 0.5, 0.25], 0.3),
            Err(DixonError::SerieSemReversao)
        );

        // N nominal > 6 (segunda parte com 6 elementos).
        let resp = [O, X, X, X, X, X, X]; // "O" + "XXXXXX", N=7
        let doses = [1.0; 7];
        assert_eq!(
            estimar_limiar(&resp, &doses, 0.3),
            Err(DixonError::NNominalForaDaTabela { n_nominal: 7 })
        );

        // Tamanhos incompatíveis.
        assert_eq!(
            estimar_limiar(&[O, X], &[1.0], 0.3),
            Err(DixonError::TamanhosIncompativeis { respostas: 2, doses: 1 })
        );

        // Dose inválida.
        assert_eq!(
            estimar_limiar(&[O, X], &[1.0, 0.0], 0.3),
            Err(DixonError::DoseInvalida { indice: 1, valor: 0.0 })
        );

        // d inválido.
        assert_eq!(
            estimar_limiar(&[O, X], &[1.0, 2.0], 0.0),
            Err(DixonError::DInvalido { valor: 0.0 })
        );
    }
}
