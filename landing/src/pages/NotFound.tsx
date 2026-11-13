import { ArrowLeft, BookOpen } from 'lucide-react'

import { ButtonLink } from '../components/Button'
import { Logo } from '../components/Logo'
import { brand, links } from '../content'
import { Link, usePath } from '../lib/router'

/** 404: a dead channel, and two ways out. */
export function NotFound() {
  const path = usePath()
  return (
    <div className="flex min-h-dvh flex-col bg-surface">
      <header className="px-5 py-6 sm:px-9">
        <Link href="/" aria-label={`${brand.name} home`} className="inline-flex min-h-10 items-center rounded-sm">
          <Logo size={16} />
        </Link>
      </header>
      <main className="grid flex-1 place-items-center px-5 pb-20 text-center">
        <div className="max-w-md">
          <DeadChannel />
          <p className="mt-6 text-sm font-semibold text-brand">404</p>
          <h1 className="mt-1 text-[32px] leading-10 font-bold tracking-[-0.03em] text-ink">No signal on this channel</h1>
          <p className="mt-2 text-base text-muted">
            Nothing lives at <code className="rounded-xs bg-subtle px-1 py-0.5 text-sm text-body">{path}</code>. It may have moved, or the link is wrong.
          </p>
          <div className="mt-7 flex flex-wrap justify-center gap-3">
            <ButtonLink href="/" variant="outline" icon={<ArrowLeft className="size-4" aria-hidden />}>
              Back home
            </ButtonLink>
            <ButtonLink href={links.docs} icon={<BookOpen className="size-4" aria-hidden />}>
              Documentation
            </ButtonLink>
          </div>
        </div>
      </main>
    </div>
  )
}

function DeadChannel() {
  return (
    <svg viewBox="0 0 240 150" className="mx-auto w-56" aria-hidden>
      <ellipse cx="120" cy="136" rx="98" ry="7" fill="#f3f4f6" />
      <rect x="34" y="26" width="172" height="98" rx="8" fill="#0a0e1c" />
      <rect x="42" y="34" width="156" height="82" rx="4" fill="#141a2e" />
      <g stroke="#3a4363" strokeWidth="4" strokeLinecap="round">
        <path d="M64 62h34" />
        <path d="M64 86h58" />
      </g>
      <g fill="#f24e1e" opacity="0.85">
        {[0, 1, 2, 3, 4].map((i) => (
          <rect key={i} x={140 + i * 10} y={58 + (i % 3) * 10} width="6" height="6" rx="1" />
        ))}
      </g>
      <rect x="104" y="124" width="32" height="6" rx="3" fill="#e5e7eb" />
      <text x="120" y="76" textAnchor="middle" fontSize="12" fontWeight="700" fill="#626977" fontFamily="var(--font-sans)">
        NO SIGNAL
      </text>
    </svg>
  )
}
