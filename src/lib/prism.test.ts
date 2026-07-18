import { describe, it, expect } from "vitest";
import {
  montarTabelaPrism,
  tabelaPrismParaTsv,
  gerarTsvPrism,
  type PrismExperimento,
  type PrismLimiar,
} from "./prism";

// Experimento de exemplo: 2 grupos, timepoints propositalmente FORA de ordem no
// array (ordem 0 e 1) para checar que a ordenação usa `ordem`, não a posição.
const experimento: PrismExperimento = {
  grupos: [
    { nome: "Controle", animais: [{ marcacao: "A1" }, { marcacao: "A2" }] },
    { nome: "Tratado", animais: [{ marcacao: "B1" }] },
  ],
  timepoints: [
    { rotulo: "1h", ordem: 1 },
    { rotulo: "Basal", ordem: 0 },
  ],
};

// Limiares brutos. Repare que falta (Tratado/B1/1h): esse animal não foi testado
// nesse timepoint → deve virar célula VAZIA, sem quebrar.
const limiares: PrismLimiar[] = [
  { grupo: "Controle", animal: "A1", timepoint: "Basal", limiar: 4.2 },
  { grupo: "Controle", animal: "A2", timepoint: "Basal", limiar: 3.8 },
  { grupo: "Tratado", animal: "B1", timepoint: "Basal", limiar: 4.1 },
  { grupo: "Controle", animal: "A1", timepoint: "1h", limiar: 3.9 },
  { grupo: "Controle", animal: "A2", timepoint: "1h", limiar: 3.95 },
  // (Tratado/B1/1h) AUSENTE de propósito.
];

describe("montarTabelaPrism", () => {
  it("coloca colunas na ordem grupo→animal com cabeçalho Grupo_Marcacao", () => {
    const t = montarTabelaPrism(experimento, limiares);
    expect(t.colunasAnimais).toEqual(["Controle_A1", "Controle_A2", "Tratado_B1"]);
  });

  it("ordena as linhas por `ordem` do timepoint (cronológico), não pela posição no array", () => {
    const t = montarTabelaPrism(experimento, limiares);
    expect(t.linhas.map((l) => l.timepoint)).toEqual(["Basal", "1h"]);
  });

  it("preenche as células e deixa VAZIA a combinação sem limiar", () => {
    const t = montarTabelaPrism(experimento, limiares);
    // Basal: todos os 3 animais têm valor (vírgula decimal)
    expect(t.linhas[0].valores).toEqual(["4,2000", "3,8000", "4,1000"]);
    // 1h: Tratado/B1 ausente → última célula vazia
    expect(t.linhas[1].valores).toEqual(["3,9000", "3,9500", ""]);
  });

  it("usa vírgula decimal e respeita o número de casas (opção `decimais`)", () => {
    const t = montarTabelaPrism(experimento, limiares, { decimais: 2 });
    expect(t.linhas[0].valores[0]).toBe("4,20");
    expect(t.linhas[0].valores[0]).not.toContain(".");
  });
});

describe("tabelaPrismParaTsv / gerarTsvPrism", () => {
  it("gera o TSV exato esperado (TAB entre células, \\n entre linhas)", () => {
    const tsv = gerarTsvPrism(experimento, limiares);
    const esperado = [
      "Timepoint\tControle_A1\tControle_A2\tTratado_B1",
      "Basal\t4,2000\t3,8000\t4,1000",
      "1h\t3,9000\t3,9500\t", // célula final vazia (Tratado/B1/1h ausente)
    ].join("\n");
    expect(tsv).toBe(esperado);
  });

  it("cada linha tem o mesmo número de colunas do cabeçalho (retangular)", () => {
    const tsv = gerarTsvPrism(experimento, limiares);
    const linhas = tsv.split("\n").map((l) => l.split("\t"));
    const nCols = linhas[0].length;
    for (const l of linhas) expect(l.length).toBe(nCols);
    expect(nCols).toBe(4); // Timepoint + 3 animais
  });

  it("lida com limiar null/NaN como célula vazia", () => {
    const lim: PrismLimiar[] = [
      { grupo: "Controle", animal: "A1", timepoint: "Basal", limiar: null },
      { grupo: "Controle", animal: "A2", timepoint: "Basal", limiar: NaN },
    ];
    const t = montarTabelaPrism(experimento, lim);
    expect(t.linhas[0].valores[0]).toBe("");
    expect(t.linhas[0].valores[1]).toBe("");
  });

  it("sanitiza rótulos com TAB/quebra de linha para não corromper o TSV", () => {
    const exp: PrismExperimento = {
      grupos: [{ nome: "Grupo\tX", animais: [{ marcacao: "A\n1" }] }],
      timepoints: [{ rotulo: "Ba\tsal", ordem: 0 }],
    };
    const tsv = gerarTsvPrism(exp, []);
    // Nenhuma TAB "extra" além das 1 que separa Timepoint da única coluna.
    expect(tsv.split("\n")[0]).toBe("Timepoint\tGrupo X_A 1");
    expect(tsv.split("\n")[1].split("\t").length).toBe(2);
  });
});
