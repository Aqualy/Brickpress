<script lang="ts">
  import type { Editor } from '../../stores/editor.svelte';
  import Icon from './Icon.svelte';
  let { editor, inline = false }: { editor: Editor; inline?: boolean } = $props();
  let returnFocus: HTMLElement | null = null;
  function dismiss() {
    const restore = document.activeElement?.getAttribute('aria-label') === 'Dismiss notification';
    editor.dismissToast();
    if (restore && returnFocus?.isConnected) returnFocus.focus();
  }
</script>

{#if editor.toast}
  <div
    class={inline ? 'inline-notification' : 'toast'}
    role="group"
    aria-label="Notification"
    onpointerenter={() => editor.pauseToast()}
    onpointerleave={(event) => {
      if (!event.currentTarget.contains(document.activeElement)) editor.resumeToast();
    }}
  >
    <span role="status"><Icon name="info" size={16} />{editor.toast}</span>
    <button
      class="icon-button"
      aria-label="Dismiss notification"
      onfocus={(event) => {
        returnFocus = event.relatedTarget instanceof HTMLElement ? event.relatedTarget : null;
        editor.pauseToast();
      }}
      onblur={() => editor.resumeToast()}
      onclick={dismiss}><Icon name="close" size={16} /></button
    >
  </div>
{/if}
