<script lang="ts" module>
  export type Side = 'top' | 'right' | 'bottom' | 'left';
</script>

<script lang="ts">
  import { Dialog as SheetPrimitive } from 'bits-ui';
  import XIcon from '@lucide/svelte/icons/x';
  import { Button } from '$lib/components/ui/button/index.js';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';
  import SheetOverlay from './sheet-overlay.svelte';
  import SheetPortal from './sheet-portal.svelte';
  import type { Snippet } from 'svelte';
  import type { ComponentProps } from 'svelte';

  let {
    ref = $bindable(null),
    class: className,
    side = 'right',
    showCloseButton = true,
    portalProps,
    children,
    ...restProps
  }: WithoutChildrenOrChild<SheetPrimitive.ContentProps> & {
    portalProps?: WithoutChildrenOrChild<ComponentProps<typeof SheetPortal>>;
    side?: Side;
    showCloseButton?: boolean;
    children: Snippet;
  } = $props();
</script>

<SheetPortal {...portalProps}>
  <SheetOverlay />
  <SheetPrimitive.Content
    bind:ref
    data-slot="sheet-content"
    data-side={side}
    class={cn(
      'editor-sheet fixed z-[70] flex flex-col border-border bg-white text-[12px] data-[side=right]:inset-y-0 data-[side=right]:w-[340px] data-[side=right]:max-w-full data-[side=right]:border-l data-[side=right]:right-0 data-[side=left]:inset-y-0 data-[side=left]:w-[340px] data-[side=left]:max-w-full data-[side=left]:border-r data-[side=left]:left-0 data-[side=top]:inset-x-0 data-[side=top]:border-b data-[side=top]:top-0 data-[side=bottom]:inset-x-0 data-[side=bottom]:border-t data-[side=bottom]:bottom-0',
      className
    )}
    {...restProps}
  >
    {@render children?.()}
    {#if showCloseButton}
      <SheetPrimitive.Close data-slot="sheet-close">
        {#snippet child({ props })}
          <Button variant="ghost" class="absolute top-3 right-3" size="icon-sm" {...props}>
            <XIcon />
            <span class="sr-only">Close</span>
          </Button>
        {/snippet}
      </SheetPrimitive.Close>
    {/if}
  </SheetPrimitive.Content>
</SheetPortal>
