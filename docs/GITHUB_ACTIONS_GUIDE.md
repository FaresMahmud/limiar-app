# Guia: Acompanhando e Baixando Builds do GitHub Actions

> Este guia ajuda você a acompanhar a compilação automática (na nuvem) e baixar os instaladores prontos.

---

## Resumo rápido

1. Você faz commit + push para `main` → GitHub Actions compila automaticamente
2. Você acessa a aba "Actions" → escolhe a execução → aguarda conclusão (~15 min)
3. Baixa os artifacts (`.msi` / `.dmg`) → instala normalmente

---

## Passo a passo detalhado

### Passo 1: Fazer commit e push

Após fazer alterações no código, na branch `main`:

```bash
git add .
git commit -m "Sua mensagem de commit"
git push origin main
```

Neste momento, o workflow é **acionado automaticamente**.

> **Nota:** o workflow também pode ser acionado manualmente sem fazer commit.
> Ver "Execução manual" abaixo.

---

### Passo 2: Acessar a aba "Actions"

1. **Vá para seu repositório no GitHub:**
   ```
   https://github.com/seu-usuario/limiar-app
   ```

2. **Clique na aba "Actions"** (no topo do repositório):
   ```
   ┌─────────────────────────────────────────────┐
   │ <> Code  |  Issues  |  Pull requests  |  ... │
   │ Discussions  |  Actions  |  ...           │
   └─────────────────────────────────────────────┘
                          ↑ Clique aqui
   ```

   Ou acesse direto:
   ```
   https://github.com/seu-usuario/limiar-app/actions
   ```

---

### Passo 3: Localizar a execução do workflow

Na página de Actions, você verá uma lista de execuções. A mais recente fica no topo.

```
Workflow "Build Limiar (Windows + macOS)"

📋 Build Limiar (Windows + macOS)  →  ⏳ In progress (ou ✅ Completed)
   └─ Triggered by: seu-usuario
   └─ Branch: main
   └─ Commit: abc123de (sua mensagem de commit)
   └─ Started: 2 minutes ago
```

**Clique na execução mais recente** para abrir os detalhes.

---

### Passo 4: Acompanhar o progresso

Dentro da execução, você verá 4 **jobs** em paralelo/sequência:

```
├─ ✅ test                (testes — verde = passou, vermelho = falhou)
├─ ⏳ build-windows       (compilando para Windows...)
├─ ⏳ build-macos         (compilando para macOS...)
└─ ⏳ summary             (resumo final)
```

Cada job mostra:
- **Status** (⏳ em andamento, ✅ concluído, ❌ erro)
- **Tempo decorrido**
- **Logs** (clique para ver detalhes técnicos se quiser)

**Aguarde até todos os jobs ficarem verdes** (✅). Leva ~15 minutos no total.

---

### Passo 5: Baixar os instaladores (Artifacts)

Quando todos os jobs terminarem com sucesso (✅), **role para baixo** na página até encontrar a seção **"Artifacts"**:

```
════════════════════════════════════════════════════════════════
                          ARTIFACTS

  📦 limiar-windows (155 MB)     ⬇️ Download
  📦 limiar-windows-portable (---)  ⬇️ Download
  📦 limiar-macos (85 MB)        ⬇️ Download

════════════════════════════════════════════════════════════════
```

**Clique em "Download"** ao lado do artifact que deseja:

- **`limiar-windows`** → `.msi` (instalador Windows — **recomendado**)
- **`limiar-windows-portable`** → `.exe` (portável, sem instalação — opcional)
- **`limiar-macos`** → `.dmg` (instalador macOS)

O arquivo será baixado para a pasta padrão de downloads do seu navegador.

---

## Instalando o aplicativo

### Windows

1. Faça download do arquivo `limiar-windows-*.msi`
2. **Clique duas vezes** no `.msi`
3. Siga o assistente de instalação

O app será instalado em:
```
C:\Program Files\Limiar
```

### macOS

1. Faça download do arquivo `limiar-macos-*.dmg`
2. **Clique duas vezes** no `.dmg` (abre um Finder com o app)
3. **Arraste "Limiar"** para a pasta "Applications"

---

## Solução de problemas

### O workflow demorou muito ou está travado

- GitHub Actions pode ter fila. Aguarde mais tempo.
- Se algo falhar, os logs mostram o erro. Procure por linhas em vermelho.

### Todos os jobs ficaram vermelhos (❌)

1. **Clique no job que falhou** para ver o log completo
2. Procure por mensagens de erro (geralmente no final)
3. Possíveis causas:
   - Erro de TypeScript/JavaScript (`npm run check` falhou)
   - Erro de teste Rust (`cargo test` falhou)
   - Erro de compilação Tauri (erro no build do Rust)

Se o erro for no seu código:
- Corrija o código localmente
- Faça commit + push → o workflow roda novamente automaticamente
- Aguarde a próxima execução

### Artifacts estão vazios ou não aparecem

- Os jobs não concluíram com sucesso (confira os logs)
- Ou o comando `npm run tauri build` não gerou os arquivos esperados

---

## Execução manual (sem fazer commit)

Se quiser compilar **sem** fazer commit, use `workflow_dispatch`:

1. **Acesse Actions** → https://github.com/seu-usuario/limiar-app/actions
2. **Clique em "Build Limiar (Windows + macOS)"** (no painel lateral esquerdo)
3. Clique em **"Run workflow"** (botão azul à direita)
4. Confirme a branch (`main`)
5. Clique em **"Run workflow"**

O build começa imediatamente.

---

## Referências

- [Documentação oficial — GitHub Actions](https://docs.github.com/en/actions)
- [Documentação Tauri — Build](https://v2.tauri.app/guides/build/)
- [ARQUITETURA.md § 6.1](./ARQUITETURA.md) — detalhes técnicos do workflow
