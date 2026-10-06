<script lang="ts">
  import type { Snippet } from 'svelte';
  import * as Popover from './popover';
  import { Button } from './button';
  let {
    label,
    trigger,
    children,
    className = '',
    placement = 'below',
    width = 300
  }: {
    label: string;
    trigger: Snippet;
    children: Snippet;
    className?: string;
    placement?: 'below' | 'above';
    width?: number;
  } = $props();
  let open = $state(false);
  let triggerElement = $state<HTMLButtonElement | null>(null);
</script>

<Popover.Root bind:open>
  <Popover.Trigger>
    {#snippet child({ props })}
      <Button
        {...props}
        bind:ref={triggerElement}
        variant="ghost"
        class={className}
        aria-label={label}
      >
        {@render trigger()}
      </Button>
    {/snippet}
  </Popover.Trigger>
  <Popover.Content
    class="control-popover"
    role="dialog"
    aria-label={label}
    side={placement === 'above' ? 'top' : 'bottom'}
    align="start"
    sideOffset={6}
    collisionPadding={8}
    style={`width: ${width}px`}
    onCloseAutoFocus={(event) => {
      // A menu action may deliberately focus the artboard.
      if (document.activeElement?.matches('.canvas-workspace')) event.preventDefault();
    }}
    onclick={(event) => {
      if (
        event.target instanceof Element &&
        event.target.closest('.menu-action, .menu-actions button')
      )
        open = false;
    }}
  >
    {@render children()}
  </Popover.Content>
</Popover.Root>
