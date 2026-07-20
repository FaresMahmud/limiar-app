# Arquitetura técnica

> Decisões de engenharia e o raciocínio por trás delas. Complementa
> [RESTRICOES.md](RESTRICOES.md) (o que é obrigatório) explicando o **como** e o
> **porquê**.

---

## 1. Tauri em vez de Electron

**Decisão:** o shell desktop é **Tauri 2**, não Electron.

**Motivo (não-negociável):** o app roda em **notebooks fracos e variados** de
estudantes e pesquisadores, cujas specs não conhecemos. Precisa ser **leve em RAM
e disco**.

| Critério | Electron | **Tauri (escolhido)** |
|----------|----------|-----------------------|
| Runtime web | Empacota o Chromium inteiro | Usa o **webview nativo do SO** (WebView2 no Windows, WKWebView no macOS) |
| Tamanho do instalador | ~85–150 MB típico | **~3–10 MB** típico |
| RAM em repouso | Centenas de MB | Bem menor (sem processo Chromium dedicado) |
| Backend | Node.js | **Rust** (nativo, sem GC, binário pequeno) |

**Trade-off aceito:** o webview do SO pode ter pequenas diferenças de renderização
entre Windows e macOS (motores diferentes). Mitigação: manter a UI simples e
testar em ambos (ver pendência de macOS).

---

## 2. Frontend: Svelte + TypeScript + Vite (não SvelteKit)

**Decisão:** **Svelte 5 puro + Vite**, TypeScript, **sem SvelteKit**.

**Motivo:**
- Svelte compila para JS mínimo, sem runtime de framework pesado → coerente com a
  meta de leveza.
- **SvelteKit** traz roteamento baseado em arquivos, SSR/SSG e um servidor de
  aplicação que **este app desktop não precisa** (é uma SPA local single-window).
  Evitá-lo reduz complexidade e superfície de manutenção — importante para o
  projeto ser retomável por outra IA/pessoa.
- A CLI oficial `create-tauri-app --template svelte-ts` hoje gera SvelteKit;
  por isso o scaffold foi feito com `create-vite --template svelte-ts` + `tauri
  init`, resultando em Svelte + Vite puro.

**Integração com Tauri:** `vite.config.ts` fixa a porta **1420** (`strictPort`)
e ignora `src-tauri/**` no watch; o `tauri.conf.json` aponta `devUrl` para
`http://localhost:1420` e `frontendDist` para `../dist`.

---

## 3. Banco de dados: SQLite local via `rusqlite` (no backend Rust)

**Decisão:** persistência em **SQLite**, arquivo local, acessado **exclusivamente
pelo backend Rust** usando **`rusqlite`** (feature `bundled` — o SQLite é embutido,
sem dependência de sistema). O frontend **nunca** fala com o banco diretamente: ele
chama comandos Tauri (`invoke`) e o Rust executa o SQL.

> ⚠️ **Não usamos `tauri-plugin-sql`.** Ele chegou a ser configurado, mas o frontend
> nunca chamou `Database.load`, então suas migrations nunca rodavam — e apontavam
> para um **caminho de banco diferente** do que os comandos Rust usam. Isso criava
> **dois bancos/dois mecanismos de migração divergentes** e foi a causa da confusão
> do bug `no such table`. O plugin foi **removido** (também reduz o build — leveza).

**Motivo:**
- **100% offline**, sem servidor, sem internet — requisito não-negociável.
- Um **único arquivo** de banco por máquina, fácil de localizar/backup.
- Uma **única fonte de verdade** para acesso e migrações (backend Rust).

**Onde fica o arquivo:** `<app_data_dir>/limiar.db` — resolvido em runtime por
`app.path().app_data_dir()` (no Windows, `%APPDATA%/com.limiar.vonfrey/`). Tanto as
migrações (no `setup`) quanto todos os comandos (`obter_conexao`) abrem **este mesmo
arquivo**. Ver [MODELO_DE_DADOS.md](MODELO_DE_DADOS.md).

**Migrations:** função `executar_migracoes` em [`lib.rs`](../src-tauri/src/lib.rs),
chamada no `setup` do app (antes de qualquer comando). Usa `PRAGMA user_version`
para aplicar, em ordem, os arquivos `src-tauri/migrations/00N_*.sql` que ainda não
foram aplicados, e loga as tabelas resultantes. Para adicionar uma migração:
crie `004_*.sql` e adicione um bloco `if version < 4 { ... user_version = 4 }`.
Nunca editar o banco à mão em produção.

**Permissões:** o acesso ao banco é do backend Rust, então **não requer permissão
de capability** para SQL. As capabilities em `src-tauri/capabilities/default.json`
cobrem apenas `core`, `dialog` e `fs` (usados na exportação de arquivos).

---

## 4. Estrutura de pastas e responsabilidades

```
src/            → FRONTEND (Svelte + TS). UI em pt-BR.
  lib/          → módulos reutilizáveis. O MOTOR DE CÁLCULO (Dixon) vive aqui,
                  como TS puro e testável, SEM depender de componentes de UI.
src-tauri/      → BACKEND (Rust) + config Tauri.
  src/lib.rs    → registro de plugins (sql, log) e comandos Tauri (#[tauri::command]).
  capabilities/ → permissões concedidas ao frontend.
  tauri.conf.json → janela, bundle/instalador, identifier, comandos de build.
```

**Regra de camadas:** lógica científica crítica (tabela de Dixon, cálculo do
limiar, cálculo de `d`) deve ser **isolável e testável sem UI e sem banco**. Pode
ficar em TS (`src/lib/`) ou em Rust (`src-tauri/src/`) — a decisão de onde será
tomada na etapa 1 e registrada aqui. Preferência inicial: **TS em `src/lib/`**,
pela facilidade de testar e iterar; migrar para Rust só se houver ganho real.

---

## 5. Fluxo de dados frontend ↔ backend

**Uma única via:** o frontend chama **comandos Tauri (`invoke`)** e o backend Rust
faz tudo — inclusive o acesso ao banco. Não há acesso ao SQLite pelo frontend.

- O frontend usa o wrapper `invokeCommand` em [`src/lib/tauri.ts`](../src/lib/tauri.ts),
  que chama `invoke(cmd, args)` quando rodando no Tauri, ou cai num **adaptador
  mock em memória** quando rodando no navegador puro (`npm run dev`, sem Tauri) —
  útil para desenvolver a UI sem o backend.
- Os comandos Rust (`#[tauri::command]` em `filamentos.rs`, `experimentos.rs`,
  `sequencias.rs`) abrem conexões `rusqlite` no mesmo `limiar.db` via `obter_conexao`.

```
┌─────────────── Frontend (Svelte/TS, webview) ───────────────┐
│  UI (pt-BR)  →  src/lib/tauri.ts (invokeCommand)            │
│                         │  (ou mock em memória no browser)  │
└─────────────────────────┼────────────────────────────────────┘
                          ▼  invoke()
┌──── Backend Rust (src-tauri) ──┐     ┌──── SQLite (arquivo local) ────┐
│ #[tauri::command] handlers     │     │ <app_data_dir>/limiar.db       │
│  filamentos/experimentos/...   │────▶│ (via rusqlite, feature bundled)│
│ motor Dixon (dixon.rs)         │     │ schema por executar_migracoes  │
└────────────────────────────────┘     └────────────────────────────────┘
```

**Diretriz:** manter o motor de cálculo **puro** (entra sequência O/X + filamentos,
sai limiar) e deixar UI e persistência nas bordas. Isso facilita testes e a
retomada do projeto.

---

## 6. Build e distribuição

- **Dev:** `npm run tauri dev` sobe o Vite (1420) e o app Tauri com hot reload.
- **Produção:** `npm run tauri build` gera o instalador do SO atual em
  `src-tauri/target/release/bundle/`.
- **Cross-platform:** `tauri build` **não faz cross-compile de macOS a partir do
  Windows**. O `.dmg` precisa ser gerado num Mac. Ver pendência em
  [RESTRICOES.md](RESTRICOES.md) e no [ROADMAP.md](ROADMAP.md) (etapa 8).

---

## 7. Pendências arquiteturais em aberto

- Definir se o motor de Dixon fica em TS ou Rust: **Resolvido**. Implementado em Rust (com testes isolados) e exposto via comandos Tauri.
- Definir nome/caminho do arquivo SQLite e estratégia de migrations: **Resolvido**. Banco único em `<app_data_dir>/limiar.db`, acessado por `rusqlite` no backend; migrations por `executar_migracoes` (PRAGMA user_version). O `tauri-plugin-sql` foi removido (ver §3).
- Confirmar com o laboratório se `d` é média calculada ou passo fixo do kit: **Resolvido**. Implementado como média das diferenças do log10 das forças.

---

## 8. Abordagem de Gráficos e Exportações (Etapa 5)

Para atender à meta de leveza (sem Electron) e à política offline, desenhamos a seguinte stack de geração visual e de relatórios:

### 8.1 Renderização de Gráfico (Svelte + SVG Puro)
- **Decisão**: Desenhar a curva temporal usando um componente customizado escrito em **Svelte/SVG puro**, sem bibliotecas de terceiros (como Chart.js ou D3).
- **Motivo**: Reduz o tamanho final do bundle e o consumo de RAM em repouso, utilizando os Runes do Svelte 5 para reagir e recalcular coordenadas SVG instantaneamente.
- **Exportação da Imagem**: O gráfico expõe botões para baixar o gráfico em SVG (serializando a tag XML) ou em PNG (carregando o SVG em um elemento Canvas e gerando a imagem em alta definição).

### 8.2 Geração e Gravação de Arquivos (Tauri + Navegador Híbridos)
- **Gravação Nativa (Desktop)**: Se o aplicativo rodar no shell Tauri, utilizamos os plugins `@tauri-apps/plugin-dialog` e `@tauri-apps/plugin-fs` para abrir a caixa de diálogo nativa de salvamento ("Salvar Como") e escrever o arquivo binário/texto direto no caminho selecionado pelo usuário.
- **Gravação Web (Fallback)**: Se o aplicativo rodar no navegador (modo mockado para desenvolvimento), ele faz o download via blobs do navegador.

### 8.3 Biblioteca de Relatórios e Excel (JS Leve e Portável)
- **XLSX**: Escolhemos a biblioteca **SheetJS (`xlsx`)** instalada via npm, que é executada inteiramente na thread JS e cria planilhas Excel reais com múltiplas abas formatadas de forma leve, sem exigir dependências compiladas nativas que poderiam disparar o bloqueio do SAC.
- **PDF**: Escolhemos a biblioteca **jsPDF** para desenhar o relatório estruturado em PDF, embutindo o gráfico nociceptivo (convertido de SVG para PNG Base64 via Canvas) e renderizando tabelas estatísticas completas.

---

## 9. Criação atômica do experimento (wizard) e atalhos de teclado

Duas decisões de UX com consequência arquitetural, motivadas por teste manual real.

### 9.1 Criação do experimento é uma operação ATÔMICA

**Problema:** o fluxo original obrigava salvamentos intermediários (criar experimento
→ salvar → reabrir → criar grupo → salvar → reabrir → criar animal…). Além de lento,
isso deixava **estrutura pela metade** no banco se o pesquisador desistisse no meio.

**Decisão:** o wizard monta experimento + timepoints + grupos + animais **em memória
no frontend** e envia tudo num único comando, gravado numa **única transação SQLite**:

- Backend: `criar_experimento_completo` (comando Tauri) →
  `criar_experimento_completo_conn` em [`experimentos.rs`](../src-tauri/src/experimentos.rs).
- **Todas as validações acontecem ANTES de abrir a transação** (nome, timepoints,
  nome/cor de grupo, marcação e peso de animal, existência do conjunto de filamentos),
  de modo que erro de preenchimento não chega a tocar o banco.
- Se qualquer `INSERT` falhar já dentro da transação, o `Transaction` do rusqlite é
  descartado e o SQLite faz **rollback total** — não fica experimento órfão, nem grupo,
  nem timepoint.
- No frontend, o erro é exibido e **o formulário NÃO é limpo** — o pesquisador não
  perde o que digitou e pode tentar de novo.

**Testes:** `criar_completo_faz_rollback_total_em_falha_no_meio` (sabota a tabela
`animais` para forçar falha no meio e verifica que nada sobrou),
`criar_completo_valida_antes_de_escrever` e `criar_completo_cria_tudo_numa_transacao`.

**Edição continua incremental:** adicionar grupos/animais a um experimento que já
existe segue usando os comandos `criar_grupo`/`criar_animal` — só o fluxo de **criação
inicial** virou atômico.

### 9.2 Atalhos de teclado no fluxo de teste

Durante a medição o pesquisador tem as mãos ocupadas com o animal e o filamento, então
a tela de execução aceita teclado além dos botões (os botões continuam funcionando):

| Tecla | Ação |
|-------|------|
| `0` | Não respondeu (**O**) |
| `1` | Respondeu (**X**) |
| `Backspace` | Desfazer última aplicação |

Regras implementadas em `handleAtalhosTeste` ([`App.svelte`](../src/App.svelte)), via
um único listener em `<svelte:window>`:

- Só agem quando **a tela de execução está visível** (`showActiveTestScreen`).
- **Não disparam se o foco estiver num campo** (`input`/`textarea`/`select`/
  `contenteditable`) — e nesse caso o `Backspace` **não** é bloqueado, preservando a
  edição normal de texto.
- Ignoram combinações com `Ctrl`/`Alt`/`Meta` e são inibidos enquanto uma gravação
  está em andamento (`carregando`), evitando registro duplicado.
- **Não existe atalho para "finalizar sequência e calcular limiar"** — decisão
  deliberada: finalizar é irreversível para aquela série, então exige clique físico.

Uma dica discreta ("Atalhos: 0 = Não respondeu · 1 = Respondeu · Backspace = Desfazer")
fica logo abaixo dos botões.

---

## 10. Erros nunca podem falhar em silêncio (bug do botão "Finalizar")

**Sintoma relatado:** na tela de execução do teste, o botão "Finalizar sequência e
calcular limiar" "não fazia nada" — sem mensagem, sem travamento.

**Causa raiz — duas falhas somadas, ambas de visibilidade:**

1. **O erro existia, mas era invisível.** O `catch` do `finalizarTeste` gravava em
   `erroMsg`, e o banner de `erroMsg` é renderizado **no topo da página**. Durante a
   medição o pesquisador está com a tela rolada para baixo, no painel de teste (que
   fica ~550 linhas de markup abaixo do banner). O erro era exibido **fora da
   viewport** → visualmente, "nada acontecia".
2. **O botão ficava habilitado num estado que só podia falhar.** `pode_finalizar`
   era `n_nominal >= 2`, **sem limite superior**. Mas a Tabela 7 de Dixon cobre
   apenas **N de 2 a 6** — então, se o pesquisador registrasse aplicações até N > 6,
   o botão continuava verde, o comando era chamado e o motor de Dixon devolvia
   `NNominalForaDaTabela`… cujo erro caía no caso (1) e sumia.

> As três suspeitas iniciais foram descartadas por leitura de código: o handler
> **estava** conectado (`onclick={finalizarTeste}`), os parâmetros **já** iam em
> camelCase (`sequenciaId`) e o comando Rust **já** devolvia `Err` com mensagem clara.
> O defeito estava na *apresentação* do erro e no *gate* do botão.

**Correções:**

- `pode_finalizar_agora(n)` = `n >= 2 && n <= N_NOMINAL_MAX (6)` — o botão só habilita
  quando a finalização pode de fato dar certo (`sequencias.rs`, aplicado em
  `registrar_resposta` e `desfazer_ultima_resposta`), com `aviso` explícito quando N > 6.
- **Erro visível no ponto da ação:** novo estado `erroTeste` no frontend, renderizado
  **dentro do painel de teste**, junto aos botões — usado por `finalizarTeste`,
  `registrarResposta` e `desfazerUltima`. O banner global continua existindo.
- **Explicação quando o botão está desabilitado** (antes ficava mudo): mostra se falta
  reversão ou se N passou de 6.
- **Fim dos `return` mudos:** os `if (!testandoSequencia) return;` dessas três funções
  agora avisam o usuário em vez de sair silenciosamente.

**Princípio geral do projeto:** *nenhum comando pode falhar em silêncio*. Toda ação
disparada pelo usuário precisa (a) tratar o erro, e (b) exibi-lo **onde o usuário está
olhando**. Erro capturado mas renderizado fora da viewport conta como falha silenciosa.

**Testes:** `finalizar_sequencia_conn` foi extraído do comando para ser testável com
banco em memória — `finalizar_sequencia_calcula_e_persiste_limiar` (sucesso, série
Figure 6), `finalizar_sequencia_ja_finalizada_retorna_erro`,
`finalizar_sequencia_vazia_retorna_erro`,
`finalizar_sequencia_com_n_acima_de_6_retorna_erro_claro` e
`pode_finalizar_respeita_faixa_da_tabela`.

### 10.1 O padrão obrigatório para erros (aplicado em todo o app)

Ao escrever **qualquer** função que chame o backend (`invoke`) ou faça uma validação
que possa falhar, siga os três pontos:

1. **Trate o erro explicitamente** — nunca deixe uma Promise rejeitada sem `catch`.
2. **Exiba a mensagem perto da ação** — não só no banner global do topo, que pode
   estar fora da viewport.
3. **Nunca use `return` mudo após uma validação falhar** — diga ao usuário o que falta.

#### Componente reutilizável

Use [`src/lib/AlertaErro.svelte`](../src/lib/AlertaErro.svelte) (estilos
auto-contidos, some sozinho quando a mensagem é `null`, botão de fechar embutido):

```svelte
<script lang="ts">
  import AlertaErro from './lib/AlertaErro.svelte';

  let erroWizard = $state<string | null>(null);   // 1 estado por CONTEXTO de tela

  async function salvarExperimentoCompleto() {
    // (3) validação que falha => mensagem, nunca `return` mudo
    if (!expNome.trim()) {
      erroWizard = "O nome do experimento é obrigatório.";
      return;
    }
    try {
      carregando = true;
      erroWizard = null;
      await invokeCommand('criar_experimento_completo', { /* ... */ });
      showExpForm = false;                    // só fecha/limpa em caso de SUCESSO
    } catch (e: any) {
      // (1) e (2): erro tratado e exibido no contexto + reforço no banner global
      erroWizard = "Não foi possível salvar o experimento (nada foi gravado): " + (e.message || e);
      erroMsg    = "Erro ao salvar experimento: " + (e.message || e);
    } finally {
      carregando = false;
    }
  }
</script>

<!-- (2) o alerta fica JUNTO do botão que dispara a ação -->
<AlertaErro bind:mensagem={erroWizard} />
<div class="form-actions">
  <button onclick={salvarExperimentoCompleto}>Salvar experimento completo</button>
</div>
```

#### Estados de erro por contexto

| Estado | Onde é exibido | Cobre |
|--------|----------------|-------|
| `erroTeste` | painel de teste e formulário de início | iniciar/registrar/desfazer/finalizar/descartar |
| `erroWizard` | formulário/wizard de experimento | criação (atômica) e edição |
| `erroFilamento` | formulário e lista de Conjuntos de Filamentos | salvar/excluir conjunto |
| `erroExperimento` | sub-aba ativa do experimento, formulários de grupo/animal e área de exportação | grupos, animais, estatísticas, CSV/XLSX/PDF/Prism, carga de dados |
| `erroMsg` | banner global no topo | **reforço apenas** — nunca o único lugar |

Limpe o estado local ao **abrir** o formulário e ao **iniciar** a ação (`= null`),
para o usuário não ver um erro velho.

#### `return` mudo: quando é aceitável

Só quando **não houve falha de validação do usuário** — ou seja, quando o "não fazer
nada" é o comportamento correto e esperado. Nesses casos, **comente o porquê**:

```ts
if (!showActiveTestScreen) return;  // no-op intencional: a tecla não é atalho aqui
if (!g) return;                     // guarda interna: índice inválido não é acionável pelo usuário
```

Casos assim no código hoje: os guards de `handleAtalhosTeste`, os `if (!g)` do wizard
e o `carregarEstatisticasGrafico` sem experimento selecionado (a seção nem é renderizada).


### 10.2 Variante de aviso (sucesso parcial)

`AlertaErro` aceita `variante="aviso"` (âmbar) além do padrão `"erro"` (vermelho).

**Use `aviso` quando a ação DEU CERTO mas algo merece atenção** — por exemplo, a
colagem em massa de animais que adicionou 4 itens e ignorou 2 (duplicata + linha
inválida). Sinalizar isso em vermelho como "Erro" faz o pesquisador concluir que
nada foi salvo, o que é pior do que não avisar.

```svelte
<AlertaErro bind:mensagem={erroWizard} />                      <!-- falhou -->
<AlertaErro bind:mensagem={avisoWizard} variante="aviso" />    <!-- deu certo, com ressalvas -->
```
