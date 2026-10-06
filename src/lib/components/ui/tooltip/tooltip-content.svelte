<script lang="ts">
  import { Tooltip as TooltipPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';
  import type { WithoutChildrenOrChild } from '$lib/utils.js';
  import TooltipPortal from './tooltip-portal.svelte';
  import type { ComponentProps } from 'svelte';

  let {
    ref = $bindable(null),
    class: className,
    sideOffset = 6,
    side = 'top',
    children,
    arrowClasses,
    portalProps,
    ...restProps
  }: TooltipPrimitive.ContentProps & {
    arrowClasses?: string;
    portalProps?: WithoutChildrenOrChild<ComponentProps<typeof TooltipPortal>>;
  } = $props();
</script>

<TooltipPortal {...portalProps}>
  <TooltipPrimitive.Content
    bind:ref
    role="tooltip"
    data-slot="tooltip-content"
    {sideOffset}
    {side}
    class={cn(
      'z-[90] w-fit max-w-xs rounded-[5px] border border-border bg-white px-2 py-1.5 text-[11px] text-foreground',
      className
    )}
    {...restProps}
  >
    {@render children?.()}
    <TooltipPrimitive.Arrow>
      {#snippet child({ props })}
        <div class={cn('hidden', arrowClasses)} {...props}></div>
      {/snippet}
    </TooltipPrimitive.Arrow>
  </TooltipPrimitive.Content>
</TooltipPortal>
