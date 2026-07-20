<script lang="ts">
  import { onMount } from 'svelte';
  import { invokeCommand } from './lib/tauri';
  import GraficoCurva from './lib/GraficoCurva.svelte';
  import { montarTabelaPrism, tabelaPrismParaTsv, type TabelaPrism } from './lib/prism';
  import { save } from '@tauri-apps/plugin-dialog';
  import { writeTextFile, writeFile } from '@tauri-apps/plugin-fs';
  import * as XLSX from 'xlsx';
  import { jsPDF } from 'jspdf';

  interface ConjuntoFilamentos {
    id: number;
    nome: string;
    descricao: string | null;
    d: number;
    valores: number[];
  }

  interface Timepoint {
    id: number;
    experimento_id: number;
    rotulo: string;
    ordem: number;
    opcional: number;
  }

  interface Animal {
    id: number;
    experimento_id: number;
    grupo_id: number;
    marcacao: string;
    peso: number | null;
  }

  interface GrupoCompleto {
    id: number;
    experimento_id: number;
    nome: string;
    cor: string;
    animais: Animal[];
  }

  interface ExperimentoCompleto {
    id: number;
    nome: string;
    descricao: string | null;
    conjunto_id: number;
    conjunto_nome: string;
    responsavel: string | null;
    criado_em: string;
    atualizado_em: string;
    timepoints: Timepoint[];
    grupos: GrupoCompleto[];
  }

  // Estado Geral
  let activeTab = $state<'filamentos' | 'experimentos'>('experimentos');
  let carregando = $state(false);
  let erroMsg = $state<string | null>(null);
  let salvoD = $state<number | null>(null);
  let salvoNome = $state<string | null>(null);

  // =============================================================================
  // ESTADOS: CONJUNTOS DE FILAMENTOS (ETAPA 2)
  // =============================================================================
  let conjuntos = $state<ConjuntoFilamentos[]>([]);
  let showFilamentoForm = $state(false);
  let filamentoEditandoId = $state<number | null>(null);
  let filamentoNome = $state("");
  let filamentoDescricao = $state("");
  let filamentoValuesInput = $state<string[]>([]);

  // =============================================================================
  // ESTADOS: EXPERIMENTOS, GRUPOS E ANIMAIS (ETAPA 3)
  // =============================================================================
  let experimentos = $state<ExperimentoCompleto[]>([]);
  let selectedExpId = $state<number | null>(null);
  let selectedExp = $derived(experimentos.find(e => e.id === selectedExpId) || null);

  // Carregar sequências reativamente ao selecionar experimento
  $effect(() => {
    if (selectedExpId !== null) {
      invokeCommand<any[]>('listar_sequencias_concluidas', { experimentoId: selectedExpId })
        .then(res => {
          listSequencias = res;
        })
        .catch(e => {
          erroMsg = "Erro ao carregar sequências do experimento: " + (e.message || e);
        });
    } else {
      listSequencias = [];
    }
  });

  // Controle de formulários
  let showExpForm = $state(false);
  let expEditandoId = $state<number | null>(null);
  let expNome = $state("");
  let expDescricao = $state("");
  let expConjuntoId = $state<number | null>(null);
  let expResponsavel = $state("");
  let expTimepoints = $state<string[]>([]);

  // ---- Wizard de criação de experimento (experimento + grupos + animais de uma vez) ----
  interface WizardAnimal { marcacao: string; peso: string; }
  interface WizardGrupo {
    nome: string;
    cor: string;
    animais: WizardAnimal[];
    novaMarcacao: string;
    novoPeso: string;
  }
  const CORES_GRUPO = ["#3b82f6", "#ef4444", "#16a34a", "#f59e0b", "#8b5cf6", "#ec4899"];
  let wizardEtapa = $state<1 | 2>(1);
  let wizardGrupos = $state<WizardGrupo[]>([]);
  // refs para o fluxo de foco com Enter (marcação → peso → adiciona → marcação)
  let refMarcacao = $state<Record<number, HTMLInputElement | undefined>>({});
  let refPeso = $state<Record<number, HTMLInputElement | undefined>>({});

  let showGroupForm = $state(false);
  let grupoEditandoId = $state<number | null>(null);
  let grupoNome = $state("");
  let grupoCor = $state("#3b82f6");

  let showAnimalForm = $state(false);
  let animalEditandoId = $state<number | null>(null);
  let animalMarcacao = $state("");
  let animalPeso = $state("");
  let animalGrupoId = $state<number | null>(null);

  // =============================================================================
  // ESTADOS DO TESTE SEQUENCIAL (ETAPA 4)
  // =============================================================================
  let expTab = $state<'matriz' | 'estrutura' | 'grafico'>('matriz');
  let testandoAnimal = $state<Animal | null>(null);
  let testandoTimepoint = $state<Timepoint | null>(null);
  let testandoSequencia = $state<any | null>(null);
  let testandoFilamentoInicial = $state("");
  let testandoUltimaSugestao = $state<any | null>(null);
  let showStartTestForm = $state(false);
  let showActiveTestScreen = $state(false);
  let listSequencias = $state<any[]>([]);
  const concluidas = $derived(listSequencias.filter(s => s.status === 'concluida'));
  let salvoLimiar = $state<number | null>(null);
  let salvoAnimal = $state("");
  let salvoTimepoint = $state("");

  // =============================================================================
  // CÁLCULOS REATIVOS (RUNES)
  // =============================================================================
  
  // Para Filamentos
  const parsedFilamentoValores = $derived(
    filamentoValuesInput
      .map(v => parseFloat(v.replace(',', '.')))
      .filter(v => !isNaN(v))
  );

  const filamentoCalcResult = $derived.by(() => {
    if (filamentoValuesInput.some(v => v.trim() === '')) {
      return { ok: false, error: "Aguardando o preenchimento de todos os filamentos..." };
    }
    if (parsedFilamentoValores.length < 2) {
      return { ok: false, error: "Insira pelo menos 2 filamentos para poder calcular d." };
    }
    for (const v of parsedFilamentoValores) {
      if (v <= 0) return { ok: false, error: "Todos os filamentos devem ter força maior que zero." };
    }
    const sorted = [...parsedFilamentoValores].sort((a, b) => a - b);
    for (let i = 0; i < sorted.length - 1; i++) {
      if (Math.abs(sorted[i+1] - sorted[i]) < 1e-9) {
        return { ok: false, error: "O conjunto possui valores duplicados." };
      }
    }
    let soma = 0;
    for (let i = 0; i < sorted.length - 1; i++) {
      soma += Math.log10(sorted[i+1]) - Math.log10(sorted[i]);
    }
    return { ok: true, d: soma / (sorted.length - 1) };
  });

  // =============================================================================
  // FUNÇÕES DE DADOS (CARREGAMENTO)
  // =============================================================================
  async function carregarDados() {
    carregando = true;
    try {
      conjuntos = await invokeCommand<ConjuntoFilamentos[]>('listar_conjuntos');
      experimentos = await invokeCommand<ExperimentoCompleto[]>('listar_experimentos');
      if (selectedExpId !== null) {
        listSequencias = await invokeCommand<any[]>('listar_sequencias_concluidas', { experimentoId: selectedExpId });
      }
      erroMsg = null;
    } catch (e: any) {
      erroMsg = "Erro ao carregar dados: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  // =============================================================================
  // FUNÇÕES DE TESTES SEQUENCIAIS (ETAPA 4)
  // =============================================================================
  function obterFilamentosDoExperimento(): number[] {
    if (!selectedExp) return [];
    const kit = conjuntos.find(c => c.id === selectedExp.conjunto_id);
    return kit ? kit.valores : [];
  }

  function mockSugerirProximoLocal(valores: number[], ultimo: number, resp: 'O' | 'X'): { proximo: number, aviso: string | null } {
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

  function mockCalcularNNominalLocal(respostas: string[]): number {
    if (respostas.length === 0) return 0;
    const lider = respostas[0];
    let m = 0;
    while (m < respostas.length && respostas[m] === lider) {
      m++;
    }
    if (m === respostas.length) return 0;
    return respostas.length - m + 1;
  }

  async function abrirSequenciaEmAndamento(animal: Animal, timepoint: Timepoint, seqId: number) {
    erroMsg = null;
    try {
      carregando = true;
      testandoAnimal = animal;
      testandoTimepoint = timepoint;
      
      const seq = await invokeCommand<any>('obter_sequencia_ativa', { 
        animalId: animal.id, 
        timepointId: timepoint.id 
      });
      
      if (!seq) {
        throw new Error("Sequência ativa não encontrada.");
      }
      
      testandoSequencia = seq;
      
      if (seq.respostas.length === 0) {
        testandoUltimaSugestao = {
          sequenciaId: seq.id,
          proximo_filamento: seq.filamento_inicial,
          aviso: null,
          n_nominal: 0,
          pode_finalizar: false,
          respostas: []
        };
      } else {
        const kitValores = obterFilamentosDoExperimento();
        const respostas = seq.respostas;
        const ult = respostas[respostas.length - 1];
        const resSug = mockSugerirProximoLocal(kitValores, ult.filamento_g, ult.resposta);
        const respostasStr = respostas.map((r: any) => r.resposta);
        const n_nominal = mockCalcularNNominalLocal(respostasStr);
        
        testandoUltimaSugestao = {
          sequenciaId: seq.id,
          proximo_filamento: resSug.proximo,
          aviso: resSug.aviso,
          n_nominal,
          pode_finalizar: n_nominal >= 2,
          respostas
        };
      }
      
      showActiveTestScreen = true;
      showStartTestForm = false;
      salvoLimiar = null;
    } catch (e: any) {
      erroMsg = "Erro ao retomar teste: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  function iniciarFormularioTeste(animal: Animal, timepoint: Timepoint) {
    testandoAnimal = animal;
    testandoTimepoint = timepoint;
    erroMsg = null;
    
    const filamentos = obterFilamentosDoExperimento();
    if (filamentos.length > 0) {
      const midIdx = Math.floor(filamentos.length / 2);
      testandoFilamentoInicial = filamentos[midIdx].toString();
    } else {
      testandoFilamentoInicial = "";
    }
    
    showStartTestForm = true;
    showActiveTestScreen = false;
    salvoLimiar = null;
  }

  async function iniciarSequencia() {
    if (!testandoAnimal || !testandoTimepoint || !testandoFilamentoInicial) return;
    erroMsg = null;
    try {
      carregando = true;
      const filInicialNum = parseFloat(testandoFilamentoInicial);
      if (isNaN(filInicialNum)) {
        throw new Error("Filamento inicial inválido.");
      }
      
      const seq = await invokeCommand<any>('iniciar_sequencia', {
        animalId: testandoAnimal.id,
        timepointId: testandoTimepoint.id,
        filamentoInicial: filInicialNum
      });
      
      testandoSequencia = seq;
      testandoUltimaSugestao = {
        sequenciaId: seq.id,
        proximo_filamento: seq.filamento_inicial,
        aviso: null,
        n_nominal: 0,
        pode_finalizar: false,
        respostas: []
      };
      
      showActiveTestScreen = true;
      showStartTestForm = false;
      
      // Atualiza a lista local de sequências
      if (selectedExpId !== null) {
        listSequencias = await invokeCommand<any[]>('listar_sequencias_concluidas', { experimentoId: selectedExpId });
      }
    } catch (e: any) {
      erroMsg = "Erro ao iniciar sequência: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  async function registrarResposta(resp: 'O' | 'X') {
    if (!testandoSequencia) return;
    erroMsg = null;
    try {
      carregando = true;
      const sugestao = await invokeCommand<any>('registrar_resposta', {
        sequenciaId: testandoSequencia.id,
        resposta: resp
      });
      testandoUltimaSugestao = sugestao;
      
      // Atualiza em tempo real
      if (selectedExpId !== null) {
        listSequencias = await invokeCommand<any[]>('listar_sequencias_concluidas', { experimentoId: selectedExpId });
      }
    } catch (e: any) {
      erroMsg = "Erro ao registrar resposta: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  async function desfazerUltima() {
    if (!testandoSequencia) return;
    erroMsg = null;
    try {
      carregando = true;
      const sugestao = await invokeCommand<any>('desfazer_ultima_resposta', {
        sequenciaId: testandoSequencia.id
      });
      testandoUltimaSugestao = sugestao;
      
      // Atualiza em tempo real
      if (selectedExpId !== null) {
        listSequencias = await invokeCommand<any[]>('listar_sequencias_concluidas', { experimentoId: selectedExpId });
      }
    } catch (e: any) {
      erroMsg = "Erro ao desfazer resposta: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  // ---------------------------------------------------------------------------
  // ATALHOS DE TECLADO — só na tela de execução do teste
  // 0 = Não respondeu (O) · 1 = Respondeu (X) · Backspace = Desfazer
  // Propositalmente NÃO há atalho para finalizar a sequência (evita finalização
  // acidental — essa ação exige clique). Ver docs/DOMINIO.md §9.
  // ---------------------------------------------------------------------------
  function focoEmCampoDeTexto(alvo: EventTarget | null): boolean {
    const el = alvo as HTMLElement | null;
    if (!el) return false;
    const tag = el.tagName?.toLowerCase();
    return tag === 'input' || tag === 'textarea' || tag === 'select' || el.isContentEditable === true;
  }

  function handleAtalhosTeste(e: KeyboardEvent) {
    // Só quando a tela de teste está visível.
    if (!showActiveTestScreen) return;
    // Não sequestrar o teclado se o usuário estiver digitando num campo.
    if (focoEmCampoDeTexto(e.target)) return;
    // Não interferir em combinações (Ctrl+R, Alt+Tab, etc.).
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    if (carregando) return;

    if (e.key === '0') {
      e.preventDefault();
      registrarResposta('O');
    } else if (e.key === '1') {
      e.preventDefault();
      registrarResposta('X');
    } else if (e.key === 'Backspace') {
      // Backspace fora de um campo navegaria/voltaria a página — sempre prevenir.
      e.preventDefault();
      if ((testandoUltimaSugestao?.respostas?.length ?? 0) > 0) {
        desfazerUltima();
      }
    }
  }

  async function cancelarSequenciaAtiva() {
    if (!testandoSequencia) return;
    if (!confirm("Deseja mesmo descartar esta sequência em andamento? Todas as aplicações feitas nela serão apagadas.")) {
      return;
    }
    erroMsg = null;
    try {
      carregando = true;
      await invokeCommand('cancelar_sequencia', { id: testandoSequencia.id });
      
      showActiveTestScreen = false;
      testandoAnimal = null;
      testandoTimepoint = null;
      testandoSequencia = null;
      testandoUltimaSugestao = null;
      
      if (selectedExpId !== null) {
        listSequencias = await invokeCommand<any[]>('listar_sequencias_concluidas', { experimentoId: selectedExpId });
      }
    } catch (e: any) {
      erroMsg = "Erro ao cancelar sequência: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  async function finalizarTeste() {
    if (!testandoSequencia) return;
    erroMsg = null;
    try {
      carregando = true;
      const resultado = await invokeCommand<any>('finalizar_sequencia', {
        sequenciaId: testandoSequencia.id
      });
      
      salvoLimiar = resultado.limiar;
      salvoAnimal = testandoAnimal?.marcacao || "";
      salvoTimepoint = testandoTimepoint?.rotulo || "";
      
      showActiveTestScreen = false;
      testandoAnimal = null;
      testandoTimepoint = null;
      testandoSequencia = null;
      testandoUltimaSugestao = null;
      
      await carregarDados();
      if (selectedExpId !== null) {
        listSequencias = await invokeCommand<any[]>('listar_sequencias_concluidas', { experimentoId: selectedExpId });
      }
    } catch (e: any) {
      erroMsg = "Erro ao finalizar teste: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  // =============================================================================
  // AÇÕES: FILAMENTOS
  // =============================================================================
  function iniciarCriacaoFilamentos() {
    filamentoEditandoId = null;
    filamentoNome = "";
    filamentoDescricao = "";
    filamentoValuesInput = ["0.008", "0.02", "0.04", "0.07", "0.16", "0.4", "0.6", "1.0", "1.4", "2.0"];
    showFilamentoForm = true;
    salvoD = null;
    salvoNome = null;
    erroMsg = null;
  }

  function iniciarEdicaoFilamentos(c: ConjuntoFilamentos) {
    filamentoEditandoId = c.id;
    filamentoNome = c.nome;
    filamentoDescricao = c.descricao || "";
    filamentoValuesInput = c.valores.map(v => v.toString());
    showFilamentoForm = true;
    salvoD = null;
    salvoNome = null;
    erroMsg = null;
  }

  async function salvarFilamentos() {
    if (!filamentoNome.trim()) {
      erroMsg = "O nome do conjunto é obrigatório.";
      return;
    }
    if (!filamentoCalcResult.ok) {
      erroMsg = filamentoCalcResult.error ?? "Erro ao calcular.";
      return;
    }

    try {
      carregando = true;
      let res: ConjuntoFilamentos;
      if (filamentoEditandoId !== null) {
        res = await invokeCommand<ConjuntoFilamentos>('editar_conjunto', {
          id: filamentoEditandoId,
          nome: filamentoNome.trim(),
          descricao: filamentoDescricao.trim() || null,
          valores: parsedFilamentoValores
        });
      } else {
        res = await invokeCommand<ConjuntoFilamentos>('criar_conjunto', {
          nome: filamentoNome.trim(),
          descricao: filamentoDescricao.trim() || null,
          valores: parsedFilamentoValores
        });
      }
      salvoD = res.d;
      salvoNome = res.nome;
      showFilamentoForm = false;
      await carregarDados();
    } catch (e: any) {
      erroMsg = "Erro ao salvar conjunto: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  async function excluirFilamentos(id: number, nome: string) {
    if (confirm(`Deseja realmente excluir o conjunto "${nome}"?`)) {
      try {
        await invokeCommand('excluir_conjunto', { id });
        await carregarDados();
      } catch (e: any) {
        erroMsg = "Erro ao excluir conjunto: " + (e.message || e);
      }
    }
  }

  // =============================================================================
  // AÇÕES: EXPERIMENTOS
  // =============================================================================
  function iniciarCriacaoExp() {
    expEditandoId = null;
    expNome = "";
    expDescricao = "";
    expConjuntoId = conjuntos.length > 0 ? conjuntos[0].id : null;
    expResponsavel = "";
    expTimepoints = ["basal 1", "indução", "basal 2", "tratamento", "1h", "2h", "4h", "6h"];
    // Wizard começa na etapa 1, já com um grupo em branco pronto para preencher.
    wizardEtapa = 1;
    wizardGrupos = [novoGrupoWizard(0)];
    refMarcacao = {};
    refPeso = {};
    showExpForm = true;
    erroMsg = null;
  }

  // ---------------------------------------------------------------------------
  // WIZARD: grupos e animais montados em memória e salvos de uma vez só
  // ---------------------------------------------------------------------------
  function novoGrupoWizard(indice: number): WizardGrupo {
    return {
      nome: "",
      cor: CORES_GRUPO[indice % CORES_GRUPO.length],
      animais: [],
      novaMarcacao: "",
      novoPeso: "",
    };
  }

  function adicionarGrupoWizard() {
    wizardGrupos = [...wizardGrupos, novoGrupoWizard(wizardGrupos.length)];
  }

  function removerGrupoWizard(indice: number) {
    wizardGrupos = wizardGrupos.filter((_, i) => i !== indice);
  }

  /** Adiciona o animal digitado no card do grupo e devolve o foco à marcação. */
  function adicionarAnimalWizard(gi: number) {
    const g = wizardGrupos[gi];
    if (!g) return;
    const marcacao = g.novaMarcacao.trim();
    if (!marcacao) {
      refMarcacao[gi]?.focus();
      return;
    }
    const pesoTexto = g.novoPeso.trim().replace(",", ".");
    let peso: number | null = null;
    if (pesoTexto !== "") {
      const n = Number(pesoTexto);
      if (!Number.isFinite(n) || n <= 0) {
        erroMsg = `Peso inválido para o animal "${marcacao}" (use um número maior que zero).`;
        refPeso[gi]?.focus();
        return;
      }
      peso = n;
    }
    g.animais = [...g.animais, { marcacao, peso: peso === null ? "" : String(peso) }];
    g.novaMarcacao = "";
    g.novoPeso = "";
    erroMsg = null;
    // Volta o foco para digitar o próximo animal imediatamente.
    setTimeout(() => refMarcacao[gi]?.focus(), 0);
  }

  function removerAnimalWizard(gi: number, ai: number) {
    const g = wizardGrupos[gi];
    if (!g) return;
    g.animais = g.animais.filter((_, i) => i !== ai);
  }

  /** Enter na marcação → pula para o peso. */
  function onEnterMarcacao(e: KeyboardEvent, gi: number) {
    if (e.key === "Enter") {
      e.preventDefault();
      refPeso[gi]?.focus();
    }
  }

  /** Enter no peso → adiciona o animal e volta o foco para uma nova marcação. */
  function onEnterPeso(e: KeyboardEvent, gi: number) {
    if (e.key === "Enter") {
      e.preventDefault();
      adicionarAnimalWizard(gi);
    }
  }

  const totalAnimaisWizard = $derived(
    wizardGrupos.reduce((soma, g) => soma + g.animais.length, 0)
  );

  /**
   * Salva TUDO de uma vez (experimento + timepoints + grupos + animais) via o
   * comando atômico do backend. Em caso de erro, mantém o formulário preenchido.
   */
  async function salvarExperimentoCompleto() {
    if (!expNome.trim()) {
      erroMsg = "O nome do experimento é obrigatório.";
      wizardEtapa = 1;
      return;
    }
    if (expConjuntoId === null) {
      erroMsg = "Selecione um conjunto de filamentos.";
      wizardEtapa = 1;
      return;
    }
    const timepoints = expTimepoints.map(t => t.trim()).filter(t => t !== "");
    if (timepoints.length === 0) {
      erroMsg = "O experimento deve ter pelo menos 1 timepoint.";
      wizardEtapa = 1;
      return;
    }
    // Grupos sem nome só são um problema se o usuário tiver começado a preenchê-los.
    const gruposPreenchidos = wizardGrupos.filter(
      g => g.nome.trim() !== "" || g.animais.length > 0
    );
    for (const g of gruposPreenchidos) {
      if (!g.nome.trim()) {
        erroMsg = "Há um grupo com animais mas sem nome. Dê um nome ao grupo.";
        wizardEtapa = 2;
        return;
      }
    }

    try {
      carregando = true;
      await invokeCommand('criar_experimento_completo', {
        nome: expNome.trim(),
        descricao: expDescricao.trim() || null,
        conjuntoId: expConjuntoId,
        responsavel: expResponsavel.trim() || null,
        timepoints,
        grupos: gruposPreenchidos.map(g => ({
          nome: g.nome.trim(),
          cor: g.cor,
          animais: g.animais.map(a => ({
            marcacao: a.marcacao,
            peso: a.peso === "" ? null : Number(a.peso),
          })),
        })),
      });
      // Só limpa/fecha em caso de sucesso.
      showExpForm = false;
      erroMsg = null;
      await carregarDados();
    } catch (e: any) {
      // Nada foi salvo (transação atômica) e o formulário permanece preenchido.
      erroMsg = "Erro ao salvar o experimento (nada foi gravado): " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  function iniciarEdicaoExp(e: ExperimentoCompleto) {
    expEditandoId = e.id;
    expNome = e.nome;
    expDescricao = e.descricao || "";
    expConjuntoId = e.conjunto_id;
    expResponsavel = e.responsavel || "";
    expTimepoints = e.timepoints.map(t => t.rotulo);
    showExpForm = true;
    erroMsg = null;
  }

  async function salvarExp() {
    if (!expNome.trim()) {
      erroMsg = "O nome do experimento é obrigatório.";
      return;
    }
    if (expConjuntoId === null) {
      erroMsg = "Selecione um conjunto de filamentos.";
      return;
    }
    if (expTimepoints.length === 0) {
      erroMsg = "O experimento deve ter pelo menos 1 timepoint.";
      return;
    }

    try {
      carregando = true;
      if (expEditandoId !== null) {
        await invokeCommand('editar_experimento', {
          id: expEditandoId,
          nome: expNome.trim(),
          descricao: expDescricao.trim() || null,
          conjuntoId: expConjuntoId,
          responsavel: expResponsavel.trim() || null,
          timepoints: expTimepoints.filter(t => t.trim() !== '')
        });
      } else {
        await invokeCommand('criar_experimento', {
          nome: expNome.trim(),
          descricao: expDescricao.trim() || null,
          conjuntoId: expConjuntoId,
          responsavel: expResponsavel.trim() || null,
          timepoints: expTimepoints.filter(t => t.trim() !== '')
        });
      }
      showExpForm = false;
      await carregarDados();
    } catch (e: any) {
      erroMsg = "Erro ao salvar experimento: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  async function excluirExp(id: number, nome: string) {
    if (confirm(`Deseja realmente excluir o experimento "${nome}"? Todos os animais e grupos dependentes ficarão ocultados.`)) {
      try {
        await invokeCommand('excluir_experimento', { id });
        if (selectedExpId === id) {
          selectedExpId = null;
        }
        await carregarDados();
      } catch (e: any) {
        erroMsg = "Erro ao excluir experimento: " + (e.message || e);
      }
    }
  }

  function adicionarTimepointInput() {
    expTimepoints = [...expTimepoints, ""];
  }

  function removerTimepointInput(index: number) {
    expTimepoints = expTimepoints.filter((_, i) => i !== index);
  }

  // =============================================================================
  // AÇÕES: GRUPOS
  // =============================================================================
  function iniciarCriacaoGrupo() {
    grupoEditandoId = null;
    grupoNome = "";
    grupoCor = "#3b82f6";
    showGroupForm = true;
    erroMsg = null;
  }

  function iniciarEdicaoGrupo(g: GrupoCompleto) {
    grupoEditandoId = g.id;
    grupoNome = g.nome;
    grupoCor = g.cor;
    showGroupForm = true;
    erroMsg = null;
  }

  async function salvarGrupo() {
    if (!grupoNome.trim()) {
      erroMsg = "O nome do grupo é obrigatório.";
      return;
    }
    if (selectedExpId === null) return;

    try {
      carregando = true;
      if (grupoEditandoId !== null) {
        await invokeCommand('editar_grupo', {
          id: grupoEditandoId,
          nome: grupoNome.trim(),
          cor: grupoCor
        });
      } else {
        await invokeCommand('criar_grupo', {
          experimentoId: selectedExpId,
          nome: grupoNome.trim(),
          cor: grupoCor
        });
      }
      showGroupForm = false;
      await carregarDados();
    } catch (e: any) {
      erroMsg = "Erro ao salvar grupo: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  async function excluirGrupo(id: number, nome: string) {
    if (confirm(`Deseja realmente excluir o grupo "${nome}"? Todos os animais associados a este grupo também serão ocultados.`)) {
      try {
        await invokeCommand('excluir_grupo', { id });
        await carregarDados();
      } catch (e: any) {
        erroMsg = "Erro ao excluir grupo: " + (e.message || e);
      }
    }
  }

  // =============================================================================
  // AÇÕES: ANIMAIS
  // =============================================================================
  function iniciarCriacaoAnimal(grupoId: number) {
    animalEditandoId = null;
    animalMarcacao = "";
    animalPeso = "";
    animalGrupoId = grupoId;
    showAnimalForm = true;
    erroMsg = null;
  }

  function iniciarEdicaoAnimal(a: Animal) {
    animalEditandoId = a.id;
    animalMarcacao = a.marcacao;
    animalPeso = a.peso ? a.peso.toString() : "";
    animalGrupoId = a.grupo_id;
    showAnimalForm = true;
    erroMsg = null;
  }

  async function salvarAnimal() {
    if (!animalMarcacao.trim()) {
      erroMsg = "A marcação do animal é obrigatória.";
      return;
    }
    if (animalGrupoId === null || selectedExpId === null) return;

    const pesoVal = animalPeso.trim() ? parseFloat(animalPeso.replace(',', '.')) : null;
    if (pesoVal !== null && (isNaN(pesoVal) || pesoVal <= 0)) {
      erroMsg = "Insira um peso válido e maior que zero.";
      return;
    }

    try {
      carregando = true;
      if (animalEditandoId !== null) {
        await invokeCommand('editar_animal', {
          id: animalEditandoId,
          grupoId: animalGrupoId,
          marcacao: animalMarcacao.trim(),
          peso: pesoVal
        });
      } else {
        await invokeCommand('criar_animal', {
          experimentoId: selectedExpId,
          grupoId: animalGrupoId,
          marcacao: animalMarcacao.trim(),
          peso: pesoVal
        });
      }
      showAnimalForm = false;
      await carregarDados();
    } catch (e: any) {
      erroMsg = "Erro ao salvar animal: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  async function excluirAnimal(id: number, marcacao: string) {
    if (confirm(`Deseja realmente excluir o animal "${marcacao}"?`)) {
      try {
        await invokeCommand('excluir_animal', { id });
        await carregarDados();
      } catch (e: any) {
        erroMsg = "Erro ao excluir animal: " + (e.message || e);
      }
    }
  }

  let expStats = $state<any[]>([]);
  const IS_TAURI = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

  async function carregarEstatisticasGrafico() {
    if (selectedExpId === null) return;
    carregando = true;
    try {
      expStats = await invokeCommand<any[]>('calcular_estatisticas_experimento', { experimentoId: selectedExpId });
      expTab = 'grafico';
    } catch (e: any) {
      erroMsg = "Erro ao carregar estatísticas do experimento: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  async function salvarArquivoTexto(nomeSugerido: string, extensao: string, conteudo: string, mime: string) {
    if (IS_TAURI) {
      try {
        const filePath = await save({
          defaultPath: nomeSugerido,
          filters: [{ name: extensao.toUpperCase(), extensions: [extensao] }]
        });
        if (filePath) {
          await writeTextFile(filePath, conteudo);
          alert('Arquivo gravado com sucesso em: ' + filePath);
        }
        return;
      } catch (e: any) {
        console.error("Erro ao salvar usando FS nativo do Tauri:", e);
      }
    }
    
    // Fallback navegador
    const blob = new Blob([conteudo], { type: mime });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = nomeSugerido;
    link.click();
    URL.revokeObjectURL(url);
  }

  async function salvarArquivoBinario(nomeSugerido: string, extensao: string, base64: string, mime: string, arrayBuffer?: ArrayBuffer) {
    if (IS_TAURI) {
      try {
        const filePath = await save({
          defaultPath: nomeSugerido,
          filters: [{ name: extensao.toUpperCase(), extensions: [extensao] }]
        });
        if (filePath) {
          const uint8 = arrayBuffer ? new Uint8Array(arrayBuffer) : Uint8Array.from(atob(base64), c => c.charCodeAt(0));
          await writeFile(filePath, uint8);
          alert('Arquivo gravado com sucesso em: ' + filePath);
        }
        return;
      } catch (e: any) {
        console.error("Erro ao salvar usando FS nativo do Tauri:", e);
      }
    }
    
    // Fallback navegador
    const blob = arrayBuffer 
      ? new Blob([arrayBuffer], { type: mime })
      : new Blob([Uint8Array.from(atob(base64), c => c.charCodeAt(0))], { type: mime });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = nomeSugerido;
    link.click();
    URL.revokeObjectURL(url);
  }

  // =============================================================================
  // EXPORTAÇÃO PARA GRAPHPAD PRISM (tabela "Grouped" via clipboard TSV)
  // =============================================================================
  let prismPreview = $state<TabelaPrism | null>(null);
  let prismCopiado = $state(false);
  let prismTimeout: ReturnType<typeof setTimeout> | null = null;

  /** Copia texto para o clipboard: usa a API do navegador/webview, com fallback. */
  async function copiarTexto(texto: string): Promise<boolean> {
    try {
      if (navigator.clipboard && window.isSecureContext) {
        await navigator.clipboard.writeText(texto);
        return true;
      }
    } catch {
      // cai no fallback abaixo
    }
    try {
      const ta = document.createElement('textarea');
      ta.value = texto;
      ta.style.position = 'fixed';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.focus();
      ta.select();
      const ok = document.execCommand('copy');
      document.body.removeChild(ta);
      return ok;
    } catch {
      return false;
    }
  }

  /** Monta o TSV dos limiares no formato do Prism e copia para o clipboard. */
  async function copiarParaPrism() {
    if (selectedExpId === null || !selectedExp) return;
    carregando = true;
    try {
      const limiaresData = await invokeCommand<any[]>('obter_limiares_experimento', { experimentoId: selectedExpId });
      const tabela = montarTabelaPrism(selectedExp, limiaresData);
      prismPreview = tabela;
      const tsv = tabelaPrismParaTsv(tabela);
      const ok = await copiarTexto(tsv);
      if (ok) {
        prismCopiado = true;
        if (prismTimeout) clearTimeout(prismTimeout);
        prismTimeout = setTimeout(() => { prismCopiado = false; }, 2000);
      } else {
        erroMsg = 'Não foi possível copiar automaticamente. Use a pré-visualização abaixo: selecione o texto e copie com Ctrl+C.';
      }
    } catch (e: any) {
      erroMsg = 'Erro ao preparar dados para o Prism: ' + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  async function exportarCSV() {
    if (selectedExpId === null || !selectedExp) return;
    carregando = true;
    try {
      const rawData = await invokeCommand<any[]>('obter_respostas_cruas_experimento', { experimentoId: selectedExpId });
      const limiaresData = await invokeCommand<any[]>('obter_limiares_experimento', { experimentoId: selectedExpId });

      let csv = "--- DADOS BRUTOS (RESPOSTAS INDIVIDUAIS COM VALORES DE FILAMENTO) ---\n";
      csv += "Grupo,Animal,Timepoint,Ordem,Filamento (g),Resposta\n";
      for (const r of rawData) {
        csv += `"${r.grupo}","${r.animal}","${r.timepoint}",${r.ordem},${r.filamento},"${r.resposta}"\n`;
      }
      csv += "\n";
      csv += "--- LIMIARES DIXON FINAIS ---\n";
      csv += "Grupo,Animal,Timepoint,Filamento Inicial (g),Serie de Respostas,Limiar PWT (g)\n";
      for (const l of limiaresData) {
        const limVal = l.limiar !== null && l.limiar !== undefined ? l.limiar.toFixed(4) : '';
        csv += `"${l.grupo}","${l.animal}","${l.timepoint}",${l.filamento_inicial},"${l.serie_respostas ?? ''}",${limVal}\n`;
      }

      await salvarArquivoTexto(`experimento_${selectedExp.nome.replace(/\s+/g, '_')}_export.csv`, 'csv', csv, 'text/csv;charset=utf-8');
    } catch (e: any) {
      erroMsg = "Erro ao exportar CSV: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  async function exportarXLSX() {
    if (selectedExpId === null || !selectedExp) return;
    carregando = true;
    try {
      const rawData = await invokeCommand<any[]>('obter_respostas_cruas_experimento', { experimentoId: selectedExpId });
      const limiaresData = await invokeCommand<any[]>('obter_limiares_experimento', { experimentoId: selectedExpId });

      // Sheet 1: Dados Brutos
      const rawHeaders = ["Grupo", "Animal", "Timepoint", "Ordem da Aplicação", "Força Aplicada (g)", "Resposta"];
      const rawRows = rawData.map(r => [r.grupo, r.animal, r.timepoint, r.ordem, r.filamento, r.resposta]);
      const wsRaw = XLSX.utils.aoa_to_sheet([rawHeaders, ...rawRows]);

      // Sheet 2: Limiares
      const limiaresHeaders = ["Grupo", "Animal", "Timepoint", "Filamento Inicial (g)", "Série de Respostas", "Limiar PWT (g)"];
      const limiaresRows = limiaresData.map(l => [
        l.grupo, 
        l.animal, 
        l.timepoint, 
        l.filamento_inicial, 
        l.serie_respostas ?? '', 
        l.limiar !== null && l.limiar !== undefined ? l.limiar : ''
      ]);
      const wsLimiares = XLSX.utils.aoa_to_sheet([limiaresHeaders, ...limiaresRows]);

      // Sheet 3: Resumo Estatístico por Grupo
      const statsHeaders = ["Grupo", "Timepoint", "N (Amostra)", "Média Geométrica (g)", "Limite Inferior EP (g)", "Limite Superior EP (g)"];
      const statsRows = expStats.map(s => [
        s.grupo_nome, 
        s.timepoint_rotulo, 
        s.n, 
        s.media_geometrica_g, 
        s.limite_inferior_g !== null && s.limite_inferior_g !== undefined ? s.limite_inferior_g : '', 
        s.limite_superior_g !== null && s.limite_superior_g !== undefined ? s.limite_superior_g : ''
      ]);
      const wsStats = XLSX.utils.aoa_to_sheet([statsHeaders, ...statsRows]);

      // Workbook
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, wsRaw, "Dados Brutos");
      XLSX.utils.book_append_sheet(wb, wsLimiares, "Limiares");
      XLSX.utils.book_append_sheet(wb, wsStats, "Resumo por Grupo");

      const wbout = XLSX.write(wb, { bookType: 'xlsx', type: 'array' });
      const wbbase64 = XLSX.write(wb, { bookType: 'xlsx', type: 'base64' });

      await salvarArquivoBinario(
        `experimento_${selectedExp.nome.replace(/\s+/g, '_')}_export.xlsx`, 
        'xlsx', 
        wbbase64, 
        'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet', 
        wbout
      );
    } catch (e: any) {
      erroMsg = "Erro ao exportar XLSX: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  async function exportarPDF() {
    if (selectedExpId === null || !selectedExp) return;
    carregando = true;
    try {
      const doc = new jsPDF();
      
      // Cabeçalho e metadados
      doc.setFont("helvetica", "bold");
      doc.setFontSize(16);
      doc.setTextColor(40, 40, 40);
      doc.text("Relatório Nociceptivo - Método Dixon Up-Down", 14, 20);
      
      doc.setFont("helvetica", "normal");
      doc.setFontSize(10);
      doc.setTextColor(80, 80, 80);
      doc.text(`Experimento: ${selectedExp.nome}`, 14, 30);
      doc.text(`Descrição: ${selectedExp.descricao || 'Sem descrição'}`, 14, 36);
      doc.text(`Responsável: ${selectedExp.responsavel || 'Não informado'}`, 14, 42);
      doc.text(`Conjunto de Filamentos: ${selectedExp.conjunto_nome}`, 14, 48);
      doc.text(`Data de Geração: ${new Date().toLocaleString()}`, 14, 54);
      
      doc.setDrawColor(200, 200, 200);
      doc.line(14, 60, 196, 60);

      // Embutir gráfico SVG
      const svgEl = document.querySelector('.temporal-chart-svg') as SVGSVGElement;
      if (svgEl) {
        await new Promise<void>((resolve) => {
          const serializer = new XMLSerializer();
          let source = serializer.serializeToString(svgEl);
          if (!source.match(/^<svg[^>]+xmlns="http:\/\/www\.w3\.org\/2000\/svg"/)) {
            source = source.replace(/^<svg/, '<svg xmlns="http://www.w3.org/2000/svg"');
          }
          
          const img = new Image();
          img.src = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(source);
          img.onload = () => {
            const canvas = document.createElement('canvas');
            canvas.width = svgEl.clientWidth * 2;
            canvas.height = svgEl.clientHeight * 2;
            const ctx = canvas.getContext('2d');
            if (ctx) {
              ctx.scale(2, 2);
              ctx.fillStyle = '#ffffff';
              ctx.fillRect(0, 0, svgEl.clientWidth, svgEl.clientHeight);
              ctx.drawImage(img, 0, 0);
              const imgData = canvas.toDataURL('image/png');
              
              doc.setFont("helvetica", "bold");
              doc.setFontSize(11);
              doc.setTextColor(40, 40, 40);
              doc.text("Curva Temporal de Limiares Mecânicos (g)", 14, 68);
              doc.addImage(imgData, 'PNG', 14, 72, 175, 100);
            }
            resolve();
          };
          img.onerror = () => resolve();
        });
      }

      // Tabela de Estatísticas
      doc.setFont("helvetica", "bold");
      doc.setFontSize(11);
      doc.text("Resumo Estatístico por Grupo (Média Geométrica ± Erro Padrão)", 14, 185);
      
      doc.setFont("helvetica", "normal");
      doc.setFontSize(9);
      
      let y = 194;
      doc.setFillColor(240, 240, 240);
      doc.rect(14, y - 4, 182, 6, "F");
      doc.text("Grupo", 16, y);
      doc.text("Timepoint", 60, y);
      doc.text("N", 100, y);
      doc.text("Média Geométrica (g)", 115, y);
      doc.text("Intervalo EP (g)", 155, y);
      
      doc.setDrawColor(220, 220, 220);
      doc.line(14, y + 2, 196, y + 2);
      y += 8;
      
      for (const s of expStats) {
        if (y > 275) {
          doc.addPage();
          y = 25;
          
          doc.setFillColor(240, 240, 240);
          doc.rect(14, y - 4, 182, 6, "F");
          doc.text("Grupo", 16, y);
          doc.text("Timepoint", 60, y);
          doc.text("N", 100, y);
          doc.text("Média Geométrica (g)", 115, y);
          doc.text("Intervalo EP (g)", 155, y);
          doc.line(14, y + 2, 196, y + 2);
          y += 8;
        }
        
        const infStr = s.limite_inferior_g !== null && s.limite_inferior_g !== undefined ? `${s.limite_inferior_g.toFixed(3)}g` : '';
        const supStr = s.limite_superior_g !== null && s.limite_superior_g !== undefined ? `${s.limite_superior_g.toFixed(3)}g` : '';
        const epStr = infStr ? `[${infStr} - ${supStr}]` : 'Indefinido (N=1)';
        
        doc.text(s.grupo_nome, 16, y);
        doc.text(s.timepoint_rotulo, 60, y);
        doc.text(s.n.toString(), 100, y);
        doc.text(`${s.media_geometrica_g.toFixed(3)} g`, 115, y);
        doc.text(epStr, 155, y);
        
        doc.line(14, y + 2, 196, y + 2);
        y += 8;
      }

      const pdfBase64 = doc.output('datauristring').split(',')[1];
      const pdfArrayBuffer = doc.output('arraybuffer');
      
      await salvarArquivoBinario(
        `experimento_${selectedExp.nome.replace(/\s+/g, '_')}_relatorio.pdf`, 
        'pdf', 
        pdfBase64, 
        'application/pdf', 
        pdfArrayBuffer
      );
    } catch (e: any) {
      erroMsg = "Erro ao gerar PDF: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }

  onMount(() => {
    carregarDados();
  });
</script>

<!-- Atalhos de teclado do fluxo de teste (0 / 1 / Backspace).
     O handler já checa se a tela de teste está ativa e se o foco está num campo. -->
<svelte:window onkeydown={handleAtalhosTeste} />

<main class="app-container">
  <!-- Abas Globais de Navegação -->
  <nav class="app-tabs">
    <button 
      onclick={() => { activeTab = 'experimentos'; selectedExpId = null; showExpForm = false; showFilamentoForm = false; }} 
      class={activeTab === 'experimentos' ? 'tab-active' : ''}
    >
      Experimentos & Grupos
    </button>
    <button 
      onclick={() => { activeTab = 'filamentos'; showFilamentoForm = false; showExpForm = false; }} 
      class={activeTab === 'filamentos' ? 'tab-active' : ''}
    >
      Conjuntos de Filamentos
    </button>
  </nav>

  <header class="app-header">
    {#if activeTab === 'experimentos'}
      <h1>Experimentos</h1>
      <p class="subtitle">Estrutura de estudos, tratamentos e animais do laboratório (Etapa 3)</p>
    {:else}
      <h1>Filamentos</h1>
      <p class="subtitle">Cadastro do Conjunto de Filamentos de von Frey (Etapa 2)</p>
    {/if}
  </header>

  <!-- Mensagens de Erro Globais -->
  {#if erroMsg}
    <div class="alert alert-error">
      <strong>Erro:</strong> {erroMsg}
      <button onclick={() => erroMsg = null} class="btn-close">&times;</button>
    </div>
  {/if}

  <!-- TAB 1: FILAMENTOS DE VON FREY -->
  {#if activeTab === 'filamentos'}
    {#if salvoD !== null}
      <div class="alert alert-success">
        <div class="success-icon">✓</div>
        <div>
          <strong>Salvo com sucesso!</strong>
          <p>O conjunto <strong>"{salvoNome}"</strong> foi persistido.</p>
          <p class="d-val-highlight">Valor <code>d</code> calculado: <strong>{salvoD.toFixed(4)}</strong></p>
        </div>
        <button onclick={() => { salvoD = null; salvoNome = null; }} class="btn-close">&times;</button>
      </div>
    {/if}

    <div class="layout-grid">
      <section class="section-list">
        <div class="section-header">
          <h2>Conjuntos Cadastrados</h2>
          <button onclick={iniciarCriacaoFilamentos} class="btn-primary" disabled={showFilamentoForm}>
            + Novo Conjunto
          </button>
        </div>

        {#if conjuntos.length === 0}
          <div class="empty-state">
            <p>Nenhum conjunto cadastrado.</p>
          </div>
        {:else}
          <div class="cards-grid">
            {#each conjuntos as c (c.id)}
              <div class="card {filamentoEditandoId === c.id ? 'card-editing' : ''}">
                <div class="card-body">
                  <div class="card-title-row">
                    <h3>{c.nome}</h3>
                    <span class="badge-d">d = {c.d.toFixed(4)}</span>
                  </div>
                  {#if c.descricao}
                    <p class="card-description">{c.descricao}</p>
                  {/if}
                  <div class="filaments-preview">
                    <strong>Filamentos ({c.valores.length}):</strong>
                    <div class="tags-container">
                      {#each c.valores as val}
                        <span class="tag-val">{val}g</span>
                      {/each}
                    </div>
                  </div>
                </div>
                <div class="card-actions">
                  <button onclick={() => iniciarEdicaoFilamentos(c)} class="btn-secondary" disabled={showFilamentoForm}>Editar</button>
                  <button onclick={() => excluirFilamentos(c.id, c.nome)} class="btn-danger" disabled={showFilamentoForm}>Excluir</button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </section>

      {#if showFilamentoForm}
        <section class="section-form">
          <div class="form-container">
            <h2>{filamentoEditandoId !== null ? "Editar Conjunto" : "Novo Conjunto"}</h2>
            
            <div class="form-group">
              <label for="fil-nome">Nome do Conjunto *</label>
              <input id="fil-nome" type="text" bind:value={filamentoNome} placeholder="Ex: Kit padrão do laboratório" class="form-control" />
            </div>

            <div class="form-group">
              <label for="fil-desc">Descrição (Opcional)</label>
              <textarea id="fil-desc" bind:value={filamentoDescricao} placeholder="Ex: Usado na bancada de camundongos" class="form-control" rows="2"></textarea>
            </div>

            <div class="form-group">
              <div class="filaments-header">
                <span class="form-label-text">Valores dos Filamentos (g)</span>
                <button onclick={() => filamentoValuesInput = [...filamentoValuesInput, ""]} class="btn-small-add" type="button">+ Adicionar</button>
              </div>
              <div class="filaments-inputs-grid">
                {#each filamentoValuesInput as val, idx}
                  <div class="filament-input-row">
                    <span class="row-num">{idx + 1}.</span>
                    <input type="text" bind:value={filamentoValuesInput[idx]} placeholder="ex: 0.16" class="form-control-small" />
                    <span class="unit-g">g</span>
                    <button onclick={() => filamentoValuesInput = filamentoValuesInput.filter((_, i) => i !== idx)} class="btn-remove" disabled={filamentoValuesInput.length <= 2}>&times;</button>
                  </div>
                {/each}
              </div>
            </div>

            <div class="calc-feedback-box {filamentoCalcResult.ok ? 'feedback-ok' : 'feedback-error'}">
              {#if filamentoCalcResult.ok}
                <div class="calc-success">
                  <span class="calc-label">Valor d calculado:</span>
                  <span class="calc-value">d = {(filamentoCalcResult.d ?? 0).toFixed(4)}</span>
                </div>
              {:else}
                <div class="calc-error"><span>⚠️ {filamentoCalcResult.error}</span></div>
              {/if}
            </div>

            <div class="form-actions">
              <button onclick={() => showFilamentoForm = false} class="btn-link" type="button">Cancelar</button>
              <button onclick={salvarFilamentos} class="btn-primary" disabled={!filamentoNome.trim() || !filamentoCalcResult.ok || carregando}>Salvar</button>
            </div>
          </div>
        </section>
      {:else}
        <section class="section-help">
          <div class="help-box">
            <h3>Cálculo de d</h3>
            <p>O <code>d</code> é a média das diferenças entre o log10 das forças consecutivas de von Frey.</p>
            <div class="math-expr">d = média( log₁₀(f_i₊₁) - log₁₀(f_i) )</div>
            <p>Necessário para alimentar o algoritmo de limiar (Dixon).</p>
          </div>
        </section>
      {/if}
    </div>
  {/if}

  <!-- TAB 2: EXPERIMENTOS E ANIMAIS -->
  {#if activeTab === 'experimentos'}
    {#if selectedExpId === null}
      <!-- LISTAGEM DE EXPERIMENTOS -->
      <div class="layout-grid">
        <section class="section-list">
          <div class="section-header">
            <h2>Experimentos Ativos</h2>
            <button onclick={iniciarCriacaoExp} class="btn-primary" disabled={showExpForm}>
              + Novo Experimento
            </button>
          </div>

          {#if experimentos.length === 0}
            <div class="empty-state">
              <p>Nenhum experimento cadastrado.</p>
              <p>Registre um experimento para organizar seus grupos de tratamento e animais.</p>
            </div>
          {:else}
            <div class="cards-grid">
              {#each experimentos as e (e.id)}
                <div class="card">
                  <div class="card-body">
                    <div class="card-title-row">
                      <h3>{e.nome}</h3>
                      <span class="badge-d">Kit: {e.conjunto_nome}</span>
                    </div>
                    {#if e.descricao}
                      <p class="card-description">{e.descricao}</p>
                    {/if}
                    <div class="exp-summary-details">
                      <p><strong>Responsável:</strong> {e.responsavel || "Não informado"}</p>
                      <p><strong>Grupos:</strong> {e.grupos.length} | <strong>Animais:</strong> {e.grupos.reduce((sum, g) => sum + g.animais.length, 0)}</p>
                      <p><strong>Curva Temporal:</strong> {e.timepoints.map(t => t.rotulo).join(' → ')}</p>
                    </div>
                  </div>
                  <div class="card-actions">
                    <button onclick={() => selectedExpId = e.id} class="btn-secondary" style="font-weight: bold; color: var(--accent);">Gerenciar Estrutura</button>
                    <button onclick={() => iniciarEdicaoExp(e)} class="btn-secondary">Editar</button>
                    <button onclick={() => excluirExp(e.id, e.nome)} class="btn-danger">Excluir</button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </section>

        <!-- COLUNA DA DIREITA: FORMULÁRIO DO EXPERIMENTO OU AJUDA -->
        {#if showExpForm}
          <section class="section-form">
            <div class="form-container">
              <h2>{expEditandoId !== null ? "Editar Experimento" : "Novo Experimento"}</h2>

              {#if expEditandoId === null}
                <!-- Indicador de etapas do wizard -->
                <div class="wizard-steps">
                  <span class="wizard-step {wizardEtapa === 1 ? 'ativo' : 'feito'}">1. Dados & timepoints</span>
                  <span class="wizard-step-sep">→</span>
                  <span class="wizard-step {wizardEtapa === 2 ? 'ativo' : ''}">2. Grupos & animais</span>
                </div>
              {/if}

              {#if expEditandoId !== null || wizardEtapa === 1}
              <div class="form-group">
                <label for="exp-nome">Nome do Experimento *</label>
                <input id="exp-nome" type="text" bind:value={expNome} placeholder="Ex: Teste Basal de Hiperalgesia Morfina" class="form-control" />
              </div>

              <div class="form-group">
                <label for="exp-desc">Descrição (Opcional)</label>
                <textarea id="exp-desc" bind:value={expDescricao} placeholder="Descrição dos objetivos, dosagens..." class="form-control" rows="2"></textarea>
              </div>

              <div class="form-group">
                <label for="exp-resp">Pesquisador Responsável</label>
                <input id="exp-resp" type="text" bind:value={expResponsavel} placeholder="Ex: Dr. Silva" class="form-control" />
              </div>

              <div class="form-group">
                <label for="exp-kit">Conjunto de Filamentos (von Frey) *</label>
                {#if conjuntos.length === 0}
                  <p class="select-kit-warning">⚠️ Cadastre um kit de filamentos na outra aba primeiro!</p>
                {:else}
                  <select id="exp-kit" bind:value={expConjuntoId} class="form-control">
                    {#each conjuntos as c}
                      <option value={c.id}>{c.nome} (d = {c.d.toFixed(4)})</option>
                    {/each}
                  </select>
                {/if}
              </div>

              <div class="form-group">
                <div class="filaments-header">
                  <span class="form-label-text">Curva Temporal (Timepoints na ordem)</span>
                  <button onclick={adicionarTimepointInput} class="btn-small-add" type="button">+ Adicionar</button>
                </div>
                <div class="timepoints-inputs-list">
                  {#each expTimepoints as tp, idx}
                    <div class="timepoint-input-row">
                      <span class="row-num">{idx + 1}.</span>
                      <input type="text" bind:value={expTimepoints[idx]} placeholder="Ex: basal, 1h, 24h" class="form-control-small" />
                      <button onclick={() => removerTimepointInput(idx)} class="btn-remove" disabled={expTimepoints.length <= 1}>&times;</button>
                    </div>
                  {/each}
                </div>
              </div>

              {#if expEditandoId !== null}
                <!-- MODO EDIÇÃO: mantém o comportamento anterior (só dados + timepoints). -->
                <div class="form-actions">
                  <button onclick={() => showExpForm = false} class="btn-link" type="button">Cancelar</button>
                  <button onclick={salvarExp} class="btn-primary" disabled={!expNome.trim() || expConjuntoId === null || carregando}>Salvar Experimento</button>
                </div>
              {:else}
                <!-- MODO CRIAÇÃO (wizard): avança para grupos/animais na mesma tela. -->
                <div class="form-actions">
                  <button onclick={() => showExpForm = false} class="btn-link" type="button">Cancelar</button>
                  <button onclick={() => { erroMsg = null; wizardEtapa = 2; }} class="btn-primary"
                          disabled={!expNome.trim() || expConjuntoId === null || expTimepoints.filter(t => t.trim()).length === 0}>
                    Próximo: Grupos e Animais →
                  </button>
                </div>
              {/if}
            {/if}

            {#if expEditandoId === null && wizardEtapa === 2}
              <!-- ================= ETAPA 2: GRUPOS + ANIMAIS (inline) ================= -->
              <div class="wizard-resumo">
                <strong>{expNome.trim() || "(sem nome)"}</strong>
                — {expTimepoints.filter(t => t.trim()).length} timepoints
                · {wizardGrupos.filter(g => g.nome.trim() || g.animais.length).length} grupos
                · {totalAnimaisWizard} animais
                <button class="btn-link" type="button" onclick={() => { erroMsg = null; wizardEtapa = 1; }}>editar dados</button>
              </div>

              <div class="form-group">
                <div class="filaments-header">
                  <span class="form-label-text">Grupos de tratamento e seus animais</span>
                  <button onclick={adicionarGrupoWizard} class="btn-small-add" type="button">+ Adicionar grupo</button>
                </div>
                <p class="wizard-dica">
                  Dica: digite a marcação e pressione <kbd>Enter</kbd> para ir ao peso;
                  <kbd>Enter</kbd> de novo adiciona o animal e já volta para a marcação seguinte.
                </p>

                {#each wizardGrupos as g, gi (gi)}
                  <div class="wizard-grupo-card" style="border-left: 5px solid {g.cor};">
                    <div class="wizard-grupo-head">
                      <input type="text" bind:value={g.nome} placeholder="Nome do grupo (ex: Controle, Morfina 5mg/kg)"
                             class="form-control-small" style="flex: 1;" />
                      <input type="color" bind:value={g.cor} class="wizard-cor" title="Cor do grupo" />
                      <span class="wizard-contador">{g.animais.length} animal(is)</span>
                      <button onclick={() => removerGrupoWizard(gi)} class="btn-remove"
                              disabled={wizardGrupos.length <= 1} title="Remover grupo">&times;</button>
                    </div>

                    {#if g.animais.length > 0}
                      <div class="wizard-animais-lista">
                        {#each g.animais as a, ai (ai)}
                          <span class="wizard-animal-chip">
                            <strong>{a.marcacao}</strong>{a.peso ? ` · ${a.peso} g` : ''}
                            <button onclick={() => removerAnimalWizard(gi, ai)} title="Remover animal">&times;</button>
                          </span>
                        {/each}
                      </div>
                    {/if}

                    <div class="wizard-add-animal">
                      <input type="text" bind:value={g.novaMarcacao}
                             bind:this={refMarcacao[gi]}
                             onkeydown={(e) => onEnterMarcacao(e, gi)}
                             placeholder="Marcação (ex: 4P, 2L)" class="form-control-small" />
                      <input type="text" bind:value={g.novoPeso}
                             bind:this={refPeso[gi]}
                             onkeydown={(e) => onEnterPeso(e, gi)}
                             placeholder="Peso g (opcional)" class="form-control-small" style="max-width: 150px;" />
                      <button onclick={() => adicionarAnimalWizard(gi)} class="btn-small-add" type="button">+ Animal</button>
                    </div>
                  </div>
                {/each}
              </div>

              <div class="form-actions">
                <button onclick={() => { erroMsg = null; wizardEtapa = 1; }} class="btn-link" type="button">← Voltar</button>
                <button onclick={() => showExpForm = false} class="btn-link" type="button">Cancelar</button>
                <button onclick={salvarExperimentoCompleto} class="btn-primary"
                        disabled={!expNome.trim() || expConjuntoId === null || carregando}>
                  {carregando ? 'Salvando…' : '💾 Salvar experimento completo'}
                </button>
              </div>
            {/if}
            </div>
          </section>
        {:else}
          <section class="section-help">
            <div class="help-box">
              <h3>Desenho Experimental</h3>
              <p>Cadastre o seu experimento definindo:</p>
              <ul>
                <li><strong>Conjunto de Filamentos</strong> que será utilizado nas medições, para travar o valor de <code>d</code> correto da fórmula.</li>
                <li><strong>Timepoints</strong> da curva temporal do estudo. A ordem deles é muito importante para a plotagem gráfica posterior.</li>
              </ul>
            </div>
          </section>
        {/if}
      </div>
    {:else if selectedExp}
      <!-- ESTRUTURA DETALHADA DO EXPERIMENTO SELECIONADO -->
      <div class="expanded-exp-view">
        <div class="detail-header-nav">
          <button onclick={() => { selectedExpId = null; showGroupForm = false; showAnimalForm = false; showActiveTestScreen = false; showStartTestForm = false; salvoLimiar = null; }} class="btn-back">
            ← Voltar para Experimentos
          </button>
          {#if expTab === 'estrutura'}
            <div class="detail-header-actions">
              <button onclick={() => iniciarCriacaoGrupo()} class="btn-primary" disabled={showGroupForm || showAnimalForm}>+ Novo Grupo</button>
            </div>
          {/if}
        </div>

        <div class="app-tabs" style="margin-top: 12px; margin-bottom: 16px;">
          <button class={expTab === 'matriz' ? 'tab-active' : ''} onclick={() => { expTab = 'matriz'; showGroupForm = false; showAnimalForm = false; }}>
            🧪 Executar Medições (Dixon)
          </button>
          <button class={expTab === 'estrutura' ? 'tab-active' : ''} onclick={() => { expTab = 'estrutura'; }}>
            ⚙️ Configurar Grupos & Animais
          </button>
          <button class={expTab === 'grafico' ? 'tab-active' : ''} onclick={carregarEstatisticasGrafico}>
            📊 Gráfico & Exportação
          </button>
        </div>

        <div class="detail-banner">
          <div class="detail-banner-title">
            <h2>{selectedExp.nome}</h2>
            <span class="badge-d">Kit de Filamentos: {selectedExp.conjunto_nome}</span>
          </div>
          {#if selectedExp.descricao}
            <p class="detail-banner-desc">{selectedExp.descricao}</p>
          {/if}
          <div class="detail-banner-meta">
            <span><strong>Responsável:</strong> {selectedExp.responsavel || "Não informado"}</span>
            <span><strong>Criado em:</strong> {new Date(selectedExp.criado_em).toLocaleDateString()}</span>
          </div>
        </div>

        {#if expTab === 'estrutura'}
          <!-- Linha do Tempo (Timepoints) -->
          <div class="timepoints-timeline-card">
            <h3>Curva Temporal Definida</h3>
            <div class="timeline-flow">
              {#each selectedExp.timepoints as tp, idx}
                <div class="timeline-node">
                  <span class="node-num">{idx + 1}</span>
                  <span class="node-label">{tp.rotulo}</span>
                </div>
                {#if idx < selectedExp.timepoints.length - 1}
                  <span class="timeline-arrow">→</span>
                {/if}
              {/each}
            </div>
          </div>

          <!-- Formulários de Grupo ou Animal (Exibição Condicional centralizada) -->
          {#if showGroupForm}
            <div class="inline-form-card">
              <h3>{grupoEditandoId !== null ? "Editar Grupo" : "Criar Novo Grupo de Tratamento"}</h3>
              <div class="inline-form-row">
                <div class="form-group flex-2">
                  <label for="grp-nome">Nome do Grupo *</label>
                  <input id="grp-nome" type="text" bind:value={grupoNome} placeholder="Ex: Controle (Veículo) ou Tratado (Fármaco X)" class="form-control" />
                </div>
                <div class="form-group flex-1">
                  <label for="grp-cor">Cor de Identificação *</label>
                  <div class="color-picker-row">
                    <input id="grp-cor" type="color" bind:value={grupoCor} class="color-picker-input" />
                    <input type="text" bind:value={grupoCor} class="form-control color-text-input" />
                  </div>
                </div>
              </div>
              <div class="form-actions">
                <button onclick={() => showGroupForm = false} class="btn-link" type="button">Cancelar</button>
                <button onclick={salvarGrupo} class="btn-primary" disabled={!grupoNome.trim() || carregando}>Salvar Grupo</button>
              </div>
            </div>
          {/if}

          {#if showAnimalForm}
            <div class="inline-form-card">
              <h3>{animalEditandoId !== null ? "Editar Animal" : "Adicionar Animal"}</h3>
              <div class="inline-form-row">
                <div class="form-group flex-1">
                  <label for="ani-grupo">Grupo de Tratamento *</label>
                  <select id="ani-grupo" bind:value={animalGrupoId} class="form-control">
                    {#each selectedExp.grupos as g}
                      <option value={g.id}>{g.nome}</option>
                    {/each}
                  </select>
                </div>
                <div class="form-group flex-1">
                  <label for="ani-mar">Marcação Visual (riscos na cauda) *</label>
                  <input id="ani-mar" type="text" bind:value={animalMarcacao} placeholder="Ex: 4P, 2L, 1P1L" class="form-control" />
                </div>
                <div class="form-group flex-1">
                  <label for="ani-peso">Peso Corporal (g - Opcional)</label>
                  <input id="ani-peso" type="text" bind:value={animalPeso} placeholder="Ex: 25.4" class="form-control" />
                </div>
              </div>
              <div class="form-actions">
                <button onclick={() => showAnimalForm = false} class="btn-link" type="button">Cancelar</button>
                <button onclick={salvarAnimal} class="btn-primary" disabled={!animalMarcacao.trim() || animalGrupoId === null || carregando}>Salvar Animal</button>
              </div>
            </div>
          {/if}

          <!-- LISTAGEM DE GRUPOS E ANIMAIS -->
          <div class="groups-section">
            <h3>Grupos de Tratamento e Animais</h3>
            
            {#if selectedExp.grupos.length === 0}
              <div class="empty-state">
                <p>Nenhum grupo de tratamento cadastrado para este experimento.</p>
                <p>Clique em "+ Novo Grupo" no canto superior para registrar tratamentos.</p>
              </div>
            {:else}
              <div class="groups-grid">
                {#each selectedExp.grupos as g (g.id)}
                  <div class="group-card" style="border-top: 4px solid {g.cor};">
                    <div class="group-card-header">
                      <div class="group-title-row">
                        <div class="color-dot" style="background-color: {g.cor};"></div>
                        <h4>{g.nome}</h4>
                      </div>
                      <div class="group-header-actions">
                        <button onclick={() => iniciarCriacaoAnimal(g.id)} class="btn-icon-add" title="Adicionar Animal" disabled={showGroupForm || showAnimalForm}>+ Animal</button>
                        <button onclick={() => iniciarEdicaoGrupo(g)} class="btn-icon-edit" title="Editar Grupo" disabled={showGroupForm || showAnimalForm}>✎</button>
                        <button onclick={() => excluirGrupo(g.id, g.nome)} class="btn-icon-del" title="Excluir Grupo" disabled={showGroupForm || showAnimalForm}>&times;</button>
                      </div>
                    </div>

                    <div class="group-card-body">
                      {#if g.animais.length === 0}
                        <p class="no-animals-text">Nenhum animal neste grupo.</p>
                      {:else}
                        <table class="animals-table">
                          <thead>
                            <tr>
                              <th>Marcação</th>
                              <th>Peso (g)</th>
                              <th class="actions-col">Ações</th>
                            </tr>
                          </thead>
                          <tbody>
                            {#each g.animais as a (a.id)}
                              <tr>
                                <td class="font-mono"><strong>{a.marcacao}</strong></td>
                                <td>{a.peso !== null ? `${a.peso}g` : "—"}</td>
                                <td class="actions-col">
                                  <button onclick={() => iniciarEdicaoAnimal(a)} class="btn-table-edit" title="Editar" disabled={showGroupForm || showAnimalForm}>✎</button>
                                  <button onclick={() => excluirAnimal(a.id, a.marcacao)} class="btn-table-del" title="Remover" disabled={showGroupForm || showAnimalForm}>&times;</button>
                                </td>
                              </tr>
                            {/each}
                          </tbody>
                        </table>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {:else if expTab === 'matriz'}
          <!-- MEDIÇÕES & MATRIZ DIXON -->
          {#if salvoLimiar !== null}
            <div class="alert alert-success">
              <div style="display: flex; align-items: center; gap: 10px;">
                <span class="success-icon">✓</span>
                <div>
                  <strong>Teste Concluído com Sucesso!</strong>
                  <p style="margin: 4px 0 0 0;">O limiar (PWT) do animal <strong>{salvoAnimal}</strong> ({salvoTimepoint}) foi calculado em: <strong>{salvoLimiar.toFixed(4)} g</strong>.</p>
                </div>
              </div>
              <button onclick={() => salvoLimiar = null} class="btn-close">&times;</button>
            </div>
          {/if}

          {#if showStartTestForm && testandoAnimal && testandoTimepoint}
            <div class="active-test-panel">
              <div class="active-test-header">
                <h2>Iniciar Sequência de Testes (Dixon Up-Down)</h2>
                <button class="btn-close" onclick={() => showStartTestForm = false}>&times;</button>
              </div>
              <div class="form-container" style="box-shadow: none; border: none; padding: 0; background: transparent;">
                <p style="margin: 0 0 16px 0; font-size: 15px;">
                  Preparando teste para o animal <strong>{testandoAnimal.marcacao}</strong> no timepoint <strong>{testandoTimepoint.rotulo}</strong>.
                </p>
                <div class="form-group" style="max-width: 400px;">
                  <label for="fil-inicial-select">Selecione o Filamento Inicial (g) *</label>
                  <select id="fil-inicial-select" bind:value={testandoFilamentoInicial} class="form-control">
                    <option value="" disabled>Escolha um filamento...</option>
                    {#each obterFilamentosDoExperimento() as val}
                      <option value={val.toString()}>{val} g</option>
                    {/each}
                  </select>
                  <p style="margin: 6px 0 0 0; font-size: 13px; opacity: 0.7;">
                    Recomendação: Escolha um filamento próximo ao limiar médio estimado (geralmente o valor do meio do kit).
                  </p>
                </div>
                <div class="form-actions" style="justify-content: flex-start; margin-top: 24px;">
                  <button class="btn-primary" onclick={iniciarSequencia} disabled={!testandoFilamentoInicial || carregando}>
                    Iniciar Teste 🧪
                  </button>
                  <button class="btn-link" onclick={() => showStartTestForm = false} type="button">Cancelar</button>
                </div>
              </div>
            </div>
          {/if}

          {#if showActiveTestScreen && testandoAnimal && testandoTimepoint && testandoUltimaSugestao}
            <div class="active-test-panel">
              <div class="active-test-header">
                <div>
                  <span class="badge-d" style="background-color: var(--accent-bg); color: var(--accent); border: 1px solid var(--accent-border);">TESTE EM ANDAMENTO</span>
                  <h2 style="margin-top: 8px;">Animal {testandoAnimal.marcacao} &mdash; Timepoint: {testandoTimepoint.rotulo}</h2>
                </div>
                <button class="btn-link" onclick={cancelarSequenciaAtiva} style="color: #ef4444; font-weight: bold;">Descartar Teste</button>
              </div>
              
              <div class="filament-sugestion-card">
                <div class="sug-title">Próxima força a aplicar no animal:</div>
                <div class="sug-value">{testandoUltimaSugestao.proximo_filamento} g</div>
                
                {#if testandoUltimaSugestao.aviso}
                  <div class="sug-warning">
                    {testandoUltimaSugestao.aviso}
                  </div>
                {/if}
              </div>
              
              <!-- Respostas Buttons -->
              <div class="test-buttons-row">
                <button class="btn-response-o" onclick={() => registrarResposta('O')} disabled={carregando} title="Sem Resposta">
                  <span style="font-size: 24px; font-weight: 900;">◯</span> Não respondeu (O)
                </button>
                <button class="btn-response-x" onclick={() => registrarResposta('X')} disabled={carregando} title="Com Resposta">
                  <span style="font-size: 24px; font-weight: 900;">✕</span> Respondeu (X)
                </button>
              </div>

              <!-- Dica discreta dos atalhos de teclado -->
              <p class="atalhos-hint">
                Atalhos: <kbd>0</kbd> = Não respondeu · <kbd>1</kbd> = Respondeu · <kbd>Backspace</kbd> = Desfazer
                <span style="opacity: 0.7;">(finalizar exige clique)</span>
              </p>

              <!-- Timeline -->
              <div class="response-timeline">
                <div class="response-timeline-header">
                  <h4>Aplicações e Respostas Gravadas (Salvas a cada clique)</h4>
                  {#if testandoUltimaSugestao.respostas.length > 0}
                    <button class="btn-small-add" onclick={desfazerUltima} style="background-color: var(--code-bg); border-color: var(--border); color: var(--text);" disabled={carregando}>
                      ⟲ Desfazer Último Clique
                    </button>
                  {/if}
                </div>
                
                {#if testandoUltimaSugestao.respostas.length === 0}
                  <p style="margin: 0; font-size: 14px; opacity: 0.6; font-style: italic;">Aguardando primeira aplicação...</p>
                {:else}
                  <div class="response-timeline-flow">
                    {#each testandoUltimaSugestao.respostas as r, idx}
                      <div class="timeline-step {r.resposta === 'X' ? 'step-x' : 'step-o'}">
                        <span style="opacity: 0.5; margin-right: 4px;">#{idx+1}</span>
                        <strong>{r.filamento_g}g</strong>
                        <span style="margin-left: 6px; font-weight: 800;">[{r.resposta}]</span>
                      </div>
                      {#if idx < testandoUltimaSugestao.respostas.length - 1}
                        <span style="opacity: 0.3;">→</span>
                      {/if}
                    {/each}
                  </div>
                {/if}
              </div>
              
              <!-- Dixon Nominal N criteria -->
              <div class="n-nominal-indicator">
                <div class="n-nominal-header">
                  <span>Tamanho N Nominal (Dixon): <strong>{testandoUltimaSugestao.n_nominal} / 6</strong></span>
                  {#if testandoUltimaSugestao.n_nominal >= 6}
                    <span style="color: #166534; font-weight: bold; display: flex; align-items: center; gap: 4px;">
                      ✅ Recomendação atingida!
                    </span>
                  {:else}
                    <span style="opacity: 0.7; font-size: 13px;">Necessário reversão para iniciar contagem nominal.</span>
                  {/if}
                </div>
                <div class="n-nominal-bar-container">
                  <div class="n-nominal-bar" style="width: {Math.min(100, (testandoUltimaSugestao.n_nominal / 6) * 100)}%;"></div>
                </div>
                <p style="margin: 0; font-size: 13px; opacity: 0.75; line-height: 1.4;">
                  * O critério padrão da tabela de Dixon exige uma sequência de 6 respostas válidas após a primeira reversão (reversão é a primeira mudança de resposta O&harr;X).
                  Você pode continuar aplicando respostas adicionais se desejar, mas já pode clicar em Finalizar quando o botão verde for ativado.
                </p>
              </div>
              
              <div class="form-actions" style="border-top: 1px solid var(--border); padding-top: 16px;">
                <button class="btn-primary" onclick={finalizarTeste} disabled={!testandoUltimaSugestao.pode_finalizar || carregando} style="background-color: #22c55e; border-color: #22c55e; box-shadow: 0 2px 4px rgba(34, 197, 94, 0.2);">
                  Finalizar Teste e Calcular Limiar 🔬
                </button>
                <button class="btn-link" onclick={() => { showActiveTestScreen = false; testandoAnimal = null; testandoTimepoint = null; testandoSequencia = null; testandoUltimaSugestao = null; }} type="button">
                  Voltar para Matriz (Manter Ativo)
                </button>
              </div>
            </div>
          {/if}

          {#if !showStartTestForm && !showActiveTestScreen}
            <div style="background: var(--bg); border: 1px solid var(--border); border-radius: 10px; padding: 20px; box-shadow: 0 1px 3px rgba(0,0,0,0.05);">
              <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; flex-wrap: wrap; gap: 8px;">
                <h3 style="margin: 0; font-size: 18px; color: var(--text-h);">Matriz de Limiares (Timepoint &times; Animal)</h3>
                <p style="margin: 0; font-size: 13px; opacity: 0.7;">
                  Clique nas células pontilhadas para realizar os testes ou retomar sequências.
                </p>
              </div>
              
              {#if selectedExp.grupos.length === 0 || selectedExp.grupos.every(g => g.animais.length === 0)}
                <div class="empty-state" style="margin-top: 16px;">
                  <p>Nenhum animal cadastrado neste experimento.</p>
                  <p>Vá para a aba <strong>⚙️ Configurar Grupos & Animais</strong> para adicionar animais primeiro.</p>
                </div>
              {:else}
                <div style="overflow-x: auto; margin-top: 16px;">
                  <table class="matrix-table">
                    <thead>
                      <tr>
                        <th class="align-left">Grupo</th>
                        <th class="align-left">Animal</th>
                        <th>Peso</th>
                        {#each selectedExp.timepoints as tp}
                          <th>{tp.rotulo}</th>
                        {/each}
                      </tr>
                    </thead>
                    <tbody>
                      {#each selectedExp.grupos as g}
                        {#each g.animais as a}
                          <tr>
                            <td class="align-left" style="font-weight: 500;">
                              <div style="display: flex; align-items: center; gap: 8px;">
                                <div class="color-dot" style="background-color: {g.cor};"></div>
                                <span>{g.nome}</span>
                              </div>
                            </td>
                            <td class="align-left font-mono"><strong>{a.marcacao}</strong></td>
                            <td>{a.peso !== null ? `${a.peso}g` : "—"}</td>
                            {#each selectedExp.timepoints as tp}
                              {@const seq = listSequencias.find(s => s.animal_id === a.id && s.timepoint_id === tp.id)}
                              <td>
                                {#if seq}
                                  {#if seq.status === 'concluida'}
                                    <div class="badge-threshold" title="Inicial: {seq.filamento_inicial}g | Cliques: {seq.respostas} | d = {seq.d_usado || '?'}" style="border-left: 4px solid {g.cor};">
                                      {seq.limiar !== null && seq.limiar !== undefined ? `${seq.limiar.toFixed(3)} g` : '—'}
                                    </div>
                                  {:else if seq.status === 'em_andamento'}
                                    <button class="badge-in-progress" onclick={() => abrirSequenciaEmAndamento(a, tp, seq.id)}>
                                      🧪 Retomar ({seq.respostas.length})
                                    </button>
                                  {/if}
                                {:else}
                                  <button class="btn-test-action" onclick={() => iniciarFormularioTeste(a, tp)}>
                                    + Testar
                                  </button>
                                {/if}
                              </td>
                            {/each}
                          </tr>
                        {/each}
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>
            
            <!-- Histórico Detalhado -->
            <div style="background: var(--bg); border: 1px solid var(--border); border-radius: 10px; padding: 20px; box-shadow: 0 1px 3px rgba(0,0,0,0.05); margin-top: 24px;">
              <h3 style="margin: 0 0 16px 0; font-size: 18px; color: var(--text-h);">Histórico Detalhado de Sequências Concluídas</h3>
              
              {#if concluidas.length === 0}
                <p style="margin: 0; font-size: 14px; opacity: 0.6; font-style: italic;">Nenhuma sequência concluída até o momento.</p>
              {:else}
                <div style="overflow-x: auto;">
                  <table class="animals-table" style="width: 100%; font-size: 13px;">
                    <thead>
                      <tr>
                        <th>Animal</th>
                        <th>Grupo</th>
                        <th>Timepoint</th>
                        <th>Filamento Inicial</th>
                        <th>Série de Respostas (Up-Down)</th>
                        <th>Limiar PWT (g)</th>
                        <th>Data do Registro</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each concluidas as c}
                        <tr>
                          <td class="font-mono"><strong>{c.animal_marcacao}</strong></td>
                          <td>
                            <div style="display: flex; align-items: center; gap: 6px;">
                              <div class="color-dot" style="background-color: {c.grupo_cor};"></div>
                              <span>{c.grupo_nome}</span>
                            </div>
                          </td>
                          <td><span class="badge-d" style="background-color: var(--code-bg); border-color: var(--border); color: var(--text); padding: 2px 6px;">{c.timepoint_rotulo}</span></td>
                          <td class="font-mono">{c.filamento_inicial} g</td>
                          <td class="font-mono" style="letter-spacing: 1px; font-weight: bold; color: var(--accent);">{c.respostas}</td>
                          <td class="font-mono" style="font-weight: bold; color: #166534;">{c.limiar !== null && c.limiar !== undefined ? `${c.limiar.toFixed(4)} g` : '—'}</td>
                          <td>{new Date(c.criado_em).toLocaleString()}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>
          {/if}
        {:else if expTab === 'grafico'}
           <!-- GRÁFICOS E EXPORTAÇÕES -->
           <div class="graficos-exportacao-section" style="margin-top: 24px;">
             <h3>Análise de Curva Temporal e Estatísticas</h3>
             
             {#if expStats.length === 0}
               <div class="empty-state" style="margin-top: 16px;">
                 <p>Nenhuma sequência concluída para gerar estatísticas ou gráfico.</p>
                 <p>Execute e conclua os testes de Dixon na aba <strong>🧪 Executar Medições (Dixon)</strong> primeiro.</p>
               </div>
             {:else}
               <div style="margin-bottom: 24px;">
                 <p style="font-size: 14px; opacity: 0.8; margin-bottom: 16px;">
                   O gráfico abaixo exibe a <strong>Média Geométrica</strong> (calculada no espaço logarítmico $\log_{10}$) e o <strong>Erro Padrão</strong> assimétrico em escala linear, conforme as diretrizes nociceptivas do método Up-Down de Dixon.
                 </p>
                 
                 <!-- Componente Gráfico SVG -->
                 <GraficoCurva stats={expStats} timepoints={selectedExp.timepoints} />
               </div>

               <!-- Botões de Exportação -->
               <div class="export-actions-row" style="display: flex; gap: 12px; margin-top: 24px; margin-bottom: 24px; flex-wrap: wrap;">
                 <button class="btn-primary" onclick={exportarCSV} disabled={carregando}>
                   📄 Exportar CSV
                 </button>
                 <button class="btn-primary" onclick={exportarXLSX} disabled={carregando} style="background-color: #166534; border-color: #166534;">
                   🟢 Exportar XLSX (Excel)
                 </button>
                 <button class="btn-primary" onclick={exportarPDF} disabled={carregando} style="background-color: #c2410c; border-color: #c2410c;">
                   🔴 Exportar Relatório PDF
                 </button>
                 <button class="btn-primary" onclick={copiarParaPrism} disabled={carregando} style="background-color: {prismCopiado ? '#15803d' : '#6d28d9'}; border-color: {prismCopiado ? '#15803d' : '#6d28d9'};">
                   {prismCopiado ? '✓ Copiado!' : '📋 Copiar para o Prism'}
                 </button>
               </div>

               <!-- Pré-visualização da tabela do Prism (formato "Grouped") -->
               {#if prismPreview}
                 <div class="prism-preview" style="margin-top: 8px; margin-bottom: 24px;">
                   <p style="font-size: 13px; color: #555; margin-bottom: 8px;">
                     Formato Prism <strong>"Grouped"</strong>: colunas = animais (réplicas), linhas = timepoints.
                     Já copiado como texto (TAB). Cole direto numa tabela Grouped do Prism. Números com ponto decimal;
                     células vazias = animal sem teste naquele timepoint.
                   </p>
                   <div style="overflow-x: auto;">
                     <table class="animals-table" style="font-size: 12px; white-space: nowrap;">
                       <thead>
                         <tr>
                           <th>Timepoint</th>
                           {#each prismPreview.colunasAnimais as col}
                             <th>{col}</th>
                           {/each}
                         </tr>
                       </thead>
                       <tbody>
                         {#each prismPreview.linhas as linha}
                           <tr>
                             <td><strong>{linha.timepoint}</strong></td>
                             {#each linha.valores as v}
                               <td style="text-align: right;">{v === '' ? '—' : v}</td>
                             {/each}
                           </tr>
                         {/each}
                       </tbody>
                     </table>
                   </div>
                 </div>
               {/if}

               <!-- Tabela Estatística Resumida -->
               <div style="margin-top: 24px;">
                 <h4>Tabela Resumida por Grupo</h4>
                 <div style="overflow-x: auto; margin-top: 12px;">
                   <table class="animals-table" style="width: 100%; font-size: 13px;">
                     <thead>
                       <tr>
                         <th>Grupo</th>
                         <th>Timepoint</th>
                         <th>N (Animais)</th>
                         <th>Média Geométrica (g)</th>
                         <th>Limite Inferior EP (g)</th>
                         <th>Limite Superior EP (g)</th>
                       </tr>
                     </thead>
                     <tbody>
                       {#each expStats as s}
                         <tr>
                           <td>
                             <div style="display: flex; align-items: center; gap: 8px;">
                               <div class="color-dot" style="background-color: {s.grupo_cor};"></div>
                               <span><strong>{s.grupo_nome}</strong></span>
                             </div>
                           </td>
                           <td><span class="badge-d" style="background-color: var(--code-bg); border-color: var(--border); color: var(--text); padding: 2px 6px;">{s.timepoint_rotulo}</span></td>
                           <td>{s.n}</td>
                           <td class="font-mono" style="font-weight: bold;">{s.media_geometrica_g.toFixed(4)} g</td>
                           <td class="font-mono" style="color: #c2410c;">{s.limite_inferior_g !== null && s.limite_inferior_g !== undefined ? `${s.limite_inferior_g.toFixed(4)} g` : '—'}</td>
                           <td class="font-mono" style="color: #166534;">{s.limite_superior_g !== null && s.limite_superior_g !== undefined ? `${s.limite_superior_g.toFixed(4)} g` : '—'}</td>
                         </tr>
                       {/each}
                     </tbody>
                   </table>
                 </div>
               </div>
             {/if}
           </div>
        {/if}
      </div>
    {/if}
  {/if}
</main>

<style>
  /* Matriz e Badges */
  .matrix-table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 16px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }
  .matrix-table th, .matrix-table td {
    padding: 12px;
    border-bottom: 1px solid var(--border);
    text-align: center;
    font-size: 14px;
  }
  .matrix-table th {
    background-color: var(--code-bg);
    font-weight: 600;
    color: var(--text-h);
  }
  .matrix-table td.align-left {
    text-align: left;
  }
  
  .badge-threshold {
    background-color: rgba(34, 197, 94, 0.1);
    color: #166534;
    border: 1px solid rgba(34, 197, 94, 0.25);
    padding: 6px 12px;
    border-radius: 6px;
    font-family: var(--mono);
    font-weight: bold;
    font-size: 13px;
    cursor: help;
    display: inline-block;
  }
  
  .badge-in-progress {
    background-color: rgba(170, 59, 255, 0.1);
    color: var(--accent);
    border: 1px solid var(--accent-border);
    padding: 6px 12px;
    border-radius: 6px;
    font-weight: bold;
    font-size: 13px;
    cursor: pointer;
    animation: matrixPulse 2s infinite;
    display: inline-block;
  }

  @keyframes matrixPulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.8; transform: scale(0.98); }
  }
  
  .btn-test-action {
    background-color: var(--bg);
    border: 1px dashed var(--accent);
    color: var(--accent);
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }
  
  .btn-test-action:hover {
    background-color: var(--accent-bg);
  }
  
  /* Painel de Teste Ativo */
  .active-test-panel {
    background: var(--bg);
    border: 1.5px solid var(--accent-border);
    border-radius: 12px;
    padding: 24px;
    box-shadow: var(--shadow);
    margin-bottom: 24px;
  }
  
  .active-test-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border);
    padding-bottom: 16px;
    margin-bottom: 20px;
  }
  
  .active-test-header h2 {
    margin: 0;
    font-size: 20px;
    color: var(--text-h);
  }
  
  .filament-sugestion-card {
    background-color: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 24px;
    text-align: center;
    margin-bottom: 24px;
  }
  
  .sug-title {
    font-size: 14px;
    opacity: 0.8;
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  
  .sug-value {
    font-size: 48px;
    font-weight: 800;
    font-family: var(--mono);
    color: var(--accent);
    line-height: 1;
    margin-bottom: 8px;
  }
  
  .sug-warning {
    color: #dc2626;
    background-color: #fee2e2;
    border: 1px solid #fecaca;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 13px;
    display: inline-block;
    margin-top: 12px;
    font-weight: 500;
  }
  
  .test-buttons-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
    margin-bottom: 24px;
  }
  
  /* Acessibilidade de cor e peso visual para daltonismo */
  .btn-response-o {
    background-color: transparent;
    border: 3px solid #6b7280;
    color: #1f2937;
    font-size: 18px;
    font-weight: 700;
    padding: 20px;
    border-radius: 12px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    transition: all 0.2s;
  }
  
  .btn-response-o:hover {
    background-color: rgba(107, 114, 128, 0.08);
    border-color: #374151;
  }
  
  .btn-response-x {
    background-color: var(--accent);
    border: 3px solid var(--accent);
    color: white;
    font-size: 18px;
    font-weight: 800;
    padding: 20px;
    border-radius: 12px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    transition: all 0.2s;
    box-shadow: 0 4px 6px rgba(170, 59, 255, 0.25);
  }
  
  .btn-response-x:hover {
    opacity: 0.95;
    box-shadow: 0 6px 8px rgba(170, 59, 255, 0.35);
  }
  
  .response-timeline {
    border: 1px solid var(--border);
    background-color: var(--code-bg);
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 24px;
  }
  
  .response-timeline-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
    flex-wrap: wrap;
    gap: 8px;
  }
  
  .response-timeline-header h4 {
    margin: 0;
    font-size: 14px;
    color: var(--text-h);
  }
  
  .response-timeline-flow {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  
  .timeline-step {
    display: flex;
    align-items: center;
    background: var(--bg);
    border: 1px solid var(--border);
    padding: 4px 10px;
    border-radius: 6px;
    font-family: var(--mono);
    font-size: 13px;
  }
  
  .step-o {
    border-left: 4px solid #6b7280;
  }
  
  .step-x {
    border-left: 4px solid var(--accent);
    font-weight: bold;
  }
  
  .n-nominal-indicator {
    background-color: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 24px;
    font-size: 14px;
  }
  
  .n-nominal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }
  
  .n-nominal-bar-container {
    background-color: var(--border);
    height: 10px;
    border-radius: 5px;
    overflow: hidden;
    margin-bottom: 12px;
  }
  
  .n-nominal-bar {
    background-color: #22c55e;
    height: 100%;
    transition: width 0.3s;
  }

  :global(body) {
    background-color: var(--bg);
    color: var(--text);
    margin: 0;
    padding: 0;
  }

  .app-container {
    padding: 24px;
    max-width: 1200px;
    margin: 0 auto;
    text-align: left;
    min-height: auto;
    border: none;
  }

  /* Abas Globais */
  .app-tabs {
    display: flex;
    gap: 8px;
    border-bottom: 2px solid var(--border);
    margin-bottom: 24px;
  }

  .app-tabs button {
    background: transparent;
    border: none;
    padding: 12px 20px;
    font-size: 16px;
    font-weight: 500;
    cursor: pointer;
    color: var(--text);
    border-bottom: 3px solid transparent;
    transition: all 0.2s;
  }

  .app-tabs button:hover {
    color: var(--text-h);
  }

  .app-tabs .tab-active {
    color: var(--accent);
    border-bottom-color: var(--accent);
    font-weight: 600;
  }

  .app-header {
    margin-bottom: 24px;
    padding-bottom: 4px;
  }

  .app-header h1 {
    font-size: 28px;
    margin: 0 0 4px 0;
    font-weight: 700;
  }

  .subtitle {
    margin: 0;
    font-size: 14px;
    color: var(--text);
    opacity: 0.8;
  }

  /* Alertas */
  .alert {
    padding: 16px;
    border-radius: 8px;
    margin-bottom: 24px;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    font-size: 15px;
  }

  .alert-error {
    background-color: #fee2e2;
    border: 1px solid #fecaca;
    color: #991b1b;
  }

  .alert-success {
    background-color: #dcfce7;
    border: 1px solid #bbf7d0;
    color: #166534;
  }

  .success-icon {
    background: #22c55e;
    color: white;
    border-radius: 50%;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    flex-shrink: 0;
    font-size: 14px;
  }

  .d-val-highlight {
    margin-top: 8px;
    font-size: 16px;
  }
  .d-val-highlight code {
    background-color: rgba(22, 101, 52, 0.1);
    color: #166534;
  }

  .btn-close {
    background: transparent;
    border: none;
    font-size: 20px;
    font-weight: bold;
    cursor: pointer;
    color: inherit;
    padding: 0 4px;
    line-height: 1;
  }

  /* Grid Layout */
  .layout-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 28px;
  }

  @media (max-width: 868px) {
    .layout-grid {
      grid-template-columns: 1fr;
    }
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  .section-header h2 {
    margin: 0;
    font-size: 20px;
  }

  /* List e Cards */
  .cards-grid {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .card {
    background-color: var(--bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    transition: box-shadow 0.2s, border-color 0.2s;
  }

  .card:hover {
    box-shadow: var(--shadow);
    border-color: var(--accent-border);
  }

  .card-editing {
    border: 2px solid var(--accent);
    box-shadow: var(--accent-bg) 0 0 10px;
  }

  .card-body {
    padding: 16px;
  }

  .card-title-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 8px;
  }

  .card-title-row h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--text-h);
  }

  .badge-d {
    background-color: var(--accent-bg);
    color: var(--accent);
    font-family: var(--sans);
    font-size: 13px;
    font-weight: bold;
    padding: 4px 10px;
    border-radius: 20px;
    white-space: nowrap;
    border: 1px solid var(--accent-border);
  }

  .card-description {
    font-size: 14px;
    opacity: 0.8;
    margin: 0 0 12px 0;
    line-height: 1.4;
  }

  .filaments-preview {
    font-size: 13px;
  }

  .filaments-preview strong {
    display: block;
    margin-bottom: 6px;
  }

  .tags-container {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .tag-val {
    background-color: var(--code-bg);
    color: var(--text-h);
    padding: 2px 8px;
    border-radius: 4px;
    font-family: var(--mono);
    border: 1px solid var(--border);
  }

  .card-actions {
    display: flex;
    border-top: 1px solid var(--border);
    background-color: var(--code-bg);
    border-bottom-left-radius: 9px;
    border-bottom-right-radius: 9px;
  }

  .card-actions button {
    flex: 1;
    background: transparent;
    border: none;
    padding: 10px;
    font-size: 14px;
    cursor: pointer;
    transition: background-color 0.2s;
    color: var(--text);
  }

  .card-actions button:not(:last-child) {
    border-right: 1px solid var(--border);
  }

  .card-actions button:hover {
    background-color: rgba(0, 0, 0, 0.05);
    color: var(--text-h);
  }

  .card-actions .btn-danger:hover {
    background-color: #fee2e2;
    color: #ef4444;
  }

  /* Form */
  .form-container {
    background-color: var(--bg);
    border: 1px solid var(--border);
    padding: 20px;
    border-radius: 10px;
    box-shadow: var(--shadow);
  }

  .form-container h2 {
    margin-top: 0;
    margin-bottom: 20px;
    font-size: 20px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 8px;
    color: var(--text-h);
  }

  .form-group {
    margin-bottom: 16px;
  }

  .form-group label {
    display: block;
    margin-bottom: 6px;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-h);
  }

  .form-control {
    width: 100%;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background-color: var(--bg);
    color: var(--text-h);
    font-size: 15px;
    box-sizing: border-box;
  }

  .form-control:focus {
    border-color: var(--accent);
    outline: none;
  }

  .filaments-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .btn-small-add {
    background-color: var(--accent-bg);
    color: var(--accent);
    border: 1px solid var(--accent-border);
    padding: 4px 10px;
    font-size: 12px;
    font-weight: bold;
    border-radius: 4px;
    cursor: pointer;
  }

  .btn-small-add:hover {
    background-color: var(--accent);
    color: white;
  }

  .filaments-inputs-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
    gap: 8px;
    max-height: 220px;
    overflow-y: auto;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background-color: var(--code-bg);
  }

  .timepoints-inputs-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 200px;
    overflow-y: auto;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background-color: var(--code-bg);
  }

  .filament-input-row,
  .timepoint-input-row {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--bg);
    padding: 4px 6px;
    border-radius: 4px;
    border: 1px solid var(--border);
  }

  .row-num {
    font-size: 11px;
    font-family: var(--mono);
    opacity: 0.6;
    min-width: 16px;
  }

  .form-control-small {
    width: 100%;
    border: none;
    padding: 4px 2px;
    font-family: var(--sans);
    font-size: 14px;
    color: var(--text-h);
    background: transparent;
  }

  .form-control-small:focus {
    outline: none;
  }

  .unit-g {
    font-size: 13px;
    opacity: 0.7;
    margin-right: 4px;
  }

  .btn-remove {
    background: transparent;
    border: none;
    color: #ef4444;
    cursor: pointer;
    font-size: 18px;
    padding: 0 4px;
    line-height: 1;
    font-weight: bold;
  }

  .btn-remove:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .calc-feedback-box {
    margin-top: 20px;
    padding: 12px;
    border-radius: 6px;
    font-size: 14px;
  }

  .feedback-ok {
    background-color: rgba(170, 59, 255, 0.05);
    border: 1px solid var(--accent-border);
    color: var(--text-h);
  }

  .feedback-error {
    background-color: #fff5f5;
    border: 1px solid #feb2b2;
    color: #c53030;
  }

  .calc-success {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .calc-label {
    font-weight: 500;
  }

  .calc-value {
    font-family: var(--mono);
    font-weight: bold;
    font-size: 16px;
    color: var(--accent);
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 16px;
    margin-top: 20px;
    align-items: center;
  }

  .btn-primary {
    background-color: var(--accent);
    color: white;
    border: none;
    padding: 10px 20px;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
    font-size: 14px;
    box-shadow: 0 2px 4px rgba(170, 59, 255, 0.2);
    transition: opacity 0.2s;
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-link {
    background: transparent;
    border: none;
    color: var(--text);
    cursor: pointer;
    font-size: 14px;
  }

  .btn-link:hover {
    text-decoration: underline;
    color: var(--text-h);
  }

  /* Help box e estados vazios */
  .section-help {
    height: 100%;
  }

  .help-box {
    background-color: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 24px;
    height: calc(100% - 48px);
  }

  .help-box h3 {
    margin-top: 0;
    font-size: 18px;
    color: var(--text-h);
  }

  .help-box p {
    font-size: 14px;
    line-height: 1.5;
    margin-bottom: 12px;
  }

  .help-box ul {
    font-size: 14px;
    line-height: 1.6;
    padding-left: 20px;
  }

  .math-expr {
    background: var(--bg);
    border: 1px solid var(--border);
    padding: 12px;
    border-radius: 6px;
    font-family: var(--mono);
    font-size: 14px;
    text-align: center;
    margin: 16px 0;
    color: var(--text-h);
  }

  .empty-state {
    text-align: center;
    padding: 40px 20px;
    border: 2px dashed var(--border);
    border-radius: 10px;
    color: var(--text);
    font-size: 14px;
  }

  .empty-state p:first-child {
    font-weight: 500;
    color: var(--text-h);
    margin-bottom: 8px;
  }

  .select-kit-warning {
    color: #ef4444;
    font-size: 13px;
    margin: 4px 0 0;
  }

  /* Summary details card */
  .exp-summary-details {
    font-size: 13px;
    line-height: 1.6;
    background-color: var(--code-bg);
    padding: 10px;
    border-radius: 6px;
    margin-top: 8px;
    border: 1px solid var(--border);
  }
  .exp-summary-details p {
    margin: 0;
  }

  /* ESTRUTURA EXPANDIDA DO EXPERIMENTO */
  .expanded-exp-view {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .detail-header-nav {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .btn-back {
    background: transparent;
    border: none;
    font-size: 15px;
    color: var(--accent);
    cursor: pointer;
    font-weight: bold;
    padding: 4px 0;
  }
  .btn-back:hover {
    text-decoration: underline;
  }

  .detail-banner {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 20px;
  }

  .detail-banner-title {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 8px;
  }

  .detail-banner-title h2 {
    margin: 0;
    font-size: 24px;
    color: var(--text-h);
  }

  .detail-banner-desc {
    font-size: 14px;
    opacity: 0.8;
    margin: 0 0 12px 0;
    line-height: 1.5;
  }

  .detail-banner-meta {
    font-size: 13px;
    display: flex;
    gap: 16px;
    border-top: 1px solid var(--border);
    padding-top: 8px;
    opacity: 0.7;
  }

  /* Timeline */
  .timepoints-timeline-card {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 16px 20px;
  }
  .timepoints-timeline-card h3 {
    margin: 0 0 16px 0;
    font-size: 16px;
    color: var(--text-h);
  }

  .timeline-flow {
    display: flex;
    align-items: center;
    gap: 12px;
    overflow-x: auto;
    padding-bottom: 8px;
  }

  .timeline-node {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--code-bg);
    border: 1px solid var(--border);
    padding: 6px 12px;
    border-radius: 20px;
    white-space: nowrap;
  }

  .node-num {
    background: var(--accent);
    color: white;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-family: var(--mono);
    font-weight: bold;
  }

  .node-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-h);
  }

  .timeline-arrow {
    color: var(--border);
    font-weight: bold;
    user-select: none;
  }

  /* Inline Forms */
  .inline-form-card {
    background: var(--bg);
    border: 2px solid var(--accent);
    border-radius: 10px;
    padding: 16px 20px;
    box-shadow: var(--shadow);
  }
  .inline-form-card h3 {
    margin: 0 0 16px 0;
    font-size: 16px;
    color: var(--accent);
  }
  .inline-form-row {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }
  .flex-1 {
    flex: 1;
    min-width: 180px;
  }
  .flex-2 {
    flex: 2;
    min-width: 280px;
  }
  .color-picker-row {
    display: flex;
    gap: 8px;
  }
  .color-picker-input {
    width: 42px;
    height: 42px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    background: transparent;
  }
  .color-text-input {
    font-family: var(--mono);
    text-transform: uppercase;
  }

  /* Seção de grupos e animais */
  .groups-section {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .groups-section h3 {
    margin: 8px 0 0 0;
    font-size: 18px;
    color: var(--text-h);
  }

  .groups-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 20px;
  }

  .group-card {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
    display: flex;
    flex-direction: column;
  }

  .group-card-header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--code-bg);
  }

  .group-title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .color-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: inset 0 0 2px rgba(0,0,0,0.2);
  }

  .group-card-header h4 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-h);
  }

  .group-header-actions {
    display: flex;
    gap: 4px;
  }

  .btn-icon-add, .btn-icon-edit, .btn-icon-del {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 13px;
    padding: 2px 6px;
    border-radius: 4px;
    transition: all 0.2s;
  }

  .btn-icon-add {
    color: var(--accent);
    background-color: var(--accent-bg);
    font-weight: bold;
  }
  .btn-icon-add:hover {
    background-color: var(--accent);
    color: white;
  }
  .btn-icon-edit {
    color: var(--text);
  }
  .btn-icon-edit:hover {
    background-color: rgba(0,0,0,0.05);
    color: var(--text-h);
  }
  .btn-icon-del {
    color: #ef4444;
  }
  .btn-icon-del:hover {
    background-color: #fee2e2;
  }

  .group-card-body {
    padding: 12px 16px;
    flex-grow: 1;
    display: flex;
    flex-direction: column;
  }

  .no-animals-text {
    font-size: 13px;
    font-style: italic;
    opacity: 0.6;
    margin: 12px 0;
    text-align: center;
  }

  .animals-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    text-align: left;
  }

  .animals-table th, .animals-table td {
    padding: 8px 6px;
    border-bottom: 1px solid var(--border);
  }

  .animals-table th {
    font-weight: 500;
    opacity: 0.7;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .font-mono {
    font-family: var(--mono);
  }

  .actions-col {
    text-align: right;
    width: 60px;
    white-space: nowrap;
  }

  .btn-table-edit, .btn-table-del {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 12px;
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--text);
  }

  .btn-table-edit:hover {
    background-color: rgba(0, 0, 0, 0.05);
    color: var(--text-h);
  }

  .btn-table-del:hover {
    background-color: #fee2e2;
    color: #ef4444;
  }

  .form-label-text {
    display: block;
    margin-bottom: 6px;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-h);
  }

  /* ---- Wizard de criação de experimento ---- */
  .wizard-steps {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 20px;
    font-size: 13px;
  }
  .wizard-step {
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid var(--border);
    opacity: 0.6;
  }
  .wizard-step.ativo {
    opacity: 1;
    font-weight: 700;
    background-color: var(--accent-bg);
    color: var(--accent);
    border-color: var(--accent-border);
  }
  .wizard-step.feito { opacity: 0.85; }
  .wizard-step-sep { opacity: 0.5; }

  .wizard-resumo {
    font-size: 13px;
    padding: 10px 12px;
    margin-bottom: 16px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background-color: var(--code-bg);
  }
  .wizard-dica {
    font-size: 12px;
    opacity: 0.75;
    margin: 4px 0 12px;
  }

  .wizard-grupo-card {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 12px;
    background-color: var(--bg);
  }
  .wizard-grupo-head {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .wizard-cor {
    width: 42px;
    height: 32px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: none;
    cursor: pointer;
  }
  .wizard-contador {
    font-size: 12px;
    opacity: 0.7;
    white-space: nowrap;
  }
  .wizard-animais-lista {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 10px 0 4px;
  }
  .wizard-animal-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    padding: 3px 8px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background-color: var(--code-bg);
  }
  .wizard-animal-chip button {
    border: none;
    background: none;
    cursor: pointer;
    color: #ef4444;
    font-size: 14px;
    line-height: 1;
    padding: 0;
  }
  .wizard-add-animal {
    display: flex;
    gap: 8px;
    margin-top: 10px;
    flex-wrap: wrap;
  }

  /* ---- Dica de atalhos de teclado na tela de teste ---- */
  .atalhos-hint {
    margin: 10px 0 0;
    font-size: 12px;
    opacity: 0.75;
    text-align: center;
  }
  .atalhos-hint kbd {
    font-family: inherit;
    font-size: 11px;
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 4px;
    background-color: var(--code-bg);
  }
</style>
