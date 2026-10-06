<script lang="ts">
  import * as ResizablePrimitive from 'paneforge';
  import { cn } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    api: paneGroup = $bindable(),
    class: className,
    ...restProps
  }: ResizablePrimitive.PaneGroupProps & {
    api?: ResizablePrimitive.PaneGroup;
  } = $props();
</script>

<ResizablePrimitive.PaneGroup
  bind:ref
  bind:this={paneGroup}
  data-slot="resizable-pane-group"
  class={cn('flex h-full w-full data-[direction=vertical]:flex-col', className)}
  {...restProps}
/>
