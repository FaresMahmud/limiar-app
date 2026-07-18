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

## Etapa 1 — Tabela de Dixon completa + motor de cálculo ⬜ (próxima)

- Transcrever **exatamente** a **Tabela 7 de Dixon (1980)**, **N = 2 a 6, todas as
  combinações O/X**, a partir do **PDF do artigo** (usuário fornecerá).
  **Não inventar valores.**
- Implementar o **motor de cálculo do limiar** (TS puro em `src/lib/`, isolável):
  - função `d` = média das diferenças de `log10(forca)` dos filamentos do kit;
  - fórmula `LIMIAR = 10 ^ (log10(último_filamento) + k × d)`;
  - lookup de `k` pela sequência O/X.
- **Testes automatizados** fixando cada valor de `k` conhecido e casos de limiar.
- Corrigir o bug do software antigo: **suportar sequências de 4+ respostas iguais**.

> Depende de: PDF do artigo (pendência). Referências: [DOMINIO.md](DOMINIO.md) §3–4.

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

- ⬜ **PDF do artigo de Dixon** com a Tabela 7 (bloqueia etapa 1).
- ⬜ **Instalar Rust/Cargo** na máquina de desenvolvimento (bloqueia `dev`/`build`).
- ⬜ **Acesso a um Mac** para validar o build macOS (etapa 8).
- ⬜ Confirmar com o laboratório se `d` é média calculada ou passo fixo do kit.
