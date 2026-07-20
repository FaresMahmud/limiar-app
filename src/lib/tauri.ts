import { invoke } from '@tauri-apps/api/core';

const IS_TAURI = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__ !== undefined;

// Banco em memória mockado para desenvolvimento/teste no navegador (web pura)
let mockConjuntos = [
  {
    id: 1,
    nome: "Kit de camundongo padrão do laboratório",
    descricao: "Valores típicos do laboratório principal",
    d: 0.3835,
    criado_em: new Date().toISOString(),
    atualizado_em: new Date().toISOString(),
    valores: [0.02, 0.07, 0.16, 0.4, 1.0, 2.0, 4.0]
  },
  {
    id: 2,
    nome: "Kit da colega (Dixon Figura 6)",
    descricao: "Valores usados no exemplo do artigo de Dixon",
    d: 0.301,
    criado_em: new Date().toISOString(),
    atualizado_em: new Date().toISOString(),
    valores: [1.0, 2.0, 4.0, 8.0, 16.0]
  }
];

let mockExperimentos = [
  {
    id: 1,
    nome: "Estudo Basal Morfina 2026",
    descricao: "Avaliação do limiar nociceptivo após indução de hiperalgesia e tratamento com Morfina.",
    conjunto_id: 1,
    conjunto_nome: "Kit de camundongo padrão do laboratório",
    responsavel: "Dr. Fares Mahmud",
    criado_em: new Date().toISOString(),
    atualizado_em: new Date().toISOString(),
    timepoints: [
      { id: 1, experimento_id: 1, rotulo: "basal 1", ordem: 0, opcional: 0 },
      { id: 2, experimento_id: 1, rotulo: "indução", ordem: 1, opcional: 0 },
      { id: 3, experimento_id: 1, rotulo: "basal 2", ordem: 2, opcional: 0 },
      { id: 4, experimento_id: 1, rotulo: "tratamento", ordem: 3, opcional: 0 },
      { id: 5, experimento_id: 1, rotulo: "1h", ordem: 4, opcional: 0 },
      { id: 6, experimento_id: 1, rotulo: "2h", ordem: 5, opcional: 0 }
    ],
    grupos: [
      {
        id: 1,
        experimento_id: 1,
        nome: "Grupo Controle (Salina)",
        cor: "#3b82f6",
        animais: [
          { id: 1, experimento_id: 1, grupo_id: 1, marcacao: "1P", peso: 24.5 },
          { id: 2, experimento_id: 1, grupo_id: 1, marcacao: "2P", peso: 26.1 }
        ]
      },
      {
        id: 2,
        experimento_id: 1,
        nome: "Grupo Tratado (Morfina 5mg/kg)",
        cor: "#ef4444",
        animais: [
          { id: 3, experimento_id: 1, grupo_id: 2, marcacao: "1L", peso: 25.0 },
          { id: 4, experimento_id: 1, grupo_id: 2, marcacao: "2L", peso: 23.8 }
        ]
      }
    ]
  }
];

interface MockSequencia {
  id: number;
  animal_id: number;
  timepoint_id: number;
  status: 'em_andamento' | 'concluida';
  filamento_inicial: number;
  limiar: number | null;
  estimativa_log?: number;
  k_dixon?: number;
  d_usado?: number;
  n_nominal?: number;
  criado_em: string;
  atualizado_em: string;
}

interface MockResposta {
  sequencia_id: number;
  ordem: number;
  filamento_g: number;
  resposta: 'O' | 'X';
}

let mockSequencias: MockSequencia[] = [
  // Pré-cadastrar uma concluída para testes visuais
  {
    id: 100,
    animal_id: 1, // animal 1P
    timepoint_id: 1, // timepoint basal 1
    status: 'concluida',
    filamento_inicial: 0.16,
    limiar: 0.235,
    estimativa_log: -0.628,
    k_dixon: -0.378,
    d_usado: 0.3835,
    n_nominal: 6,
    criado_em: new Date(Date.now() - 3600000).toISOString(),
    atualizado_em: new Date(Date.now() - 3600000).toISOString()
  }
];

let mockRespostasSequencia: MockResposta[] = [
  { sequencia_id: 100, ordem: 0, filamento_g: 0.16, resposta: 'O' },
  { sequencia_id: 100, ordem: 1, filamento_g: 0.4, resposta: 'X' },
  { sequencia_id: 100, ordem: 2, filamento_g: 0.16, resposta: 'X' },
  { sequencia_id: 100, ordem: 3, filamento_g: 0.07, resposta: 'O' },
  { sequencia_id: 100, ordem: 4, filamento_g: 0.16, resposta: 'X' },
  { sequencia_id: 100, ordem: 5, filamento_g: 0.07, resposta: 'O' }
];

export async function invokeCommand<T>(cmd: string, args: Record<string, any> = {}): Promise<T> {
  if (IS_TAURI) {
    try {
      return await invoke<T>(cmd, args);
    } catch (e) {
      console.error(`Erro ao invocar comando Tauri ${cmd}:`, e);
      throw e;
    }
  } else {
    console.warn(`[Mock] Executando comando ${cmd} no navegador (sem Tauri).`, args);
    
    // Simula atraso na resposta de rede/banco
    await new Promise(resolve => setTimeout(resolve, 150));

    switch (cmd) {
      // CONJUNTOS DE FILAMENTOS (ETAPA 2)
      case 'criar_conjunto': {
        const { nome, descricao, valores } = args;
        if (!nome || nome.trim() === '') {
          throw new Error("O nome do conjunto não pode ser vazio.");
        }
        const d = mockCalcularD(valores);
        const novo = {
          id: Date.now(),
          nome: nome.trim(),
          descricao: descricao || null,
          d,
          criado_em: new Date().toISOString(),
          atualizado_em: new Date().toISOString(),
          valores: [...valores].sort((a, b) => a - b)
        };
        mockConjuntos.push(novo);
        return novo as unknown as T;
      }
      case 'listar_conjuntos': {
        return mockConjuntos as unknown as T;
      }
      case 'editar_conjunto': {
        const { id, nome, descricao, valores } = args;
        if (!nome || nome.trim() === '') {
          throw new Error("O nome do conjunto não pode ser vazio.");
        }
        const index = mockConjuntos.findIndex(c => c.id === id);
        if (index === -1) {
          throw new Error("Conjunto não encontrado.");
        }
        const d = mockCalcularD(valores);
        mockConjuntos[index] = {
          ...mockConjuntos[index],
          nome: nome.trim(),
          descricao: descricao || null,
          d,
          valores: [...valores].sort((a, b) => a - b),
          atualizado_em: new Date().toISOString()
        };
        return mockConjuntos[index] as unknown as T;
      }
      case 'excluir_conjunto': {
        const { id } = args;
        mockConjuntos = mockConjuntos.filter(c => c.id !== id);
        return null as unknown as T;
      }
      case 'recalcular_d_conjunto': {
        const { id } = args;
        const conjunto = mockConjuntos.find(c => c.id === id);
        if (!conjunto) {
          throw new Error("Conjunto não encontrado.");
        }
        const d = mockCalcularD(conjunto.valores);
        conjunto.d = d;
        return d as unknown as T;
      }

      // EXPERIMENTOS, GRUPOS, ANIMAIS (ETAPA 3)
      case 'criar_experimento': {
        const { nome, descricao, conjuntoId, responsavel, timepoints } = args; const conjunto_id = conjuntoId;;
        if (!nome || nome.trim() === '') throw new Error("O nome do experimento é obrigatório.");
        const kit = mockConjuntos.find(c => c.id === conjunto_id);
        const conjunto_nome = kit ? kit.nome : "Kit Desconhecido";
        
        const novoExp = {
          id: Date.now(),
          nome: nome.trim(),
          descricao: descricao || null,
          conjunto_id,
          conjunto_nome,
          responsavel: responsavel || null,
          criado_em: new Date().toISOString(),
          atualizado_em: new Date().toISOString(),
          timepoints: timepoints.map((rotulo: string, idx: number) => ({
            id: Date.now() + idx,
            experimento_id: Date.now(),
            rotulo: rotulo.trim(),
            ordem: idx,
            opcional: 0
          })),
          grupos: []
        };
        mockExperimentos.push(novoExp);
        return novoExp as unknown as T;
      }

      // Criação atômica do wizard: experimento + timepoints + grupos + animais.
      // No mock, "atômico" = valida tudo ANTES de inserir no array (nada parcial).
      case 'criar_experimento_completo': {
        const { nome, descricao, conjuntoId, responsavel, timepoints, grupos } = args;
        const conjunto_id = conjuntoId;
        if (!nome || nome.trim() === '') throw new Error("O nome do experimento é obrigatório.");
        const tps: string[] = (timepoints || []).map((t: string) => t.trim()).filter((t: string) => t !== '');
        if (tps.length === 0) throw new Error("O experimento deve ter pelo menos 1 timepoint.");
        const kit = mockConjuntos.find(c => c.id === conjunto_id);
        if (!kit) throw new Error("Conjunto de filamentos não encontrado ou inativo.");
        for (const g of (grupos || [])) {
          if (!g.nome || g.nome.trim() === '') throw new Error("O nome do grupo não pode ser vazio.");
          for (const a of (g.animais || [])) {
            if (!a.marcacao || a.marcacao.trim() === '') {
              throw new Error(`Há um animal sem marcação no grupo '${g.nome}'.`);
            }
            if (a.peso !== null && a.peso !== undefined && !(a.peso > 0)) {
              throw new Error(`O peso do animal '${a.marcacao}' deve ser maior que zero.`);
            }
          }
        }

        const expId = Date.now();
        const novoExpCompleto = {
          id: expId,
          nome: nome.trim(),
          descricao: descricao || null,
          conjunto_id,
          conjunto_nome: kit.nome,
          responsavel: responsavel || null,
          criado_em: new Date().toISOString(),
          atualizado_em: new Date().toISOString(),
          timepoints: tps.map((rotulo: string, idx: number) => ({
            id: expId + 1000 + idx,
            experimento_id: expId,
            rotulo,
            ordem: idx,
            opcional: 0
          })),
          grupos: (grupos || []).map((g: any, gi: number) => {
            const grupoId = expId + 2000 + gi;
            return {
              id: grupoId,
              experimento_id: expId,
              nome: g.nome.trim(),
              cor: g.cor,
              animais: (g.animais || []).map((a: any, ai: number) => ({
                id: expId + 3000 + gi * 100 + ai,
                experimento_id: expId,
                grupo_id: grupoId,
                marcacao: a.marcacao.trim(),
                peso: a.peso ?? null
              }))
            };
          })
        };
        mockExperimentos.push(novoExpCompleto);
        return novoExpCompleto as unknown as T;
      }
      case 'listar_experimentos': {
        return mockExperimentos as unknown as T;
      }
      case 'obter_experimento': {
        const { id } = args;
        const exp = mockExperimentos.find(e => e.id === id);
        if (!exp) throw new Error("Experimento não encontrado.");
        return exp as unknown as T;
      }
      case 'editar_experimento': {
        const { id, nome, descricao, conjuntoId, responsavel, timepoints } = args; const conjunto_id = conjuntoId;;
        if (!nome || nome.trim() === '') throw new Error("O nome do experimento é obrigatório.");
        const index = mockExperimentos.findIndex(e => e.id === id);
        if (index === -1) throw new Error("Experimento não encontrado.");
        
        const kit = mockConjuntos.find(c => c.id === conjunto_id);
        const conjunto_nome = kit ? kit.nome : "Kit Desconhecido";

        mockExperimentos[index] = {
          ...mockExperimentos[index],
          nome: nome.trim(),
          descricao: descricao || null,
          conjunto_id,
          conjunto_nome,
          responsavel: responsavel || null,
          timepoints: timepoints.map((rotulo: string, idx: number) => ({
            id: Date.now() + idx,
            experimento_id: id,
            rotulo: rotulo.trim(),
            ordem: idx,
            opcional: 0
          })),
          atualizado_em: new Date().toISOString()
        };
        return mockExperimentos[index] as unknown as T;
      }
      case 'excluir_experimento': {
        const { id } = args;
        mockExperimentos = mockExperimentos.filter(e => e.id !== id);
        return null as unknown as T;
      }
      case 'criar_grupo': {
        const { experimentoId, nome, cor } = args; const experimento_id = experimentoId;;
        if (!nome || nome.trim() === '') throw new Error("O nome do grupo é obrigatório.");
        const exp = mockExperimentos.find(e => e.id === experimento_id);
        if (!exp) throw new Error("Experimento não encontrado.");
        
        const novoGrupo = {
          id: Date.now(),
          experimento_id,
          nome: nome.trim(),
          cor: cor.trim(),
          animais: []
        };
        exp.grupos.push(novoGrupo);
        return {
          id: novoGrupo.id,
          experimento_id,
          nome: novoGrupo.nome,
          cor: novoGrupo.cor
        } as unknown as T;
      }
      case 'editar_grupo': {
        const { id, nome, cor } = args;
        if (!nome || nome.trim() === '') throw new Error("O nome do grupo é obrigatório.");
        
        let grupoEncontrado: any = null;
        for (const exp of mockExperimentos) {
          const g = exp.grupos.find(g => g.id === id);
          if (g) {
            g.nome = nome.trim();
            g.cor = cor.trim();
            grupoEncontrado = g;
            break;
          }
        }
        if (!grupoEncontrado) throw new Error("Grupo não encontrado.");
        return {
          id: grupoEncontrado.id,
          experimento_id: grupoEncontrado.experimento_id,
          nome: grupoEncontrado.nome,
          cor: grupoEncontrado.cor
        } as unknown as T;
      }
      case 'excluir_grupo': {
        const { id } = args;
        for (const exp of mockExperimentos) {
          const lenBefore = exp.grupos.length;
          exp.grupos = exp.grupos.filter(g => g.id !== id);
          if (exp.grupos.length < lenBefore) {
            break;
          }
        }
        return null as unknown as T;
      }
      case 'criar_animal': {
        const { experimentoId, grupoId, marcacao, peso } = args; const experimento_id = experimentoId; const grupo_id = grupoId;;
        if (!marcacao || marcacao.trim() === '') throw new Error("A marcação é obrigatória.");
        const exp = mockExperimentos.find(e => e.id === experimento_id);
        if (!exp) throw new Error("Experimento não encontrado.");
        const grupo = exp.grupos.find(g => g.id === grupo_id);
        if (!grupo) throw new Error("Grupo não encontrado.");
        
        const novoAnimal = {
          id: Date.now(),
          experimento_id,
          grupo_id,
          marcacao: marcacao.trim(),
          peso: peso || null
        };
        grupo.animais.push(novoAnimal);
        return novoAnimal as unknown as T;
      }
      case 'editar_animal': {
        const { id, grupoId, marcacao, peso } = args; const grupo_id = grupoId;;
        if (!marcacao || marcacao.trim() === '') throw new Error("A marcação é obrigatória.");
        
        let animalEncontrado: any = null;
        let oldGrupo: any = null;
        let targetGrupo: any = null;
        let parentExp: any = null;

        for (const exp of mockExperimentos) {
          for (const g of exp.grupos) {
            const idx = g.animais.findIndex(a => a.id === id);
            if (idx !== -1) {
              animalEncontrado = g.animais[idx];
              oldGrupo = g;
              parentExp = exp;
              break;
            }
          }
        }

        if (!animalEncontrado) throw new Error("Animal não encontrado.");
        
        // Se mudou de grupo, precisamos mover o animal
        if (grupo_id !== oldGrupo.id) {
          targetGrupo = parentExp.grupos.find((g: any) => g.id === grupo_id);
          if (!targetGrupo) throw new Error("Novo grupo não encontrado.");
          
          oldGrupo.animais = oldGrupo.animais.filter((a: any) => a.id !== id);
          animalEncontrado.grupo_id = grupo_id;
          targetGrupo.animais.push(animalEncontrado);
        }

        animalEncontrado.marcacao = marcacao.trim();
        animalEncontrado.peso = peso || null;

        return animalEncontrado as unknown as T;
      }
      case 'excluir_animal': {
        const { id } = args;
        for (const exp of mockExperimentos) {
          for (const g of exp.grupos) {
            const lenBefore = g.animais.length;
            g.animais = g.animais.filter(a => a.id !== id);
            if (g.animais.length < lenBefore) {
              break;
            }
          }
        }
        return null as unknown as T;
      }

      // FLUXO DE TESTE SEQUENCIAL (ETAPA 4 MOCKS)
      case 'iniciar_sequencia': {
        const { animalId, timepointId, filamentoInicial } = args; const animal_id = animalId; const timepoint_id = timepointId; const filamento_inicial = filamentoInicial;;
        const ativa = mockSequencias.find(s => s.animal_id === animal_id && s.timepoint_id === timepoint_id && s.status === 'em_andamento');
        if (ativa) {
          throw new Error("Este animal já possui uma sequência de testes em andamento para este timepoint.");
        }
        const nova = {
          id: Date.now(),
          animal_id,
          timepoint_id,
          status: 'em_andamento' as const,
          filamento_inicial,
          limiar: null,
          criado_em: new Date().toISOString(),
          atualizado_em: new Date().toISOString()
        };
        mockSequencias.push(nova);
        return {
          ...nova,
          respostas: []
        } as unknown as T;
      }
      
      case 'registrar_resposta': {
        const { sequenciaId, resposta } = args; const sequencia_id = sequenciaId;;
        const seq = mockSequencias.find(s => s.id === sequencia_id);
        if (!seq) throw new Error("Sequência não encontrada.");
        if (seq.status !== 'em_andamento') throw new Error("Esta sequência já foi finalizada.");

        // 1. Achar filamentos do kit associado
        let animal_id = seq.animal_id;
        let kitValores = mockConjuntos[0].valores; // default fallback
        for (const exp of mockExperimentos) {
          for (const g of exp.grupos) {
            if (g.animais.some(a => a.id === animal_id)) {
              const kit = mockConjuntos.find(c => c.id === exp.conjunto_id);
              if (kit) {
                kitValores = kit.valores;
              }
              break;
            }
          }
        }

        // 2. Respostas atuais
        const respostas = mockRespostasSequencia
          .filter(r => r.sequencia_id === sequencia_id)
          .sort((a, b) => a.ordem - b.ordem);

        const ordem_nova = respostas.length;
        const filamento_testado = ordem_nova === 0
          ? seq.filamento_inicial
          : mockSugerirProximo(kitValores, respostas[ordem_nova - 1].filamento_g, respostas[ordem_nova - 1].resposta).proximo;

        const novaResp: MockResposta = {
          sequencia_id,
          ordem: ordem_nova,
          filamento_g: filamento_testado,
          resposta: resposta as 'O' | 'X'
        };
        mockRespostasSequencia.push(novaResp);

        respostas.push(novaResp);

        // 3. Próxima sugestão e N nominal
        const proximo = mockSugerirProximo(kitValores, filamento_testado, resposta);
        const respostasStr = respostas.map(r => r.resposta);
        const n_nominal = mockCalcularNNominal(respostasStr);
        const pode_finalizar = n_nominal >= 2 && n_nominal <= 6; // Tabela 7 cobre N de 2 a 6

        return {
          sequencia_id,
          proximo_filamento: proximo.proximo,
          aviso: proximo.aviso,
          n_nominal,
          pode_finalizar,
          respostas: respostas.map(r => ({
            ordem: r.ordem,
            filamento_g: r.filamento_g,
            resposta: r.resposta
          }))
        } as unknown as T;
      }

      case 'desfazer_ultima_resposta': {
        const { sequenciaId } = args; const sequencia_id = sequenciaId;;
        const seq = mockSequencias.find(s => s.id === sequencia_id);
        if (!seq) throw new Error("Sequência não encontrada.");
        if (seq.status !== 'em_andamento') throw new Error("Apenas sequências em andamento podem ser modificadas.");

        // Obter todas as respostas e deletar a com ordem mais alta
        const respostas = mockRespostasSequencia
          .filter(r => r.sequencia_id === sequencia_id)
          .sort((a, b) => a.ordem - b.ordem);

        if (respostas.length > 0) {
          const maxOrdem = respostas[respostas.length - 1].ordem;
          mockRespostasSequencia = mockRespostasSequencia.filter(
            r => !(r.sequencia_id === sequencia_id && r.ordem === maxOrdem)
          );
          respostas.pop();
        }

        // Achar filamentos do kit associado
        let kitValores = mockConjuntos[0].valores;
        for (const exp of mockExperimentos) {
          for (const g of exp.grupos) {
            if (g.animais.some(a => a.id === seq.animal_id)) {
              const kit = mockConjuntos.find(c => c.id === exp.conjunto_id);
              if (kit) kitValores = kit.valores;
              break;
            }
          }
        }

        // Nova sugestão
        let proximo_filamento = seq.filamento_inicial;
        let aviso: string | null = null;
        if (respostas.length > 0) {
          const ult = respostas[respostas.length - 1];
          const res = mockSugerirProximo(kitValores, ult.filamento_g, ult.resposta);
          proximo_filamento = res.proximo;
          aviso = res.aviso;
        }

        const respostasStr = respostas.map(r => r.resposta);
        const n_nominal = mockCalcularNNominal(respostasStr);
        const pode_finalizar = n_nominal >= 2 && n_nominal <= 6; // Tabela 7 cobre N de 2 a 6

        return {
          sequencia_id,
          proximo_filamento,
          aviso,
          n_nominal,
          pode_finalizar,
          respostas: respostas.map(r => ({
            ordem: r.ordem,
            filamento_g: r.filamento_g,
            resposta: r.resposta
          }))
        } as unknown as T;
      }

      case 'finalizar_sequencia': {
        const { sequenciaId } = args; const sequencia_id = sequenciaId;;
        const seq = mockSequencias.find(s => s.id === sequencia_id);
        if (!seq) throw new Error("Sequência não encontrada.");
        if (seq.status !== 'em_andamento') throw new Error("Esta sequência já foi finalizada.");

        const respostas = mockRespostasSequencia
          .filter(r => r.sequencia_id === sequencia_id)
          .sort((a, b) => a.ordem - b.ordem);

        if (respostas.length === 0) {
          throw new Error("A sequência de testes está vazia.");
        }

        const respostasStr = respostas.map(r => r.resposta);
        const temO = respostasStr.includes("O");
        const temX = respostasStr.includes("X");
        if (!temO || !temX) {
          throw new Error("A série de testes não possui nenhuma reversão (alteração de resposta). Adicione respostas alternadas antes de finalizar.");
        }

        // Achar kit d
        let d = 0.2664;
        for (const exp of mockExperimentos) {
          for (const g of exp.grupos) {
            if (g.animais.some(a => a.id === seq.animal_id)) {
              const kit = mockConjuntos.find(c => c.id === exp.conjunto_id);
              if (kit) d = kit.d;
              break;
            }
          }
        }

        // Mock Dixon Calculation
        const xf = respostas[respostas.length - 1].filamento_g;
        let k = 0.5;
        const respJoined = respostasStr.join("");
        if (respJoined === "OXXOXO") {
          k = 0.831;
        } else if (respJoined === "XOOXOX") {
          k = -0.831;
        } else {
          k = respostasStr[respostasStr.length - 1] === "O" ? 0.45 : -0.45;
        }

        const limiar = Math.pow(10, Math.log10(xf) + k * d);
        const n_nominal = mockCalcularNNominal(respostasStr);

        seq.status = 'concluida';
        seq.limiar = limiar;
        seq.k_dixon = k;
        seq.d_usado = d;
        seq.n_nominal = n_nominal;

        return {
          sequencia_id,
          limiar,
          k,
          xf,
          d,
          n_nominal
        } as unknown as T;
      }

      case 'obter_sequencia_ativa': {
        const { animalId, timepointId } = args; const animal_id = animalId; const timepoint_id = timepointId;;
        const seq = mockSequencias.find(s => s.animal_id === animal_id && s.timepoint_id === timepoint_id && s.status === 'em_andamento');
        if (!seq) return null as unknown as T;

        const respostas = mockRespostasSequencia
          .filter(r => r.sequencia_id === seq.id)
          .sort((a, b) => a.ordem - b.ordem)
          .map(r => ({
            ordem: r.ordem,
            filamento_g: r.filamento_g,
            resposta: r.resposta
          }));

        return {
          id: seq.id,
          animal_id,
          timepoint_id,
          status: seq.status,
          filamento_inicial: seq.filamento_inicial,
          limiar: seq.limiar,
          respostas
        } as unknown as T;
      }

      case 'listar_sequencias_concluidas': {
        const { experimentoId } = args; const experimento_id = experimentoId;;
        const result: any[] = [];
        
        // Achar todos os animais do experimento
        const exp = mockExperimentos.find(e => e.id === experimento_id);
        if (!exp) return [] as unknown as T;

        const animalMap = new Map<number, { marcacao: string, grupo: string, cor: string }>();
        for (const g of exp.grupos) {
          for (const a of g.animais) {
            animalMap.set(a.id, { marcacao: a.marcacao, grupo: g.nome, cor: g.cor });
          }
        }

        const timepointMap = new Map<number, string>();
        for (const t of exp.timepoints) {
          timepointMap.set(t.id, t.rotulo);
        }

        const concluidas = mockSequencias;
        for (const s of concluidas) {
          const anim = animalMap.get(s.animal_id);
          const tpRotulo = timepoint_id_check(s.timepoint_id) ? timepointMap.get(s.timepoint_id) : undefined;
          
          function timepoint_id_check(tp_id: number) {
            return timepointMap.has(tp_id);
          }

          if (anim && tpRotulo) {
            const respostas = mockRespostasSequencia
              .filter(r => r.sequencia_id === s.id)
              .sort((a, b) => a.ordem - b.ordem)
              .map(r => r.resposta)
              .join("");

            result.push({
              id: s.id,
              animal_id: s.animal_id,
              animal_marcacao: anim.marcacao,
              grupo_nome: anim.grupo,
              grupo_cor: anim.cor,
              timepoint_id: s.timepoint_id,
              timepoint_rotulo: tpRotulo,
              filamento_inicial: s.filamento_inicial,
              limiar: s.limiar,
              status: s.status,
              criado_em: s.criado_em,
              respostas
            });
          }
        }

        return result as unknown as T;
      }

      case 'calcular_estatisticas_experimento': {
        const { experimentoId } = args; const experimento_id = experimentoId;;
        const exp = mockExperimentos.find(e => e.id === experimentoId);
        if (!exp) return [] as unknown as T;
        
        const tps = exp.timepoints;
        const tpMap = new Map<number, { id: number, rotulo: string, ordem: number }>(tps.map((t: any) => [t.id, t]));
        
        const grps = exp.grupos;
        const grpMap = new Map<number, { id: number, nome: string, cor: string }>(grps.map((g: any) => [g.id, g]));
        
        const anis: any[] = [];
        for (const g of grps) {
          for (const a of g.animais) {
            anis.push({ id: a.id, marcacao: a.marcacao, grupo_id: g.id });
          }
        }
        const aniMap = new Map<number, any>(anis.map(a => [a.id, a]));
        
        const seqs = mockSequencias.filter(s => {
          const ani = aniMap.get(s.animal_id);
          return ani && s.status === 'concluida' && s.limiar !== null;
        });
        
        const groups: { [key: string]: number[] } = {};
        for (const s of seqs) {
          const ani = aniMap.get(s.animal_id)!;
          const key = `${ani.grupo_id}_${s.timepoint_id}`;
          if (!groups[key]) groups[key] = [];
          groups[key].push(s.limiar!);
        }
        
        const result: any[] = [];
        
        for (const key in groups) {
          const [grupo_id_str, timepoint_id_str] = key.split('_');
          const grupo_id = parseInt(grupo_id_str, 10);
          const timepoint_id = parseInt(timepoint_id_str, 10);
          
          const grp = grpMap.get(grupo_id)!;
          const tp = tpMap.get(timepoint_id)!;
          const limiares = groups[key];
          const n = limiares.length;
          
          const logs = limiares.map(x => Math.log10(x));
          const somaLogs = logs.reduce((a, b) => a + b, 0);
          const mediaLog = somaLogs / n;
          const media_geometrica_g = Math.pow(10, mediaLog);
          
          let limite_inferior_g: number | null = null;
          let limite_superior_g: number | null = null;
          
          if (n > 1) {
            const somaVar = logs.reduce((a, b) => a + Math.pow(b - mediaLog, 2), 0);
            const desvioPadraoLog = Math.sqrt(somaVar / (n - 1));
            const erroPadraoLog = desvioPadraoLog / Math.sqrt(n);
            
            limite_inferior_g = Math.pow(10, mediaLog - erroPadraoLog);
            limite_superior_g = Math.pow(10, mediaLog + erroPadraoLog);
          }
          
          result.push({
            grupo_id,
            grupo_nome: grp.nome,
            grupo_cor: grp.cor,
            timepoint_id,
            timepoint_rotulo: tp.rotulo,
            timepoint_ordem: tp.ordem,
            n,
            media_geometrica_g,
            limite_inferior_g,
            limite_superior_g
          });
        }
        
        result.sort((a, b) => {
          if (a.grupo_id !== b.grupo_id) {
            return a.grupo_id - b.grupo_id;
          }
          return a.timepoint_ordem - b.timepoint_ordem;
        });
        
        return result as unknown as T;
      }

      case 'obter_respostas_cruas_experimento': {
        const { experimentoId } = args; const experimento_id = experimentoId;;
        const exp = mockExperimentos.find(e => e.id === experimentoId);
        if (!exp) return [] as unknown as T;
        
        const tps = exp.timepoints;
        const tpMap = new Map<number, { id: number, rotulo: string, ordem: number }>(tps.map((t: any) => [t.id, t]));
        
        const grps = exp.grupos;
        const grpMap = new Map<number, { id: number, nome: string, cor: string }>(grps.map((g: any) => [g.id, g]));
        
        const anis: any[] = [];
        for (const g of grps) {
          for (const a of g.animais) {
            anis.push({ id: a.id, marcacao: a.marcacao, grupo_id: g.id });
          }
        }
        const aniMap = new Map<number, any>(anis.map(a => [a.id, a]));
        
        const seqs = mockSequencias.filter(s => aniMap.has(s.animal_id));
        const seqMap = new Map<number, any>(seqs.map(s => [s.id, s]));
        
        const resps = mockRespostasSequencia
          .filter(r => seqMap.has(r.sequencia_id))
          .map(r => {
            const seq = seqMap.get(r.sequencia_id)!;
            const ani = aniMap.get(seq.animal_id)!;
            const grp = grpMap.get(ani.grupo_id)!;
            const tp = tpMap.get(seq.timepoint_id)!;
            return {
              grupo: grp.nome,
              animal: ani.marcacao,
              timepoint: tp.rotulo,
              ordem: r.ordem,
              filamento: r.filamento_g,
              resposta: r.resposta
            };
          });
          
        resps.sort((a, b) => {
          if (a.grupo !== b.grupo) return a.grupo.localeCompare(b.grupo);
          if (a.animal !== b.animal) return a.animal.localeCompare(b.animal);
          if (a.timepoint !== b.timepoint) return a.timepoint.localeCompare(b.timepoint);
          return a.ordem - b.ordem;
        });
        
        return resps as unknown as T;
      }

      case 'obter_limiares_experimento': {
        const { experimentoId } = args; const experimento_id = experimentoId;;
        const exp = mockExperimentos.find(e => e.id === experimentoId);
        if (!exp) return [] as unknown as T;
        
        const tps = exp.timepoints;
        const tpMap = new Map<number, { id: number, rotulo: string, ordem: number }>(tps.map((t: any) => [t.id, t]));
        
        const grps = exp.grupos;
        const grpMap = new Map<number, { id: number, nome: string, cor: string }>(grps.map((g: any) => [g.id, g]));
        
        const anis: any[] = [];
        for (const g of grps) {
          for (const a of g.animais) {
            anis.push({ id: a.id, marcacao: a.marcacao, grupo_id: g.id });
          }
        }
        const aniMap = new Map<number, any>(anis.map(a => [a.id, a]));
        
        const seqs = mockSequencias.filter(s => {
          const ani = aniMap.get(s.animal_id);
          return ani && s.status === 'concluida' && s.limiar !== null;
        });
        
        const result = seqs.map(s => {
          const ani = aniMap.get(s.animal_id)!;
          const grp = grpMap.get(ani.grupo_id)!;
          const tp = tpMap.get(s.timepoint_id)!;
          
          const respostasStr = mockRespostasSequencia
            .filter(r => r.sequencia_id === s.id)
            .sort((a, b) => a.ordem - b.ordem)
            .map(r => r.resposta)
            .join("");
            
          return {
            grupo: grp.nome,
            animal: ani.marcacao,
            timepoint: tp.rotulo,
            filamento_inicial: s.filamento_inicial,
            serie_respostas: respostasStr,
            limiar: s.limiar
          };
        });
        
        result.sort((a, b) => {
          if (a.grupo !== b.grupo) return a.grupo.localeCompare(b.grupo);
          if (a.animal !== b.animal) return a.animal.localeCompare(b.animal);
          return a.timepoint.localeCompare(b.timepoint);
        });
        
        return result as unknown as T;
      }

      default:
        throw new Error(`Comando mock "${cmd}" não implementado no frontend.`);
    }
  }
}

function mockCalcularD(valores: number[]): number {
  if (valores.length < 2) {
    throw new Error("O conjunto de filamentos deve conter pelo menos 2 valores para calcular d.");
  }
  const ordenados = [...valores].sort((a, b) => a - b);
  for (const v of ordenados) {
    if (isNaN(v) || !isFinite(v)) {
      throw new Error("Os valores dos filamentos devem ser números válidos.");
    }
    if (v <= 0) {
      throw new Error("Todos os valores dos filamentos devem ser maiores que zero.");
    }
  }
  for (let i = 0; i < ordenados.length - 1; i++) {
    if (Math.abs(ordenados[i+1] - ordenados[i]) < 1e-9) {
      throw new Error("O conjunto não pode conter valores duplicados.");
    }
  }
  let soma = 0;
  for (let i = 0; i < ordenados.length - 1; i++) {
    soma += Math.log10(ordenados[i+1]) - Math.log10(ordenados[i]);
  }
  return soma / (ordenados.length - 1);
}

function mockSugerirProximo(valores: number[], ultimo: number, resp: 'O' | 'X'): { proximo: number, aviso: string | null } {
  let idx = 0;
  let min_diff = Infinity;
  for (let i = 0; i < valores.length; i++) {
    const diff = Math.abs(valores[i] - ultimo);
    if (diff < min_diff) {
      min_diff = diff;
      idx = i;
    }
  }

  if (resp === 'O') {
    if (idx + 1 < valores.length) {
      return { proximo: valores[idx + 1], aviso: null };
    } else {
      return { proximo: valores[idx], aviso: "Atenção: Limite superior atingido. Sugerindo o filamento mais forte novamente." };
    }
  } else {
    if (idx > 0) {
      return { proximo: valores[idx - 1], aviso: null };
    } else {
      return { proximo: valores[idx], aviso: "Atenção: Limite inferior atingido. Sugerindo o filamento mais fraco novamente." };
    }
  }
}

function mockCalcularNNominal(respostas: string[]): number {
  if (respostas.length === 0) return 0;
  const lider = respostas[0];
  let m = 0;
  while (m < respostas.length && respostas[m] === lider) {
    m++;
  }
  if (m === respostas.length) return 0;
  return respostas.length - m + 1;
}
