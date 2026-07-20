import { describe, it, expect } from "vitest";
import { parsearListaAnimais, resumirColagem } from "./animais";

describe("parsearListaAnimais — formatos aceitos", () => {
  it("aceita só a marcação, uma por linha", () => {
    const r = parsearListaAnimais("4P\n2L\n1P1L");
    expect(r.animais).toEqual([
      { marcacao: "4P", peso: null },
      { marcacao: "2L", peso: null },
      { marcacao: "1P1L", peso: null },
    ]);
    expect(r.invalidas).toHaveLength(0);
  });

  it("aceita marcação + peso separados por espaço, TAB, vírgula ou ponto e vírgula", () => {
    const r = parsearListaAnimais("4P 25.4\n2L\t26.1\n3P,24.8\n1L;27.2");
    expect(r.animais).toEqual([
      { marcacao: "4P", peso: 25.4 },
      { marcacao: "2L", peso: 26.1 },
      { marcacao: "3P", peso: 24.8 },
      { marcacao: "1L", peso: 27.2 },
    ]);
    expect(r.invalidas).toHaveLength(0);
  });

  it("aceita vírgula decimal (padrão brasileiro), inclusive com vírgula como separador", () => {
    const r = parsearListaAnimais("4P 25,4\n2L,26,1");
    expect(r.animais).toEqual([
      { marcacao: "4P", peso: 25.4 },
      { marcacao: "2L", peso: 26.1 },
    ]);
  });

  it("ignora linhas em branco e espaços extras sem reclamar", () => {
    const r = parsearListaAnimais("\n  4P   25.4  \n\n   \n2L\n");
    expect(r.animais).toEqual([
      { marcacao: "4P", peso: 25.4 },
      { marcacao: "2L", peso: null },
    ]);
    expect(r.invalidas).toHaveLength(0);
  });

  it("tolera separadores repetidos entre marcação e peso", () => {
    const r = parsearListaAnimais("4P , 25.4");
    expect(r.animais).toEqual([{ marcacao: "4P", peso: 25.4 }]);
  });
});

describe("parsearListaAnimais — linhas problemáticas", () => {
  it("reporta peso não numérico com o número da linha", () => {
    const r = parsearListaAnimais("4P 25.4\n2L abc\n3P 27");
    expect(r.animais).toEqual([
      { marcacao: "4P", peso: 25.4 },
      { marcacao: "3P", peso: 27 },
    ]);
    expect(r.invalidas).toHaveLength(1);
    expect(r.invalidas[0].numero).toBe(2);
    expect(r.invalidas[0].motivo).toContain("abc");
  });

  it("rejeita peso zero ou negativo", () => {
    const r = parsearListaAnimais("4P 0\n2L -3");
    expect(r.animais).toHaveLength(0);
    expect(r.invalidas).toHaveLength(2);
    expect(r.invalidas[0].motivo).toContain("maior que zero");
  });

  it("uma linha inválida não impede as demais de serem aproveitadas", () => {
    const r = parsearListaAnimais("4P 25\nLIXO ???\n2L 26");
    expect(r.animais.map((a) => a.marcacao)).toEqual(["4P", "2L"]);
    expect(r.invalidas).toHaveLength(1);
  });
});

describe("parsearListaAnimais — duplicatas", () => {
  it("ignora marcações que já existem no grupo (sem diferenciar maiúsculas)", () => {
    const r = parsearListaAnimais("4P 25\n2L 26", ["4p"]);
    expect(r.animais.map((a) => a.marcacao)).toEqual(["2L"]);
    expect(r.duplicadas).toEqual(["4P"]);
  });

  it("ignora duplicatas dentro da própria lista colada", () => {
    const r = parsearListaAnimais("4P\n2L\n4P 30");
    expect(r.animais.map((a) => a.marcacao)).toEqual(["4P", "2L"]);
    expect(r.duplicadas).toEqual(["4P"]);
  });
});

describe("resumirColagem", () => {
  it("resume contagem, duplicadas e linhas com problema", () => {
    const r = parsearListaAnimais("4P 25\n4P 26\nX ruim", []);
    const texto = resumirColagem(r);
    expect(texto).toContain("1 animal detectado");
    expect(texto).toContain("duplicada");
    expect(texto).toContain("1 linha(s) com problema");
  });

  it("usa plural corretamente", () => {
    expect(resumirColagem(parsearListaAnimais("4P\n2L"))).toContain("2 animais detectados");
  });
});
