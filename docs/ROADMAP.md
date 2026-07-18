# Roadmap

> Próximas etapas em ordem de execução. Estado de cada uma marcado com:
> ⬜ não iniciado · 🟡 em andamento · ✅ concluído.
>
> **Regra:** avançar uma etapa por vez, validando antes de seguir. Atualizar o
> estado aqui a cada avanço.

---

## Etapa 0 — Estrutura + documentação ✅ (concluída)

Scaffold Tauri + Svelte + TS + Vite, `tauri-plugin-sql` configurado, estrutura de
pastas, toda a documentação (`CLAUDE.md`, `docs/`), `.gitignore`, `README.md` e
repositório git inicializado. **Nenhuma lógica de negócio implementada** (proposital).

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

## Etapa 2 — Cadastro de filamentos / laboratório ⬜

- CRUD de `ConjuntoDeFilamentos` e `Filamento` no SQLite.
- Migrations iniciais do banco; definir nome/caminho do arquivo `.db`.
- Cálculo automático de `d` a partir do cadastro (nunca hardcoded).

## Etapa 3 — Cadastro de animais e grupos ⬜

- CRUD de `Experimento`, `Grupo`, `Animal`.
- Identificação por marcação (texto livre) + cor de grupo.
- (Pode incluir a randomização/balanceamento por limiar basal, ou deixar p/ depois.)

## Etapa 4 — Fluxo de teste sequencial O/X ⬜

- Tela de execução do teste up-and-down: registrar O/X, sugerir próximo filamento
  (sobe/desce), calcular o limiar **na hora** ao fechar a sequência.
- Persistir `SequenciaDeTeste` + `Limiar` (com `k` e `d` usados, p/ rastreabilidade).

## Etapa 5 — Gestão de experimentos e timepoints ⬜

- Definição da curva temporal (basal, indução, 1h, 2h... 8h/24h opcionais).
- Visão por animal/grupo ao longo dos timepoints; apoio à decisão de estender a curva.

## Etapa 6 — Exportação de dados ⬜

- Exportar resultados (CSV/Excel) para substituir o Excel manual noturno.
- Definir formato/colunas com o laboratório.

## Etapa 7 — Testes em máquina fraca ⬜

- Validar RAM/disco/responsividade num notebook fraco real (requisito de leveza).

## Etapa 8 — Build final e instaladores ⬜

- `tauri build` no Windows (`.msi`/`.exe`).
- Gerar e **validar o `.dmg` num Mac** (cross-compile de macOS não é possível a
  partir do Windows — pendência de [RESTRICOES.md](RESTRICOES.md)).
- Verificar que o usuário final não precisa de nenhuma dependência extra.

---

## Pendências transversais (não são etapas, mas bloqueiam/afetam várias)

- ✅ **PDF do artigo de Dixon** com a Tabela 7 — em `docs/referencia/dixon1980.pdf`.
- ⬜ **Instalar Rust/Cargo** na máquina de desenvolvimento (bloqueia `dev`/`build`
  e `cargo test`).
- ⬜ **Acesso a um Mac** para validar o build macOS (etapa 8).
- ⬜ Confirmar com o laboratório se `d` é média calculada ou passo fixo do kit.
