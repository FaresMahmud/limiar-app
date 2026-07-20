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
    titulo,
    variante = "erro",
  }: {
    mensagem?: string | null;
    titulo?: string;
    /**
     * `erro` (vermelho) = a ação FALHOU.
     * `aviso` (âmbar) = a ação deu certo, mas algo merece atenção (ex.: itens
     * ignorados numa colagem). Não use `erro` para sucesso parcial — o sinal
     * vermelho faz o usuário achar que nada foi salvo.
     */
    variante?: "erro" | "aviso";
  } = $props();

  const tituloEfetivo = $derived(titulo ?? (variante === "aviso" ? "Aviso" : "Erro"));
</script>

{#if mensagem}
  <div class="alerta {variante}" role="alert" aria-live="polite">
    <span><strong>{tituloEfetivo}:</strong> {mensagem}</span>
    <button type="button" onclick={() => (mensagem = null)} aria-label="Fechar aviso">&times;</button>
  </div>
{/if}

<style>
  .alerta {
    padding: 14px 16px;
    border-radius: 8px;
    margin: 12px 0;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    font-size: 14px;
    line-height: 1.45;
  }

  .alerta.erro {
    background-color: #fee2e2;
    border: 1px solid #fecaca;
    color: #991b1b;
  }

  .alerta.aviso {
    background-color: #fef3c7;
    border: 1px solid #fde68a;
    color: #92400e;
  }

  .alerta button {
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

  .alerta button:hover {
    opacity: 1;
  }

  @media (prefers-color-scheme: dark) {
    .alerta.erro {
      background-color: rgba(153, 27, 27, 0.22);
      border-color: rgba(248, 113, 113, 0.45);
      color: #fca5a5;
    }
    .alerta.aviso {
      background-color: rgba(146, 64, 14, 0.22);
      border-color: rgba(251, 191, 36, 0.45);
      color: #fcd34d;
    }
  }
</style>
