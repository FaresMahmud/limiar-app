# Restrições não-negociáveis

> Estas são **restrições rígidas** do projeto. Qualquer decisão técnica futura
> deve respeitá-las. Se alguma precisar mudar, isso é uma decisão do dono do
> projeto (o pesquisador), não uma escolha de implementação.

---

## 1. Leveza (RAM e disco)

O app roda em **notebooks fracos e variados** de estudantes/pesquisadores, com
specs desconhecidas. Portanto:

- Shell desktop deve ser **Tauri** (webview nativo do SO), **nunca Electron**.
- Frontend deve ser **o mais leve possível**: **Svelte + Vite puro** (sem
  SvelteKit/SSR desnecessário).
- Evitar dependências pesadas no frontend e no Rust sem justificativa clara.
- **Pendência de validação:** testar em uma máquina fraca de verdade (etapa 7 do
  [ROADMAP.md](ROADMAP.md)).

## 2. Offline total

- Funciona **100% offline**, sem servidor externo e **sem necessidade de internet**.
- Persistência **local** em **SQLite** (`tauri-plugin-sql`), um arquivo por máquina.
- Nenhuma funcionalidade essencial pode depender de rede/nuvem.

## 3. Cross-platform real (Windows **e** macOS)

- Deve **compilar e funcionar** tanto em **Windows** quanto em **macOS**.
- Desenvolvimento é feito em **Windows**.
- **Pendência:** `tauri build` **não faz cross-compile de macOS a partir do
  Windows**. O instalador `.dmg` precisa ser **gerado e validado num Mac**
  depois (etapa 8 do [ROADMAP.md](ROADMAP.md)). Manter a UI e o código portáveis
  para reduzir surpresas nesse momento.

## 4. Instalador único por sistema operacional

- Distribuição via `tauri build`, gerando um **instalador único por SO**:
  - **Windows:** `.msi` e/ou `.exe`.
  - **macOS:** `.dmg`.
- O usuário final **apenas baixa e instala** — clique e usar.

## 5. Zero dependências para o usuário final

- O computador de quem **usa** o app **não precisa de nada extra** instalado:
  **sem Node, sem Rust, sem runtime adicional**.
- Node e Rust são ferramentas **apenas do desenvolvedor** (para `dev`/`build`).
- O instalador deve trazer tudo o que o app precisa (o webview do SO já existe
  no Windows/macOS modernos; garantir o requisito de WebView2 no Windows quando
  aplicável, sem exigir instalação manual pelo usuário).

## 6. Interface em português (pt-BR)

- Toda a interface visível ao usuário é em **português do Brasil** — é software
  para uso **científico no Brasil**.
- Mensagens de erro, labels, relatórios e exportações: pt-BR.

---

## 7. Restrição científica associada (crítica)

Embora seja de domínio, tem força de restrição: a **Tabela 7 de Dixon** deve ser
**transcrita exatamente do artigo original** e cobrir **N = 2 a 6, todas as
combinações O/X**. **Nunca inventar ou interpolar valores.** Ver
[DOMINIO.md](DOMINIO.md) §4. Foi justamente a tabela incompleta que fez o
software antigo travar — não repetir esse erro.
