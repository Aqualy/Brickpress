<script lang="ts">
  import * as ResizablePrimitive from 'paneforge';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    withHandle = false,
    ...restProps
  }: WithoutChildrenOrChild<ResizablePrimitive.PaneResizerProps> & {
    withHandle?: boolean;
  } = $props();
</script>

<ResizablePrimitive.PaneResizer
  bind:ref
  data-slot="resizable-handle"
  class={cn('editor-resize-handle relative w-px shrink-0 bg-border', className)}
  {...restProps}
>
  {#if withHandle}
    <div class="bg-border h-6 w-1 rounded-lg z-10 flex shrink-0"></div>
  {/if}
</ResizablePrimitive.PaneResizer>
