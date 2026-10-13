<script lang="ts">
  import { cn, type WithElementRef } from '$lib/utils.js';
  import type { HTMLInputAttributes, HTMLInputTypeAttribute } from 'svelte/elements';

  type InputType = Exclude<HTMLInputTypeAttribute, 'file'>;

  type Props = WithElementRef<
    Omit<HTMLInputAttributes, 'type'> &
      ({ type: 'file'; files?: FileList } | { type?: InputType; files?: undefined })
  >;

  let {
    ref = $bindable(null),
    value = $bindable(),
    type,
    files = $bindable(),
    class: className,
    'data-slot': dataSlot = 'input',
    ...restProps
  }: Props = $props();
</script>

{#if type === 'file'}
  <input
    bind:this={ref}
    data-slot={dataSlot}
    class={cn(
      'editor-input min-h-8 w-full min-w-0 rounded-[var(--control-radius)] border border-input bg-popover px-2 py-1 text-[0.75rem] text-foreground placeholder:text-muted-foreground disabled:opacity-40',
      className
    )}
    type="file"
    bind:files
    bind:value
    {...restProps}
  />
{:else}
  <input
    bind:this={ref}
    data-slot={dataSlot}
    class={cn(
      'editor-input min-h-8 w-full min-w-0 rounded-[var(--control-radius)] border border-input bg-popover px-2 py-1 text-[0.75rem] text-foreground placeholder:text-muted-foreground disabled:opacity-40',
      className
    )}
    {type}
    bind:value
    {...restProps}
  />
{/if}
