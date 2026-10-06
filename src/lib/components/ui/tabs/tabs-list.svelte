<script lang="ts" module>
  import { tv, type VariantProps } from 'tailwind-variants';

  export const tabsListVariants = tv({
    base: 'editor-tablist flex shrink-0 items-stretch border-b border-border bg-white',
    variants: {
      variant: {
        default: '',
        line: ''
      }
    },
    defaultVariants: {
      variant: 'default'
    }
  });

  export type TabsListVariant = VariantProps<typeof tabsListVariants>['variant'];
</script>

<script lang="ts">
  import { Tabs as TabsPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    variant = 'default',
    class: className,
    ...restProps
  }: TabsPrimitive.ListProps & {
    variant?: TabsListVariant;
  } = $props();
</script>

<TabsPrimitive.List
  bind:ref
  data-slot="tabs-list"
  data-variant={variant}
  class={cn(tabsListVariants({ variant }), className)}
  {...restProps}
/>
