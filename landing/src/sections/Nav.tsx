import { useEffect, useState } from 'react'

import { ButtonLink } from '../components/Button'
import { Logo } from '../components/Logo'
import { nav } from '../content'
import { cn } from '../lib/cn'
import { Link } from '../lib/router'

export function Nav() {
  const [scrolled, setScrolled] = useState(false)
  useEffect(() => {
    const on = () => setScrolled(window.scrollY > 8)
    on()
    window.addEventListener('scroll', on, { passive: true })
    return () => window.removeEventListener('scroll', on)
  }, [])
  return (
    <header
      className={cn(
        'sticky top-0 z-40 transition-[background-color,box-shadow] duration-300',
        scrolled ? 'bg-surface/90 shadow-[0_1px_0_var(--color-line)] backdrop-blur' : 'bg-surface',
      )}
    >
      <a href="#main" className="sr-only focus:not-sr-only focus:absolute focus:top-3 focus:left-3 focus:z-50 focus:rounded-sm focus:bg-surface focus:px-3 focus:py-2 focus:text-sm focus:shadow-pop">
        Skip to content
      </a>
      <div className="mx-auto flex h-[72px] max-w-[1440px] items-center gap-6 px-4 sm:px-9">
        <Link href="/" aria-label="OpenStream TV home" className="inline-flex min-h-10 items-center rounded-sm">
          <Logo size={16} />
        </Link>
        <nav aria-label="Sections" className="ml-6 hidden items-center gap-6 lg:flex">
          {nav.links.map((l) => (
            <a key={l.href} href={l.href} className="text-[15px] font-medium text-muted transition-colors hover:text-fg">
              {l.label}
            </a>
          ))}
        </nav>
        <div className="ml-auto flex items-center gap-2 sm:gap-[27px]">
          <a
            href={nav.secondary.href}
            className="hidden h-10 items-center rounded-sm px-2 text-[15px] font-medium text-fg transition-colors hover:text-brand sm:inline-flex"
          >
            {nav.secondary.label}
          </a>
          <ButtonLink href={nav.cta.href} variant="soft" className="h-[38px] rounded-[3px] px-[14px] text-[15px] font-medium sm:px-[14px]">
            {nav.cta.label}
          </ButtonLink>
        </div>
      </div>
    </header>
  )
}
