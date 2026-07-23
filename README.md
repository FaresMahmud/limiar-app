# Limiar — von Frey / Dixon

Software desktop **offline** para calcular, **na hora**, o **limiar de retirada da
pata (PWT)** em testes de sensibilidade mecânica com **filamentos de von Frey**,
usando o **método up-and-down de Dixon**. Feito para um laboratório de
farmacologia/neurociência, com dados persistidos localmente (sem planilha manual).

> Interface em **português (pt-BR)**. Funciona **100% offline**.

---

## 📥 Download

Baixe a versão mais recente na página de [Releases](https://github.com/FaresMahmud/limiar-app/releases/latest).

- **Windows**: baixe o arquivo `.msi` e execute o instalador.
- **macOS**: baixe o arquivo `.dmg` e arraste para a pasta Applications.

> ⚠️ Como o software não possui certificado de assinatura de código pago, o sistema operacional pode exibir um aviso de "editor desconhecido" na primeira execução. Isso é esperado para software gratuito/open-source sem certificado comercial — clique em "Mais informações" → "Executar assim mesmo" (Windows) ou permita a execução nas Preferências de Segurança (macOS).

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

## Gerar o instalador (para desenvolvimento local)

```bash
npm run tauri build
```

O instalador do seu sistema operacional é gerado em
`src-tauri/target/release/bundle/` (`.msi`/`.exe` no Windows, `.dmg` no macOS).

> ⚠️ **Nota importante:** Compilação local pode ser bloqueada pelo Smart App Control
> (Windows 11). Neste caso, use a opção de build automático abaixo.

---

## Baixar instaladores gerados automaticamente (GitHub Actions)

Como o repositório é **público**, cada commit na `main` dispara um workflow automático
que compila o app para **Windows e macOS** na nuvem, sem dependência local de Rust
ou de máquinas específicas.

### Como baixar um build pronto:

1. **Vá para a aba "Actions"** do repositório:
   https://github.com/seu-usuario/limiar-app/actions

2. **Clique no workflow mais recente** (será a execução do último commit na `main`,
   com o nome "Build Limiar (Windows + macOS)").
   - Se o workflow ainda estiver rodando, aguarde (~15 minutos no total).
   - Uma vez concluído, o status muda para ✅ (verde).

3. **Role para baixo** até a seção **"Artifacts"** no pé da página.

4. **Baixe o arquivo desejado:**
   - **Windows:** clique em `limiar-windows` para baixar o `.msi` (instalador)
   - **macOS:** clique em `limiar-macos` para baixar o `.dmg` (instalador)

5. **Instale normalmente:**
   - Windows: clique duas vezes no `.msi` → siga o assistente
   - macOS: abra o `.dmg` → arraste o app para a pasta "Applications"

### Compilar você mesmo para desenvolvimento local

Se você **editar o código** e quiser testar localmente antes de fazer push:

1. Desabilite temporariamente o Smart App Control (se for uma VM pessoal) **OU**
2. Faça commit local, dê push para uma branch, e use o workflow automático como
   "compilador na nuvem"

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

Tauri 2 · Rust · Svelte 5 · TypeScript · Vite · SQLite (via `rusqlite` no backend).

## 📖 Como citar

Se você utilizar o Limiar em sua pesquisa, por favor cite:

[![DOI](https://zenodo.org/badge/10.5281/zenodo.21501921.svg)](https://doi.org/10.5281/zenodo.21501921)

Ou veja o arquivo [CITATION.cff](./CITATION.cff) para o formato de citação estruturado.

## Licença

Este projeto está licenciado sob a licença MIT — veja o arquivo [LICENSE](LICENSE)
para mais detalhes.

> ℹ️ O artigo de Dixon (1980), usado como referência científica durante o
> desenvolvimento, **não** faz parte deste repositório: é conteúdo protegido por
> direitos autorais de terceiros (Annual Reviews). Para reproduzir os valores da
> Tabela 7, consulte o artigo original na fonte.
