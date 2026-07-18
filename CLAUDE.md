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
| Banco         | **SQLite local** via `tauri-plugin-sql`   | 100% offline, sem servidor, um arquivo por máquina. |
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
│   ├── lib/              ← componentes e módulos TS (motor de cálculo virá aqui)
│   └── assets/
└── src-tauri/            ← BACKEND (Rust) + configuração Tauri
    ├── Cargo.toml        ← deps Rust (tauri, tauri-plugin-sql, ...)
    ├── tauri.conf.json   ← config do app (janela, bundle, identifier, build)
    ├── capabilities/     ← permissões concedidas ao frontend (inclui sql)
    ├── icons/
    └── src/
        ├── main.rs       ← ponto de entrada (chama lib.rs)
        └── lib.rs        ← registro de plugins e comandos Tauri
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
- **Frontend ↔ Backend:** comunicação via `invoke()` (comandos Tauri) e via
  `tauri-plugin-sql` para o banco. Regra de ouro: **a lógica científica crítica
  (tabela de Dixon, cálculo do limiar) deve ser testável isoladamente** — manter
  em módulos TS/Rust puros, sem acoplar a componentes de UI.
- **Nunca fixar no código** valores que dependem do laboratório: `d` (passo médio
  dos filamentos) é **calculado a partir do cadastro de filamentos**, não hardcoded.
- **Tabela de Dixon:** transcrever **exatamente** do artigo original (Tabela 7).
  **Nunca inventar/interpolar valores de `k`.** Ver pendência na seção 6.
- Commits pequenos e descritivos, em pt-BR.

---

## 6. Pendências conhecidas (importante)

1. **Tabela 7 de Dixon (valores de `k`) — NÃO IMPLEMENTADA.** O artigo (PDF) será
   fornecido pelo usuário. Deve cobrir **N de 2 a 6, todas as combinações O/X**.
   Placeholder documentado em [docs/ROADMAP.md](docs/ROADMAP.md) e
   [docs/DOMINIO.md](docs/DOMINIO.md). **Não inventar valores.**
2. **Rust/Cargo não instalado nesta máquina.** Necessário para `tauri dev`/`build`.
   Instalar via https://www.rust-lang.org/tools/install (Windows: `rustup`).
3. **Build/validação em macOS pendente.** O desenvolvimento é em Windows; o `.dmg`
   precisa ser gerado e testado num Mac depois (`tauri build` não faz cross-compile
   de macOS a partir do Windows). Ver [docs/RESTRICOES.md](docs/RESTRICOES.md).
4. **Teste em máquina fraca pendente** (requisito de leveza) — etapa 7 do roadmap.

---

## 7. Índice da documentação (`docs/`)

| Documento | Para quê |
|-----------|----------|
| **[docs/DOMINIO.md](docs/DOMINIO.md)** | O método de Dixon, fórmulas, conceitos de negócio. **Leia antes de tocar em cálculos.** |
| **[docs/ARQUITETURA.md](docs/ARQUITETURA.md)** | Decisões técnicas: Tauri vs Electron, SQLite, fluxo de dados frontend↔backend. |
| **[docs/MODELO_DE_DADOS.md](docs/MODELO_DE_DADOS.md)** | Rascunho das entidades e relações (evolui com o projeto). |
| **[docs/RESTRICOES.md](docs/RESTRICOES.md)** | Restrições não-negociáveis (leveza, offline, cross-platform, instalador único). |
| **[docs/ROADMAP.md](docs/ROADMAP.md)** | Próximas etapas em ordem. Estado atual de cada uma. |

---

## 8. Estado atual do projeto

**Etapa 0 concluída:** estrutura + documentação (este commit). Nenhuma tela,
cálculo ou lógica de negócio implementados ainda — proposital.

**Próxima etapa:** #1 do roadmap — transcrever a Tabela 7 de Dixon (completa) e
construir o motor de cálculo do limiar. Aguardando o PDF do artigo.
