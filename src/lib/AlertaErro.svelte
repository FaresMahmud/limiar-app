<script lang="ts">
  /**
   * Alerta de erro LOCAL — o padrão do projeto para "nenhum comando pode falhar
   * em silêncio" (ver docs/ARQUITETURA.md §10).
   *
   * Coloque-o **junto da ação/formulário que falhou**, não só no topo da página:
   * durante o uso real o pesquisador costuma estar com a tela rolada, e um banner
   * distante fica fora da viewport — o erro existe, mas o usuário não o vê (foi
   * exatamente o bug do botão "Finalizar sequência").
   *
   * Uso:
   *   <AlertaErro bind:mensagem={erroWizard} />
   * O botão de fechar limpa a mensagem no pai via `bind:`.
   *
   * Estilos são auto-contidos de propósito (Svelte escopa CSS por componente),
   * para que possa ser reaproveitado em qualquer tela sem depender do App.svelte.
   */
  let {
    mensagem = $bindable(null),
    titulo = "Erro",
  }: {
    mensagem?: string | null;
    titulo?: string;
  } = $props();
</script>

{#if mensagem}
  <div class="alerta-erro" role="alert" aria-live="polite">
    <span><strong>{titulo}:</strong> {mensagem}</span>
    <button type="button" onclick={() => (mensagem = null)} aria-label="Fechar aviso">&times;</button>
  </div>
{/if}

<style>
  .alerta-erro {
    padding: 14px 16px;
    border-radius: 8px;
    margin: 12px 0;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    font-size: 14px;
    line-height: 1.45;
    background-color: #fee2e2;
    border: 1px solid #fecaca;
    color: #991b1b;
  }

  .alerta-erro button {
    background: none;
    border: none;
    font-size: 20px;
    line-height: 1;
    cursor: pointer;
    color: inherit;
    opacity: 0.7;
    padding: 0 2px;
    flex-shrink: 0;
  }

  .alerta-erro button:hover {
    opacity: 1;
  }

  @media (prefers-color-scheme: dark) {
    .alerta-erro {
      background-color: rgba(153, 27, 27, 0.22);
      border-color: rgba(248, 113, 113, 0.45);
      color: #fca5a5;
    }
  }
</style>
