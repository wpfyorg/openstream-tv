import { cn } from '../lib/cn'
import { Reveal } from './Reveal'

export function SectionHeading({ eyebrow, title, sub, dark, className }: { eyebrow?: string; title: string; sub?: string; dark?: boolean; className?: string }) {
  return (
    <Reveal className={cn('mx-auto max-w-[640px] text-center', className)}>
      {eyebrow && <p className={cn('text-sm font-semibold', dark ? 'text-[#ff8a63]' : 'text-brand')}>{eyebrow}</p>}
      <h2 className={cn('mt-2 text-[30px] leading-[38px] font-bold tracking-[-0.035em] sm:text-h2 sm:leading-[48px] text-balance', dark ? 'text-white' : 'text-ink')}>{title}</h2>
      {sub && <p className={cn('mt-4 text-[17px] leading-7 text-pretty', dark ? 'text-[#b4bacb]' : 'text-muted')}>{sub}</p>}
    </Reveal>
  )
}
