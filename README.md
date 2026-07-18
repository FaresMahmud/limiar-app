# Limiar — von Frey / Dixon

Software desktop **offline** para calcular, **na hora**, o **limiar de retirada da
pata (PWT)** em testes de sensibilidade mecânica com **filamentos de von Frey**,
usando o **método up-and-down de Dixon**. Feito para um laboratório de
farmacologia/neurociência, com dados persistidos localmente (sem planilha manual).

> Interface em **português (pt-BR)**. Funciona **100% offline**.

---

## Pré-requisitos (apenas para desenvolver)

Quem só vai **usar** o app não precisa de nada disto — basta o instalador.
Para **desenvolver/compilar**, você precisa de:

- **Node.js** (LTS recente) — https://nodejs.org
- **Rust + Cargo** — https://www.rust-lang.org/tools/install
  (no Windows, instale via `rustup`)
- Pré-requisitos de sistema do Tauri para o seu SO:
  https://v2.tauri.app/start/prerequisites/
  (no Windows: **WebView2** e as **Build Tools do Visual Studio** com C++)

## Rodar em modo desenvolvimento

```bash
npm install        # instala as dependências do frontend (uma vez)
npm run tauri dev  # abre o app com hot reload
```

Isso sobe o servidor Vite (porta 1420) e abre a janela do app Tauri.

> Só quer mexer na interface, sem o shell desktop? `npm run dev` roda apenas o
> frontend no navegador.

## Gerar o instalador

```bash
npm run tauri build
```

O instalador do seu sistema operacional é gerado em
`src-tauri/target/release/bundle/` (`.msi`/`.exe` no Windows, `.dmg` no macOS).

> ⚠️ O `.dmg` (macOS) **precisa ser gerado num Mac** — não é possível compilar
> para macOS a partir do Windows.

---

## Documentação

- **[CLAUDE.md](CLAUDE.md)** — visão geral, stack, comandos, convenções (ponto de
  entrada para o Claude Code / outra IA).
- **[docs/DOMINIO.md](docs/DOMINIO.md)** — o método de Dixon e os conceitos científicos.
- **[docs/ARQUITETURA.md](docs/ARQUITETURA.md)** — decisões técnicas.
- **[docs/MODELO_DE_DADOS.md](docs/MODELO_DE_DADOS.md)** — entidades e relações.
- **[docs/RESTRICOES.md](docs/RESTRICOES.md)** — restrições não-negociáveis.
- **[docs/ROADMAP.md](docs/ROADMAP.md)** — próximas etapas.

## Stack

Tauri 2 · Rust · Svelte 5 · TypeScript · Vite · SQLite (`tauri-plugin-sql`).
