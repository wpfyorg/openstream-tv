import type { HTMLAttributes, ReactNode } from 'react'

import { cn } from '../lib/cn'

/** White surface with a hairline border. `rule` adds the orange left accent. */
export function Card({
  rule,
  interactive,
  className,
  children,
  ...rest
}: HTMLAttributes<HTMLDivElement> & { rule?: boolean; interactive?: boolean }) {
  return (
    <div
      className={cn(
        'relative rounded-sm border border-line bg-surface',
        rule && 'overflow-hidden before:absolute before:inset-y-0 before:left-0 before:w-[3px] before:bg-brand-bright',
        interactive && 'transition-[box-shadow,border-color] duration-150 hover:border-[#d6d9df] hover:shadow-soft',
        className,
      )}
      {...rest}
    >
      {children}
    </div>
  )
}

export function CardTitle({ icon, children, action }: { icon?: ReactNode; children: ReactNode; action?: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-3">
      <h2 className="flex items-center gap-2 text-sm font-semibold text-fg">
        {icon}
        {children}
      </h2>
      {action}
    </div>
  )
}
