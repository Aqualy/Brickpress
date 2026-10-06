<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants';
  import { cn, type WithElementRef } from '$lib/utils.js';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  export const buttonVariants = tv({
    base: 'editor-button inline-flex shrink-0 items-center justify-center gap-1.5 rounded-[4px] text-[12px] font-normal select-none disabled:cursor-not-allowed disabled:opacity-40 [&_svg]:shrink-0',
    variants: {
      variant: {
        default: 'bg-primary text-primary-foreground hover:bg-primary/90',
        outline: 'border border-input bg-background hover:bg-muted',
        secondary: 'bg-secondary text-secondary-foreground hover:bg-muted',
        ghost: 'bg-transparent hover:bg-muted',
        destructive: 'text-destructive hover:bg-destructive/5',
        link: 'text-primary underline-offset-4 hover:underline'
      },
      size: {
        default: 'min-h-8 px-2 py-1',
        xs: 'min-h-6 px-1.5 py-0.5 text-[11px]',
        sm: 'min-h-[30px] px-2 py-1',
        lg: 'min-h-9 px-3 py-1',
        icon: 'size-8 p-0',
        'icon-xs': 'size-6 p-0',
        'icon-sm': 'size-[30px] p-0',
        'icon-lg': 'size-9 p-0'
      }
    },
    defaultVariants: {
      variant: 'ghost',
      size: 'default'
    }
  });

  export type ButtonVariant = VariantProps<typeof buttonVariants>['variant'];
  export type ButtonSize = VariantProps<typeof buttonVariants>['size'];

  export type ButtonProps = WithElementRef<HTMLButtonAttributes> & {
    variant?: ButtonVariant;
    size?: ButtonSize;
  };
</script>

<script lang="ts">
  let {
    class: className,
    variant = 'ghost',
    size = 'default',
    ref = $bindable(null),
    type = 'button',
    disabled,
    children,
    ...restProps
  }: ButtonProps = $props();
</script>

<button
  bind:this={ref}
  data-slot="button"
  class={cn(buttonVariants({ variant, size }), className)}
  {type}
  {disabled}
  {...restProps}
>
  {@render children?.()}
</button>
