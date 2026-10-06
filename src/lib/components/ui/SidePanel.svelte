<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Dialog } from 'bits-ui';
  import * as Sheet from './sheet';
  import { Button } from './button';
  import type { Editor } from '../../stores/editor.svelte';
  import Icon from './Icon.svelte';
  let {
    editor,
    side,
    label,
    children
  }: { editor: Editor; side: 'palette' | 'inspector'; label: string; children: Snippet } = $props();
  let open = $derived(editor.narrow && editor.drawer === side);
  let returnFocus: HTMLElement | null = null;
  let focusToken = 0;
</script>

<Sheet.Root
  {open}
  onOpenChange={(value) => {
    editor.drawer = value ? side : null;
  }}
>
  {#if open}<Sheet.Overlay class="editor-sheet-overlay" />{/if}
  <Dialog.Content
    forceMount
    trapFocus={editor.narrow}
    preventScroll={editor.narrow}
    onOpenAutoFocus={() => {
      returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      focusToken = editor.canvasFocusToken;
    }}
    onCloseAutoFocus={(event) => {
      event.preventDefault();
      if (focusToken === editor.canvasFocusToken) returnFocus?.focus({ preventScroll: true });
    }}
  >
    {#snippet child({ props })}
      <section
        {...props}
        role={editor.narrow ? 'dialog' : 'region'}
        aria-modal={editor.narrow ? 'true' : undefined}
        aria-label={label}
        hidden={editor.narrow && !open}
        class={`side-panel ${side}-shell ${editor.narrow ? 'drawer' : ''}`}
      >
        <div class="drawer-heading">
          <Sheet.Title>{label}</Sheet.Title>
          <Sheet.Close>
            {#snippet child({ props })}
              <Button
                {...props}
                size="icon"
                variant="ghost"
                class="icon-button"
                aria-label={`Close ${label}`}><Icon name="close" /></Button
              >
            {/snippet}
          </Sheet.Close>
        </div>
        <Sheet.Description class="sr-only"
          >{side === 'palette'
            ? 'Choose a piece, then place it on the artboard.'
            : 'Edit properties, ink passes, and export settings.'}</Sheet.Description
        >
        {@render children()}
      </section>
    {/snippet}
  </Dialog.Content>
</Sheet.Root>
