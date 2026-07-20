# CLAUDE.md — Guia do projeto Limiar (von Frey / Dixon)

> Este arquivo é lido automaticamente pelo Claude Code no início de cada sessão.
> Ele é o ponto de entrada: leia-o primeiro e siga os links para os documentos
> em `docs/` conforme a tarefa exigir. **Mantenha este arquivo atualizado.**

---

## 1. Visão geral

Software **desktop, offline** para um laboratório de **farmacologia/neurociência**
que mede **sensibilidade mecânica em roedores** com **filamentos de von Frey**,
analisados pelo **método up-and-down de Dixon** (Dixon, W.J. 1980, *"Efficient
Analysis of Experimental Observations"*, Ann. Rev. Pharmacol. Toxicol. 20:441-462).

**Problema que resolve:** hoje o laboratório usa um software antigo de terminal
que (a) trava com sequências de 4+ respostas iguais porque a tabela de Dixon está
incompleta e (b) não gerencia dados — cada limiar é anotado à mão e digitado num
Excel à noite. Isso impede decisões em tempo real (ex.: "ainda há efeito do
tratamento? estendo a curva temporal?").

**Ganho central:** calcular o **limiar de retirada da pata (PWT)** *na hora, no
laboratório*, com os dados persistidos localmente.

O "porquê" científico completo está em **[docs/DOMINIO.md](docs/DOMINIO.md)** —
leia antes de mexer em qualquer cálculo.

---

## 2. Stack técnica (e por quê)

| Camada        | Escolha                                   | Motivo |
|---------------|-------------------------------------------|--------|
| Shell desktop | **Tauri 2** (Rust + webview do SO)        | Leve em RAM/disco; roda em notebooks fracos e variados. **NÃO Electron.** |
| Frontend      | **Svelte 5 + TypeScript + Vite**          | O mais leve compatível com Tauri; sem framework de rotas/SSR (Svelte puro, não SvelteKit). |
| Backend       | **Rust** (`src-tauri/`)                    | Comandos nativos + integração SQLite. |
| Banco         | **SQLite local** via `rusqlite` (backend)  | 100% offline, sem servidor, um arquivo por máquina. **Não** usar `tauri-plugin-sql` (removido — ver ARQUITETURA.md §3). |
| Distribuição  | Instalador único por SO (`tauri build`)   | `.msi`/`.exe` (Windows), `.dmg` (macOS). Usuário final não instala nada além do app. |

Decisões e trade-offs detalhados em **[docs/ARQUITETURA.md](docs/ARQUITETURA.md)**.
Restrições não-negociáveis em **[docs/RESTRICOES.md](docs/RESTRICOES.md)**.

---

## 3. Estrutura de pastas

```
limiar-app/
├── CLAUDE.md              ← você está aqui (índice para IA)
├── README.md             ← instruções para humanos rodarem o projeto
├── docs/                 ← documentação de referência (ver índice abaixo)
├── index.html            ← entrada do Vite
├── package.json          ← scripts npm + deps do frontend
├── vite.config.ts        ← Vite integrado ao Tauri (porta fixa 1420)
├── src/                  ← FRONTEND (Svelte + TS)
│   ├── main.ts
│   ├── App.svelte
│   ├── lib/              ← componentes e módulos TS (motor de cálculo, adapter tauri)
│   │   └── tauri.ts      ← integração/mock com Tauri comandos
│   └── assets/
│   └── app.css           ← estilos CSS do app
└── src-tauri/            ← BACKEND (Rust) + configuração Tauri
    ├── Cargo.toml        ← deps Rust (tauri, rusqlite, tauri-plugin-dialog/fs...)
    ├── tauri.conf.json   ← config do app (janela, bundle, identifier, build)
    ├── capabilities/     ← permissões concedidas ao frontend (inclui sql)
    ├── migrations/       ← migrations SQL aplicadas por executar_migracoes (lib.rs)
    │   ├── 001_create_initial_tables.sql
    │   └── 002_create_experimentos_tables.sql
    ├── icons/
    └── src/
        ├── main.rs       ← ponto de entrada (chama lib.rs)
        ├── lib.rs        ← registro de plugins e comandos Tauri
        ├── dixon.rs      ← motor de Dixon
        ├── dixon_table.rs ← tabela 7 transcrita
        ├── filamentos.rs ← CRUD e cálculo de d para kits de filamentos
        └── experimentos.rs ← CRUD para experimentos, grupos e animais
```

---

## 4. Comandos úteis

> **Pré-requisito (só para o DESENVOLVEDOR):** Node.js + Rust/Cargo instalados.
> Rust ainda **não está instalado** nesta máquina — ver pendência na seção 6.

```bash
npm install            # instala deps do frontend (uma vez)
npm run tauri dev      # abre o app em modo desenvolvimento (hot reload) — precisa de Rust
npm run tauri build    # gera o instalador do SO atual em src-tauri/target/release/bundle/
npm run dev            # só o frontend no navegador (sem shell Tauri) — útil p/ UI pura
npm run build          # build de produção só do frontend (Vite)
npm run check          # checagem de tipos Svelte/TS
```

Convenção de portas: Vite serve em **1420** (`strictPort`), HMR em 1421. O Tauri
(`devUrl` em `tauri.conf.json`) aponta para `http://localhost:1420`.

---

## 5. Convenções de código

- **Idioma da interface:** **português (pt-BR)**. Todo texto visível ao usuário
  (labels, mensagens, erros) em pt-BR. Uso científico no Brasil.
- **Idioma do código:** nomes de variáveis/funções em pt-BR são aceitáveis para
  termos de domínio (`limiar`, `filamento`, `sequenciaOX`) — priorize clareza do
  domínio sobre convenção inglesa. Comentários em pt-BR.
- **Frontend ↔ Backend:** comunicação **só** via `invoke()` (comandos Tauri). O
  banco é acessado **apenas pelo backend Rust** (`rusqlite`); o frontend nunca toca
  o SQLite. Regra de ouro: **a lógica científica crítica (tabela de Dixon, cálculo
  do limiar) deve ser testável isoladamente** — manter em módulos Rust puros, sem
  acoplar a UI nem ao banco.
- **Nunca fixar no código** valores que dependem do laboratório: `d` (passo médio
  dos filamentos) é **calculado a partir do cadastro de filamentos**, não hardcoded.
- **Tabela de Dixon:** transcrever **exatamente** do artigo original (Tabela 7).
  **Nunca inventar/interpolar valores de `k`.** Ver pendência na seção 6.
- Commits pequenos e descritivos, em pt-BR.

---

## 6. Pendências conhecidas (importante)

1. **Obter os valores de gramagem dos conjuntos de filamentos A e B** para validar os testes exatos (cujos `d` calculados devem dar ~0.3835 e ~0.4 respectivamente).
2. **Rust/Cargo com bloqueio do SAC nesta máquina.** O build completo do Tauri e macros das dependências são bloqueados pelo Smart App Control do Windows 11. O motor Dixon, o cálculo de `d` e os CRUDs foram testados isoladamente via `cargo test` em crates isolados e `rustc --test`.
3. **Build/validação em macOS pendente.** O desenvolvimento é em Windows; o `.dmg` precisa ser gerado e testado num Mac depois (`tauri build` não faz cross-compile). Ver [docs/RESTRICOES.md](docs/RESTRICOES.md).
4. **Teste em máquina fraca pendente** (requisito de leveza) — etapa 7 do roadmap.

---

## 7. Índice da documentação (`docs/`)

| Documento | Para quê |
|-----------|----------|
| **[docs/DOMINIO.md](docs/DOMINIO.md)** | O método de Dixon, fórmulas, conceitos de negócio e exemplos de `d`. |
| **[docs/ARQUITETURA.md](docs/ARQUITETURA.md)** | Decisões técnicas: Tauri vs Electron, SQLite, fluxo de dados frontend↔backend. |
| **[docs/MODELO_DE_DADOS.md](docs/MODELO_DE_DADOS.md)** | Entidades e relações, tabelas SQLite e decisões de soft-delete. |
| **[docs/RESTRICOES.md](docs/RESTRICOES.md)** | Restrições não-negociáveis (leveza, offline, cross-platform, instalador único). |
| **[docs/ROADMAP.md](docs/ROADMAP.md)** | Próximas etapas em ordem. Estado atual de cada uma. |

---

## 8. Estado atual do projeto

**Etapa 0 concluída:** estrutura + documentação.

**Etapa 1 concluída:** Tabela 7 de Dixon transcrita ([`src-tauri/src/dixon_table.rs`](src-tauri/src/dixon_table.rs)) + motor de cálculo ([`src-tauri/src/dixon.rs`](src-tauri/src/dixon.rs)) + Tauri command `calcular_limiar` ([`lib.rs`](src-tauri/src/lib.rs)) + testes `#[test]`.

**Etapa 2 concluída:** Cadastro do conjunto de filamentos de von Frey, com cálculo de `d` integrado com o SQLite via migrations (`001_create_initial_tables.sql`) e comandos Tauri (`criar_conjunto`, etc.).

**Etapa 3 concluída:** Cadastro de experimentos, grupos de tratamento e animais com suporte a timepoints dinâmicos e persistência SQLite via migration (`002_create_experimentos_tables.sql`).

**Etapa 4 concluída:** Fluxo de teste sequencial O/X (tela de execução do teste, motor Up-Down de Dixon em Rust, N nominal, desfazer cliques e persistência de dados em tempo real).

**Etapa 5 concluída:** Agregação estatística logarítmica (Rust), componente de gráfico SVG de curva temporal em Svelte, e exportações CSV, XLSX (multiapas) e PDF híbridas (diálogos Tauri dialog/fs nativos e web downloads).

**Correção pós 1º teste de integração real (`npx tauri dev` em VM):** o erro
`no such table: conjuntos_filamentos` era causado por dois mecanismos de banco
divergentes. **Consolidado em `rusqlite` como camada única** (migrations por
`executar_migracoes` em `lib.rs`, banco em `<app_data_dir>/limiar.db`); o
`tauri-plugin-sql` (que nunca rodava e apontava para outro arquivo) foi **removido**.
Ver ARQUITETURA.md §3/§5. Validado: migrations criam as 8 tabelas (SQLite real) e a
lógica pura passa em `cargo test` (12/12). **Falta reconfirmar `npx tauri dev` na VM.**

**Pendências abertas:**
- `cargo test` da lógica de cálculo e banco de dados: ✅ 100% passaram (testados de forma isolada sem Tauri / SAC bloqueios). O build completo do app continua bloqueado pelo Smart App Control.
- Obter os valores de gramagem dos conjuntos de filamentos.
- `// VERIFICAR` em `dixon_table.rs`: semântica do incremento "+0,001" das 5 células "+1" — confirmar com o pesquisador.

**Próxima etapa:** #6 do roadmap — testes em máquina fraca / performance.
