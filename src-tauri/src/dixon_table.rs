//! Tabela 7 de Dixon (1980) — estimativas de máxima verossimilhança do LD50.
//!
//! FONTE EXATA: Dixon, W.J. (1980). "Efficient Analysis of Experimental
//! Observations". Annu. Rev. Pharmacol. Toxicol. 20:441-462. **Table 7, p. 454**
//! (arquivo `docs/referencia/dixon1980.pdf`, página impressa 454 = índice 13 do PDF).
//!
//! Transcrito **exatamente** da tabela original (conferido contra o texto embutido
//! do PDF e contra a imagem renderizada da página). **NÃO alterar nenhum valor sem
//! reconferir no PDF** — erros aqui produzem limiares cientificamente inválidos.
//!
//! ------------------------------------------------------------------------------
//! ESTRUTURA DA TABELA (bidirecional — este é o ponto que mais confunde)
//! ------------------------------------------------------------------------------
//! Uma série up-and-down é dividida em:
//!   - "primeira parte" (first part): a corrida inicial de respostas IGUAIS;
//!   - "segunda parte" (second part): o restante da série (rótulo da LINHA).
//!
//! - As LINHAS são rotuladas pela segunda parte. Nas linhas do topo (entrada
//!   "pela cabeça") o rótulo SEMPRE começa com `X`.
//! - As COLUNAS `O, OO, OOO, OOOO` indicam quantas respostas iguais formam a
//!   primeira parte (1, 2, 3, 4+).
//! - N nominal = (comprimento da segunda parte) + 1.
//! - Erro padrão do LD50 por bloco N, como fração de sigma (σ).
//!
//! ENTRADA "PELO PÉ" (série que começa com `X`, não `O`):
//!   O artigo instrui a entrar pela base da tabela, usando os rótulos de coluna
//!   invertidos (X, XX, XXX, XXXX) e **invertendo o sinal de k**. Isto é
//!   equivalente a: trocar O<->X na série inteira, consultar normalmente e negar k.
//!   Por isso guardamos aqui APENAS as linhas cujo rótulo começa com `X`; a
//!   inversão é tratada no motor (`dixon.rs`).
//!
//! ------------------------------------------------------------------------------
//! O SOBRESCRITO "+1" (cinco células) — ver `increment_last_col`
//! ------------------------------------------------------------------------------
//! Cinco células na última coluna (OOOO) aparecem no original com um sobrescrito
//! "+1": N=5 `XXXX` (1.000⁺¹) e N=6 `XXOXX` (0.504⁺¹), `XXXOX` (0.252⁺¹),
//! `XXXXO` (2.014⁺¹), `XXXXX` (1.496⁺¹).
//!
//! O Step 4 do artigo (p. 455) explica: "If the series begins with more than four
//! like responses (i.e. N' - N >= 3) the entry in the final column of Table 7 can
//! be used (except for five tabular entries where an additional increment in the
//! third decimal place is indicated)." Ou seja, "+1" = **+0,001 no terceiro
//! decimal**.
//!
//! **Condição de aplicação (validada com o pesquisador):** o incremento de +0,001
//! é aplicado apenas quando a primeira parte tem **mais de 4** respostas iguais
//! (m > 4). Com m == 4 exatamente, usa-se o valor-base da coluna OOOO. O impacto é
//! ínfimo (0,001 em log10 ≈ 0,23% no limiar) e só afeta séries com >= 5 respostas
//! iguais no início.

/// Uma linha da Tabela 7: rótulo da segunda parte + os 4 valores de k das colunas
/// O, OO, OOO, OOOO.
#[derive(Debug, Clone, Copy)]
pub struct DixonRow {
    /// Rótulo da segunda parte da série (sempre começa com 'X'), ex.: "XXOXO".
    pub second_part: &'static str,
    /// Valores de k para as colunas [O, OO, OOO, OOOO] (primeira parte = 1..4+).
    pub k: [f64; 4],
    /// `true` nas 5 células com sobrescrito "+1" na coluna OOOO (índice 3):
    /// incremento de +0,001 indicado pelo artigo. Ver nota acima e `dixon.rs`.
    pub increment_last_col: bool,
}

/// Um bloco N da tabela (N nominal + erro padrão + linhas).
#[derive(Debug, Clone, Copy)]
pub struct DixonBlock {
    /// N nominal (2 a 6).
    pub n: u8,
    /// Erro padrão do LD50, como fração de sigma (σ).
    pub std_error_sigma: f64,
    /// Linhas do bloco (uma por configuração possível da segunda parte).
    pub rows: &'static [DixonRow],
}

// Atalhos para deixar a transcrição legível.
const fn r(second_part: &'static str, k: [f64; 4]) -> DixonRow {
    DixonRow { second_part, k, increment_last_col: false }
}
const fn ri(second_part: &'static str, k: [f64; 4]) -> DixonRow {
    DixonRow { second_part, k, increment_last_col: true }
}

// -----------------------------------------------------------------------------
// N = 2 — erro padrão 0.88 σ
// -----------------------------------------------------------------------------
static ROWS_N2: &[DixonRow] = &[
    //          O        OO       OOO      OOOO
    r("X",  [-0.500,  -0.388,  -0.378,  -0.377]),
];

// -----------------------------------------------------------------------------
// N = 3 — erro padrão 0.76 σ
// -----------------------------------------------------------------------------
static ROWS_N3: &[DixonRow] = &[
    r("XO", [ 0.842,   0.890,   0.894,   0.894]),
    r("XX", [-0.178,   0.000,   0.026,   0.028]),
];

// -----------------------------------------------------------------------------
// N = 4 — erro padrão 0.67 σ
// -----------------------------------------------------------------------------
static ROWS_N4: &[DixonRow] = &[
    r("XOO", [ 0.299,  0.314,   0.315,   0.315]),
    r("XOX", [-0.500, -0.439,  -0.432,  -0.432]),
    r("XXO", [ 1.000,  1.122,   1.139,   1.140]),
    r("XXX", [ 0.194,  0.449,   0.500,   0.506]),
];

// -----------------------------------------------------------------------------
// N = 5 — erro padrão 0.61 σ
// -----------------------------------------------------------------------------
static ROWS_N5: &[DixonRow] = &[
    r ("XOOO", [-0.157, -0.154, -0.154, -0.154]),
    r ("XOOX", [-0.878, -0.861, -0.860, -0.860]),
    r ("XOXO", [ 0.701,  0.737,  0.741,  0.741]),
    r ("XOXX", [ 0.084,  0.169,  0.181,  0.182]),
    r ("XXOO", [ 0.305,  0.372,  0.380,  0.381]),
    r ("XXOX", [-0.305, -0.169, -0.144, -0.142]),
    r ("XXXO", [ 1.288,  1.500,  1.544,  1.549]),
    ri("XXXX", [ 0.555,  0.897,  0.985,  1.000]), // 1.000⁺¹  (+0.001)
];

// -----------------------------------------------------------------------------
// N = 6 — erro padrão 0.56 σ
// -----------------------------------------------------------------------------
static ROWS_N6: &[DixonRow] = &[
    r ("XOOOO", [-0.547, -0.547, -0.547, -0.547]),
    r ("XOOOX", [-1.250, -1.247, -1.246, -1.246]),
    r ("XOOXO", [ 0.372,  0.380,  0.381,  0.381]),
    r ("XOOXX", [-0.169, -0.144, -0.142, -0.142]),
    r ("XOXOO", [ 0.022,  0.039,  0.040,  0.040]),
    r ("XOXOX", [-0.500, -0.458, -0.453, -0.453]),
    r ("XOXXO", [ 1.169,  1.237,  1.247,  1.248]),
    r ("XOXXX", [ 0.611,  0.732,  0.756,  0.758]),
    r ("XXOOO", [-0.296, -0.266, -0.263, -0.263]),
    r ("XXOOX", [-0.831, -0.763, -0.753, -0.752]),
    r ("XXOXO", [ 0.831,  0.935,  0.952,  0.954]),
    ri("XXOXX", [ 0.296,  0.463,  0.500,  0.504]), // 0.504⁺¹  (+0.001)
    r ("XXXOO", [ 0.500,  0.648,  0.678,  0.681]),
    ri("XXXOX", [-0.043,  0.187,  0.244,  0.252]), // 0.252⁺¹  (+0.001)
    ri("XXXXO", [ 1.603,  1.917,  2.000,  2.014]), // 2.014⁺¹  (+0.001)
    ri("XXXXX", [ 0.893,  1.329,  1.465,  1.496]), // 1.496⁺¹  (+0.001)
];

/// A Tabela 7 completa (N = 2 a 6). Fonte: Dixon 1980, Table 7, p. 454.
pub static DIXON_TABLE: &[DixonBlock] = &[
    DixonBlock { n: 2, std_error_sigma: 0.88, rows: ROWS_N2 },
    DixonBlock { n: 3, std_error_sigma: 0.76, rows: ROWS_N3 },
    DixonBlock { n: 4, std_error_sigma: 0.67, rows: ROWS_N4 },
    DixonBlock { n: 5, std_error_sigma: 0.61, rows: ROWS_N5 },
    DixonBlock { n: 6, std_error_sigma: 0.56, rows: ROWS_N6 },
];

/// Incremento aplicado às 5 células marcadas com "+1" (terceiro decimal). Ver nota.
pub const INCREMENTO_TERCEIRO_DECIMAL: f64 = 0.001;

/// Busca uma linha pelo N nominal e pelo rótulo da segunda parte (canônico, começa
/// com 'X'). Retorna também o erro padrão (fração de σ) do bloco.
pub fn buscar(n_nominal: usize, second_part: &str) -> Option<(&'static DixonRow, f64)> {
    let bloco = DIXON_TABLE.iter().find(|b| b.n as usize == n_nominal)?;
    let linha = bloco.rows.iter().find(|row| row.second_part == second_part)?;
    Some((linha, bloco.std_error_sigma))
}
