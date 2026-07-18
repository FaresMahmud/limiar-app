# Roadmap

> Próximas etapas em ordem de execução. Estado de cada uma marcado com:
> ⬜ não iniciado · 🟡 em andamento · ✅ concluído.
>
> **Regra:** avançar uma etapa por vez, validando antes de seguir. Atualizar o
> estado aqui a cada avanço.

---

## Etapa 0 — Estrutura + documentação ✅ (concluída)

Scaffold Tauri + Svelte + TS + Vite, SQLite configurado, estrutura de
pastas, toda a documentação (`CLAUDE.md`, `docs/`), `.gitignore`, `README.md` e
repositório git inicializado. **Nenhuma lógica de negócio implementada** (proposital).
(O acesso a SQLite migrou depois de `tauri-plugin-sql` para `rusqlite` no backend —
ver ARQUITETURA.md §3.)

---

## Etapa 1 — Tabela de Dixon completa + motor de cálculo ✅ (concluída)

- ✅ **Tabela 7** transcrita **exatamente** do PDF (N=2 a 6, todas as combinações
  O/X, + erro padrão por N) em [`src-tauri/src/dixon_table.rs`](../src-tauri/src/dixon_table.rs).
- ✅ **Motor de cálculo** em Rust ([`src-tauri/src/dixon.rs`](../src-tauri/src/dixon.rs)),
  puro e testável, com a fórmula `LIMIAR = 10^(log10(xf) + k·d)`, decodificação
  bidirecional (primeira/segunda parte, entrada "pelo pé" com inversão de sinal).
- ✅ Exposto como Tauri command `calcular_limiar` em [`lib.rs`](../src-tauri/src/lib.rs).
- ✅ **Testes** (`#[test]`) fixando o exemplo resolvido do artigo (Figure 6 → 0.852)
  + inversão de sinal, valores da tabela, corrida longa (4+ iguais) e erros.
- ✅ Bug do software antigo resolvido: **sequências de 4+ respostas iguais** agora
  são tratadas (coluna OOOO + incremento), sem travar.

**Decisão de arquitetura:** o motor ficou em **Rust** (não TS) — melhor lugar para
lógica científica testável via `cargo test` e já perto do banco; o frontend chama
via command. (Atualiza a preferência provisória registrada em ARQUITETURA.md.)

**Validação (`cargo test`):** ✅ **6/6 testes passaram**, incluindo o obrigatório
`figura6_exemplo_do_artigo` (Figure 6 → 0.852). Rodados num crate isolado
(`scratchpad/dixon_check`) com os arquivos reais `dixon.rs`/`dixon_table.rs`,
porque o build completo do app está bloqueado (ver abaixo).

**Pendências desta etapa:**
- 🚫 **BLOQUEIO — Smart App Control (SAC).** `cargo build`/`cargo test` do app
  completo falha com `os error 4551` ("política de Controle de Aplicativo bloqueou
  este arquivo"): o **Smart App Control do Windows 11 (estado = LIGADO)** bloqueia a
  execução/carregamento dos artefatos não-assinados que o Rust gera (build scripts
  e DLLs de proc-macro das dependências do Tauri). O motor de Dixon (Rust puro, sem
  build scripts/proc-macros) compila e testa normalmente; só o build com as deps do
  Tauri é afetado. **Ação do usuário:** desligar o Smart App Control em *Segurança
  do Windows → Controle de aplicativos e navegador → Smart App Control → Desativado*
  (⚠️ no Windows 11 essa opção, uma vez desligada, só volta reinstalando o Windows),
  ou compilar em outra máquina/VM sem SAC. Isso também será necessário para
  `npm run tauri dev`/`build`.
- ⚠️ `// VERIFICAR`: condição de aplicação do incremento "+0,001" das 5 células
  com sobrescrito "+1" — confirmar leitura do artigo com o pesquisador.
- Cálculo automático de `d` a partir do cadastro de filamentos fica para a etapa 2
  (aqui `d` é recebido como parâmetro).

> Referências: [DOMINIO.md](DOMINIO.md) §3–4.

## Etapa 2 — Cadastro de filamentos / laboratório ✅ (concluída)

- ✅ **Schema SQLite e Migrations**: Criadas as tabelas `conjuntos_filamentos` (com coluna `ativo` para soft-delete) e `filamentos` em `src-tauri/migrations/001_create_initial_tables.sql`. As migrations são aplicadas por `executar_migracoes` (`lib.rs`) via `rusqlite`/`PRAGMA user_version` no arquivo `<app_data_dir>/limiar.db`. (Correção pós-teste de integração: o `tauri-plugin-sql` foi removido — apontava para outro banco e nunca rodava; ver ARQUITETURA.md §3.)
- ✅ **Lógica de Cálculo de d**: Implementada em Rust (`src-tauri/src/filamentos.rs`), com ordenação interna automática, validação de limites (valores > 0 e válidos) e prevenção contra duplicados.
- ✅ **Comandos Tauri**: Implementados e expostos os comandos `criar_conjunto`, `listar_conjuntos`, `editar_conjunto`, `excluir_conjunto` e `recalcular_d_conjunto` integrados com o SQLite via `rusqlite`.
- ✅ **Interface Svelte 5 (UI)**: Criada a tela de gerenciamento de filamentos (`src/App.svelte`) utilizando Runes do Svelte 5. Permite visualização em cards, adição/remoção dinâmica de linhas de filamentos no formulário e cálculo reativo de `d` em tempo real.
- ✅ **Mocks para Desenvolvimento**: Implementado um adaptador em `src/lib/tauri.ts` que simula as chamadas de banco e cálculo em memória no navegador quando executado sem o Tauri shell (facilitando validação offline e contornando o bloqueio do SAC).
- ✅ **Testes Unitários**: Criados testes na lógica Rust de cálculo e validados de forma isolada com 100% de sucesso.
- ✅ **Decisão de Arquitetura**: O valor de `d` é calculado no backend e persistido no banco para garantir imutabilidade e consistência histórica, mantendo a escala granular relacionada na tabela de filamentos.

## Etapa 3 — Cadastro de animais e grupos ✅ (concluída)

- ✅ **Schema SQLite e Migrations**: Criadas as tabelas `experimentos`, `grupos`, `animais` e `timepoints` via migration `002_create_experimentos_tables.sql` registrada em `src-tauri/src/lib.rs`.
- ✅ **Comandos Tauri**: Implementados e expostos os comandos CRUD `criar_experimento`, `listar_experimentos`, `obter_experimento`, `editar_experimento`, `excluir_experimento`, `criar_grupo`, `editar_grupo`, `excluir_grupo`, `criar_animal`, `editar_animal` e `excluir_animal` integrados no banco com tratamentos de erros limpos no Rust (`src-tauri/src/experimentos.rs`).
- ✅ **Decisão de Integridade & Cascade**: As exclusões são lógicas (soft-delete via coluna `ativo`) nos níveis de experimento, grupo e animal para proteger dados nociceptivos históricos de exclusão acidental. Deleção em cascata `ON DELETE CASCADE` configurada no banco físico. Deleção de grupo desativa recursivamente os animais associados.
- ✅ **Interface Svelte 5 (UI)**: Atualizada a interface (`src/App.svelte`) para suportar a alternância entre a aba de filamentos e a aba de experimentos. Fornece gerenciador visual da curva temporal (timepoints dinâmicos por experimento), editor de grupos (com color picker HTML) e tabela de animais por grupo (marcação visual e peso corporal).
- ✅ **Mocks e Validação**: Atualizado o banco em memória no arquivo `src/lib/tauri.ts` contendo toda a simulação do CRUD de experimentos para testes web.
- ✅ **Testes Unitários**: Criados testes na lógica Rust de banco de dados e validados de forma isolada com 100% de sucesso.

**Ideia de Melhoria Futura (Não-compromisso):**
- *Balanceamento assistido de grupos (randomização)*: Criar assistente de alocação de animais com base nos limiares basais medidos para distribuir animais homogeneamente entre os grupos (ex.: minimizar desvio padrão do basal médio entre os grupos).

## Etapa 4 — Fluxo de teste sequencial O/X ✅ (concluída)

- ✅ Tela de execução do teste up-and-down com O/X diferenciados por peso visual e ícones para acessibilidade de daltonismo.
- ✅ Persistência instantânea no SQLite para evitar perda de dados por queda de luz ou travamentos.
- ✅ Sugestões automáticas de subida/descida de filamento, tratamento inteligente de limites inferior/superior do kit, cálculo dinâmico de N nominal ideal e possibilidade de desfazer a última aplicação.

## Etapa 5 — Agregação estatística, gráfico e exportação ✅ (concluída)

- ✅ Módulo de cálculo estatístico logarítmico (média geométrica nociceptiva central e erro padrão amostral assimétrico) implementado em Rust com testes lógicos isolados.
- ✅ Componente gráfico de curva temporal em SVG Puro customizado em Svelte com barras de erro assimétricas, legenda e exportação nativa PNG/SVG de alta definição.
- ✅ Exportação unificada de respostas cruas e limiares em CSV.
- ✅ Exportação XLSX formatada em abas múltiplas (Dados Brutos, Limiares, Resumo de Estatísticas por Grupo) via SheetJS.
- ✅ Geração de relatório estruturado em PDF com metadados, gráfico embutido e tabela de estatísticas agregadas por jsPDF.
- ✅ Integração com os plugins `@tauri-apps/plugin-dialog` e `@tauri-apps/plugin-fs` para prover diálogos de salvamento nativos no desktop e download via blob em navegadores mockados.

## Etapa 6 — Testes em máquina fraca / Performance ⬜

- ⬜ Validar RAM/disco/responsividade num notebook fraco real (requisito de leveza).

## Etapa 7 — Build final e instaladores ⬜

- ⬜ `tauri build` no Windows (`.msi`/`.exe`).
- ⬜ Gerar e **validar o `.dmg` num Mac** (cross-compile de macOS não é possível a partir do Windows).
- ⬜ Verificar que o usuário final não precisa de nenhuma dependência extra.

---

## Pendências transversais (não são etapas, mas bloqueiam/afetam várias)

- ✅ **PDF do artigo de Dixon** com a Tabela 7 — em `docs/referencia/dixon1980.pdf`.
- ⬜ **Instalar Rust/Cargo** na máquina de desenvolvimento (bloqueia `dev`/`build` do Tauri e exige contornar o Smart App Control).
- ⬜ **Acesso a um Mac** para validar o build macOS (etapa 8).
- ✅ Confirmar com o laboratório se `d` é média calculada ou passo fixo do kit (implementado como cálculo automático da média das diferenças consecutivas).
- ✅ Obter e validar os valores reais de gramagem dos conjuntos de filamentos:
  - ✅ **Kit do Laboratório Principal (Real e definitivo)**: `[0.02, 0.07, 0.16, 0.4, 1.0, 2.0, 4.0]`. O valor `d` calculado de forma automática é aproximadamente **0.3835**, o que bate exatamente com o histórico usado na planilha do laboratório.
  - ⬜ **Kit do Segundo Laboratório (Colega)**: ainda pendente (aguardando valores reais do usuário).
