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

## 3. Banco de dados: SQLite local via `tauri-plugin-sql`

**Decisão:** persistência em **SQLite**, arquivo local, acessado pelo
**plugin oficial `tauri-plugin-sql`** (feature `sqlite`).

**Motivo:**
- **100% offline**, sem servidor, sem internet — requisito não-negociável.
- Um **único arquivo** de banco por máquina, fácil de localizar/backup.
- Plugin oficial → manutenção e permissões integradas ao modelo de segurança do
  Tauri.

**Onde fica o arquivo:** por padrão o plugin resolve caminhos relativos ao
diretório de dados do app do SO (`AppData` no Windows, `Application Support` no
macOS). O nome/caminho exato do `.db` será definido na etapa de implementação do
banco e documentado em [MODELO_DE_DADOS.md](MODELO_DE_DADOS.md).

**Migrations:** `tauri-plugin-sql` suporta migrations versionadas definidas no
lado Rust (ao registrar o plugin). O schema evoluirá por migrations — nunca
editar o banco à mão em produção.

**Permissões:** concedidas em `src-tauri/capabilities/default.json`
(`"sql:default"`). Sem essa permissão o frontend não consegue falar com o banco.

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

Duas vias de comunicação, ambas locais:

1. **Comandos Tauri (`invoke`)** — o frontend chama funções Rust anotadas com
   `#[tauri::command]` via `import { invoke } from '@tauri-apps/api/core'`. Uso:
   operações que exigem o backend nativo (sistema de arquivos, lógica pesada,
   exportação).

2. **`tauri-plugin-sql`** — o frontend abre o banco e roda SQL diretamente via
   `import Database from '@tauri-apps/plugin-sql'` (`Database.load(...)`,
   `db.execute(...)`, `db.select(...)`). Uso: CRUD das entidades do domínio.

```
┌─────────────── Frontend (Svelte/TS, webview) ───────────────┐
│  UI (pt-BR)  →  src/lib/ (motor Dixon, TS puro)             │
│        │                         │                          │
│   invoke()                 @tauri-apps/plugin-sql           │
└────────┼─────────────────────────┼──────────────────────────┘
         ▼                         ▼
┌──── Backend Rust (src-tauri) ──┐  ┌──── SQLite (arquivo local) ────┐
│ #[tauri::command] handlers     │  │ dados do laboratório           │
│ registro de plugins (lib.rs)   │──│ (via tauri-plugin-sql)         │
└────────────────────────────────┘  └────────────────────────────────┘
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

- Definir se o motor de Dixon fica em TS ou Rust (preferência: TS) — etapa 1.
- Definir nome/caminho do arquivo SQLite e estratégia de migrations — etapa 2.
- Definir formato de exportação de dados (CSV/Excel) — etapa 6.
- Confirmar com o laboratório se `d` é média calculada ou passo fixo do kit —
  ver [DOMINIO.md](DOMINIO.md) §3.
