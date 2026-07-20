<script lang="ts">
  /**
   * Rodapé discreto de créditos (assinatura de autoria).
   *
   * Intencionalmente sóbrio: fonte pequena, cor `--text` com opacidade reduzida,
   * sem negrito e SEM a cor de destaque roxa dos botões principais — para não
   * competir com o conteúdo científico. Fica no fim do fluxo da página (rola junto
   * com o conteúdo), e não fixo, para nunca sobrepor as telas de trabalho
   * (wizard, execução do teste, gráfico) em notebooks de tela pequena.
   *
   * Os links NÃO são <a href> comuns: dentro do WebView nativo do Tauri isso pode
   * navegar a própria janela do app (ou simplesmente não fazer nada). Usamos o
   * `@tauri-apps/plugin-shell` para abrir no navegador PADRÃO do sistema, com
   * fallback para `window.open` quando rodando no navegador (`npm run dev`).
   * Ver docs/ARQUITETURA.md §11.
   */
  const IS_TAURI =
    typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__ !== undefined;

  const LINKEDIN = "https://www.linkedin.com/in/fares-mahmud-412693376/";
  const GITHUB = "https://github.com/FaresMahmud";

  let erro = $state<string | null>(null);

  async function abrirExterno(url: string) {
    erro = null;
    try {
      if (IS_TAURI) {
        const { open } = await import("@tauri-apps/plugin-shell");
        await open(url);
      } else {
        window.open(url, "_blank", "noopener,noreferrer");
      }
    } catch (e: any) {
      // Nunca falhar em silêncio (ARQUITETURA.md §10).
      erro = "Não foi possível abrir o link: " + (e?.message ?? e);
    }
  }
</script>

<footer class="rodape">
  <span>
    Desenvolvido por Fares Mahmud
    <span class="sep">·</span>
    <button type="button" onclick={() => abrirExterno(LINKEDIN)}>LinkedIn</button>
    <span class="sep">·</span>
    <button type="button" onclick={() => abrirExterno(GITHUB)}>GitHub</button>
  </span>
  {#if erro}
    <span class="rodape-erro">{erro}</span>
  {/if}
</footer>

<style>
  .rodape {
    margin-top: 40px;
    padding: 12px 0 4px;
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 12px;
    line-height: 1.4;
    color: var(--text);
    opacity: 0.6;
  }

  .rodape:hover {
    opacity: 0.85;
  }

  .sep {
    opacity: 0.5;
    margin: 0 2px;
  }

  /* Links como <button> para acionar a API do Tauri, mas com aparência de link. */
  .rodape button {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    color: inherit;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .rodape button:hover {
    color: var(--text-h);
  }

  .rodape button:focus-visible {
    outline: 1px solid var(--border);
    outline-offset: 2px;
    border-radius: 2px;
  }

  .rodape-erro {
    width: 100%;
    text-align: center;
    color: #b45309;
    opacity: 1;
  }
</style>
