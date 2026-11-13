import type { ReactNode } from 'react'

import { cn } from '../lib/cn'

export type Tone = 'brand' | 'teal' | 'slate' | 'amber' | 'violet' | 'free' | 'busy' | 'danger'

export const toneClass: Record<Tone, string> = {
  brand: 'bg-brand-soft text-brand-press',
  teal: 'bg-teal-soft text-[#0c6f67]',
  slate: 'bg-slate-soft text-slate',
  amber: 'bg-amber-soft text-amber',
  violet: 'bg-violet-soft text-violet',
  free: 'bg-free-soft text-[#0b6b53]',
  busy: 'bg-busy-soft text-[#4b5260]',
  danger: 'bg-danger-soft text-danger',
}

export function Badge({ tone = 'slate', dot, children, className }: { tone?: Tone; dot?: boolean; children: ReactNode; className?: string }) {
  return (
    <span
      className={cn(
        'inline-flex h-5 shrink-0 items-center gap-1 rounded-xs px-1.5 text-2xs font-semibold whitespace-nowrap',
        toneClass[tone],
        className,
      )}
    >
      {dot && <span aria-hidden className="size-1.5 rounded-full bg-current" />}
      {children}
    </span>
  )
}

/** Square tinted chip behind an icon, as in the resource strip. */
export function IconChip({ tone = 'slate', children, size = 'md' }: { tone?: Tone; children: ReactNode; size?: 'sm' | 'md' | 'lg' }) {
  const iconTone: Partial<Record<Tone, string>> = { teal: 'text-teal', amber: 'text-amber-icon', brand: 'text-brand-bright' }
  return (
    <span
      aria-hidden
      className={cn(
        'inline-grid shrink-0 place-items-center rounded-sm [&>svg]:size-[55%]',
        size === 'sm' ? 'size-6' : size === 'md' ? 'size-7' : 'size-10 rounded-md',
        toneClass[tone],
        iconTone[tone],
      )}
    >
      {children}
    </span>
  )
}
