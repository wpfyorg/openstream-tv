/*
  Shared class builders for the landing page. They mirror the small design
  system the page was designed with, so sections stay markup-first.
*/

// Vite's base, so the page's own links still work when GitHub Pages serves it
// from a repository subpath such as /openstream-tv/.
const base = import.meta.env.BASE_URL.replace(/\/$/, '')

export const homeUrl = `${base}/`

// The router compares paths against '/', so the deployment base is stripped
// first; without this every page under /openstream-tv/ looked like a 404.
export function routePath(pathname) {
  if (base && (pathname === base || pathname.startsWith(`${base}/`))) {
    return pathname.slice(base.length) || '/'
  }
  return pathname || '/'
}

export function cn(...parts) {
  return parts.filter(Boolean).join(' ')
}

const buttonVariants = {
  primary: 'bg-brand text-white hover:bg-brand-hover active:bg-brand-press disabled:bg-brand/45',
  soft: 'bg-surface text-fg shadow-soft hover:shadow-pop hover:-translate-y-px active:translate-y-0 active:shadow-soft disabled:text-muted',
  outline:
    'bg-surface text-fg ring-1 ring-inset ring-line hover:bg-canvas hover:ring-[#d6d9df] active:bg-subtle disabled:text-faint disabled:bg-canvas',
  ghost: 'text-body hover:bg-subtle hover:text-fg active:bg-line-soft disabled:text-faint',
  danger:
    'bg-surface text-danger ring-1 ring-inset ring-danger/30 hover:bg-danger-soft active:bg-danger/15 disabled:opacity-50',
}

const buttonSizes = {
  xs: 'h-7 px-2.5 text-xs gap-1 rounded-sm',
  sm: 'h-8 px-3 text-sm gap-1.5 rounded-sm',
  md: 'h-10 px-4 text-[14px] gap-2 rounded-sm',
  lg: 'h-[46px] px-7 text-lg gap-2.5 rounded-xs',
}

export function buttonClass(variant = 'primary', size = 'md', extra = '') {
  return cn(
    'relative inline-flex select-none items-center justify-center whitespace-nowrap font-semibold tracking-[-0.01em]',
    'transition-[background-color,box-shadow,transform,color] duration-150 ease-out active:scale-[0.98]',
    buttonVariants[variant],
    variant === 'primary' && (size === 'md' || size === 'lg') && 'shadow-button',
    buttonSizes[size],
    extra,
  )
}

const tones = {
  brand: 'bg-brand-soft text-brand-press',
  teal: 'bg-teal-soft text-[#0c6f67]',
  slate: 'bg-slate-soft text-slate',
  amber: 'bg-amber-soft text-amber',
  violet: 'bg-violet-soft text-violet',
  free: 'bg-free-soft text-[#0b6b53]',
  busy: 'bg-busy-soft text-[#4b5260]',
  danger: 'bg-danger-soft text-danger',
}

export function badgeClass(tone = 'slate', extra = '') {
  return cn(
    'inline-flex h-5 shrink-0 items-center gap-1 rounded-xs px-1.5 text-2xs font-semibold whitespace-nowrap',
    tones[tone],
    extra,
  )
}

const chipIconTones = { teal: 'text-teal', amber: 'text-amber-icon', brand: 'text-brand-bright' }

export function iconChipClass(tone = 'slate', size = 'md') {
  return cn(
    'inline-grid shrink-0 place-items-center rounded-sm [&>svg]:size-[55%]',
    size === 'sm' ? 'size-6' : size === 'md' ? 'size-7' : 'size-10 rounded-md',
    tones[tone],
    chipIconTones[tone],
  )
}

export function cardClass({ rule = false, interactive = false, extra = '' } = {}) {
  return cn(
    'relative rounded-sm border border-line bg-surface',
    rule && 'overflow-hidden before:absolute before:inset-y-0 before:left-0 before:w-[3px] before:bg-brand-bright',
    interactive && 'transition-[box-shadow,border-color] duration-150 hover:border-[#d6d9df] hover:shadow-soft',
    extra,
  )
}
