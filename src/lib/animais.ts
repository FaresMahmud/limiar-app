// -----------------------------------------------------------------------------
// Colagem em massa de animais ("Colar lista") no card de grupo do wizard.
//
// O pesquisador normalmente já tem a lista de marcações digitada em outro lugar
// (planilha, caderno digitalizado, mensagem). Digitar animal por animal é a parte
// mais repetitiva do cadastro, então aceitamos colar tudo de uma vez.
//
// Formato aceito: UM ANIMAL POR LINHA, com peso opcional.
//   4P            -> marcação "4P", sem peso
//   4P 25.4       -> marcação "4P", peso 25.4
//   4P;25,4       -> idem (aceita vírgula decimal, padrão brasileiro)
//   4P,25.4       -> idem
//   4P<TAB>25.4   -> idem (colagem direta do Excel)
//
// Regra: a marcação é o PRIMEIRO token da linha; o resto (se houver) é o peso.
// Separadores aceitos entre eles: TAB, ponto e vírgula, vírgula ou espaços.
// Linhas em branco são ignoradas silenciosamente.
//
// Módulo PURO (sem DOM) para ser testável — ver src/lib/animais.test.ts.
// -----------------------------------------------------------------------------

export interface AnimalColado {
  marcacao: string;
  /** Peso em gramas, ou `null` quando não informado. */
  peso: number | null;
}

export interface LinhaInvalida {
  /** Número da linha (1-indexado) como o usuário vê no textarea. */
  numero: number;
  conteudo: string;
  motivo: string;
}

export interface ResultadoColagem {
  /** Animais válidos, na ordem em que aparecem. */
  animais: AnimalColado[];
  /** Linhas que não puderam ser interpretadas (com o motivo). */
  invalidas: LinhaInvalida[];
  /** Marcações ignoradas por já existirem (no grupo ou repetidas na própria lista). */
  duplicadas: string[];
}

/** Separa a linha em [marcação, resto] no primeiro separador encontrado. */
function separarLinha(linha: string): [string, string] {
  const t = linha.trim();
  const idx = t.search(/[\t;,\s]/);
  if (idx === -1) return [t, ""];
  const marcacao = t.slice(0, idx);
  // remove os separadores que sobraram no início do resto (ex.: "4P , 25.4")
  const resto = t.slice(idx).replace(/^[\t;,\s]+/, "").trim();
  return [marcacao, resto];
}

/**
 * Interpreta o texto colado e devolve os animais válidos, as linhas inválidas e
 * as marcações duplicadas.
 *
 * @param texto texto colado pelo usuário (uma linha por animal)
 * @param marcacoesExistentes marcações já presentes no grupo (para não duplicar);
 *        a comparação ignora maiúsculas/minúsculas e espaços nas pontas.
 */
export function parsearListaAnimais(
  texto: string,
  marcacoesExistentes: string[] = []
): ResultadoColagem {
  const animais: AnimalColado[] = [];
  const invalidas: LinhaInvalida[] = [];
  const duplicadas: string[] = [];

  const vistas = new Set(
    marcacoesExistentes.map((m) => m.trim().toLowerCase()).filter((m) => m !== "")
  );

  const linhas = (texto ?? "").split(/\r?\n/);

  linhas.forEach((linhaBruta, i) => {
    const numero = i + 1;
    const linha = linhaBruta.trim();
    if (linha === "") return; // linha em branco: ignora sem reclamar

    const [marcacao, restoPeso] = separarLinha(linha);

    if (marcacao === "") {
      invalidas.push({ numero, conteudo: linhaBruta, motivo: "sem marcação" });
      return;
    }

    let peso: number | null = null;
    if (restoPeso !== "") {
      const pesoTexto = restoPeso.replace(/\s/g, "");
      if (!/^\d+(?:[.,]\d+)?$/.test(pesoTexto)) {
        invalidas.push({
          numero,
          conteudo: linhaBruta,
          motivo: `peso "${restoPeso}" não é um número válido`,
        });
        return;
      }
      const n = Number(pesoTexto.replace(",", "."));
      if (!Number.isFinite(n) || n <= 0) {
        invalidas.push({
          numero,
          conteudo: linhaBruta,
          motivo: "o peso deve ser maior que zero",
        });
        return;
      }
      peso = n;
    }

    const chave = marcacao.toLowerCase();
    if (vistas.has(chave)) {
      duplicadas.push(marcacao);
      return;
    }
    vistas.add(chave);
    animais.push({ marcacao, peso });
  });

  return { animais, invalidas, duplicadas };
}

/** Resumo curto para mostrar na interface antes de confirmar a colagem. */
export function resumirColagem(r: ResultadoColagem): string {
  const partes: string[] = [];
  partes.push(
    r.animais.length === 1 ? "1 animal detectado" : `${r.animais.length} animais detectados`
  );
  if (r.duplicadas.length > 0) {
    partes.push(
      `${r.duplicadas.length} duplicada(s) ignorada(s): ${r.duplicadas.join(", ")}`
    );
  }
  if (r.invalidas.length > 0) {
    partes.push(`${r.invalidas.length} linha(s) com problema`);
  }
  return partes.join(" · ");
}
