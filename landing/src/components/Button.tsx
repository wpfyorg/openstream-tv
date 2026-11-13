import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from 'react'

import { cn } from '../lib/cn'
import { Link } from '../lib/router'
import { Spinner } from './Spinner'

export type ButtonVariant = 'primary' | 'soft' | 'outline' | 'ghost' | 'danger'
export type ButtonSize = 'xs' | 'sm' | 'md' | 'lg'

const variants: Record<ButtonVariant, string> = {
  primary: 'bg-brand text-white hover:bg-brand-hover active:bg-brand-press disabled:bg-brand/45',
  soft: 'bg-surface text-fg shadow-soft hover:shadow-pop hover:-translate-y-px active:translate-y-0 active:shadow-soft disabled:text-muted',
  outline:
    'bg-surface text-fg ring-1 ring-inset ring-line hover:bg-canvas hover:ring-[#d6d9df] active:bg-subtle disabled:text-faint disabled:bg-canvas',
  ghost: 'text-body hover:bg-subtle hover:text-fg active:bg-line-soft disabled:text-faint',
  danger: 'bg-surface text-danger ring-1 ring-inset ring-danger/30 hover:bg-danger-soft active:bg-danger/15 disabled:opacity-50',
}

const sizes: Record<ButtonSize, string> = {
  xs: 'h-7 px-2.5 text-xs gap-1 rounded-sm',
  sm: 'h-8 px-3 text-sm gap-1.5 rounded-sm',
  md: 'h-10 px-4 text-[14px] gap-2 rounded-sm',
  lg: 'h-[46px] px-7 text-lg gap-2.5 rounded-xs',
}

/** Static look of each interactive state, so /styleguide can show them side by side. */
export const forcedState: Record<ButtonVariant, Record<'hover' | 'focus' | 'active', string>> = {
  primary: { hover: '!bg-brand-hover', focus: 'outline-2 outline-offset-2 outline-brand', active: '!bg-brand-press scale-[0.98]' },
  soft: { hover: '!shadow-pop -translate-y-px', focus: 'outline-2 outline-offset-2 outline-brand', active: 'scale-[0.98]' },
  outline: { hover: '!bg-canvas !ring-[#d6d9df]', focus: 'outline-2 outline-offset-2 outline-brand', active: '!bg-subtle scale-[0.98]' },
  ghost: { hover: '!bg-subtle !text-fg', focus: 'outline-2 outline-offset-2 outline-brand', active: '!bg-line-soft scale-[0.98]' },
  danger: { hover: '!bg-danger-soft', focus: 'outline-2 outline-offset-2 outline-brand', active: '!bg-danger/15 scale-[0.98]' },
}

export function buttonClass(variant: ButtonVariant = 'primary', size: ButtonSize = 'md', className?: string) {
  return cn(
    'relative inline-flex select-none items-center justify-center whitespace-nowrap font-semibold tracking-[-0.01em]',
    'transition-[background-color,box-shadow,transform,color] duration-150 ease-out active:scale-[0.98] disabled:pointer-events-none',
    variants[variant],
    variant === 'primary' && (size === 'md' || size === 'lg') && 'shadow-button disabled:shadow-none',
    sizes[size],
    className,
  )
}

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant
  size?: ButtonSize
  loading?: boolean
  icon?: ReactNode
  iconRight?: ReactNode
}

/** The one button. `loading` keeps its width and swaps the icon for a spinner. */
export const Button = forwardRef<HTMLButtonElement, Props>(function Button(
  { variant = 'primary', size = 'md', loading, icon, iconRight, className, children, disabled, type = 'button', ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      className={buttonClass(variant, size, className)}
      {...rest}
    >
      {loading ? <Spinner className="size-3.5" /> : icon}
      {children}
      {iconRight}
    </button>
  )
})

/** Same look, as an in-app link. */
export function ButtonLink({
  href,
  variant = 'primary',
  size = 'md',
  className,
  children,
  icon,
  iconRight,
  ...rest
}: {
  href: string
  variant?: ButtonVariant
  size?: ButtonSize
  className?: string
  children: ReactNode
  icon?: ReactNode
  iconRight?: ReactNode
  'aria-label'?: string
}) {
  return (
    <Link href={href} className={buttonClass(variant, size, className)} {...rest}>
      {icon}
      {children}
      {iconRight}
    </Link>
  )
}

export function IconButton({
  label,
  className,
  children,
  size = 'md',
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { label: string; size?: 'sm' | 'md' }) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={cn(
        'relative inline-grid shrink-0 place-items-center rounded-md text-muted transition-colors duration-150 hover:bg-subtle hover:text-fg active:bg-line-soft disabled:opacity-40',
        size === 'sm' ? 'size-10 md:size-8' : 'size-10',
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  )
}
