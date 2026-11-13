import type { ReactNode } from 'react'

import { cn } from '../lib/cn'

/** A desktop window: traffic lights and a title bar around any content. */
export function WindowFrame({ title, children, className }: { title: string; children: ReactNode; className?: string }) {
  return (
    <div className={cn('overflow-hidden rounded-[10px] border-[2.5px] border-[#2a2f3d] bg-surface shadow-window', className)}>
      <div className="flex h-[46px] items-center gap-[7px] border-b border-line bg-[#fbfbfc] px-5">
        <span aria-hidden className="size-[13px] rounded-full bg-light-red" />
        <span aria-hidden className="size-[13px] rounded-full bg-light-yellow" />
        <span aria-hidden className="size-[13px] rounded-full bg-light-green" />
        <span className="ml-[18px] text-[15px] text-muted">{title}</span>
      </div>
      {children}
    </div>
  )
}
