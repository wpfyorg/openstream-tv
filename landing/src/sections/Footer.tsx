import { ArrowUpRight } from 'lucide-react'

import { Logo } from '../components/Logo'
import { footer } from '../content'

export function Footer() {
  return (
    <footer className="border-t border-line px-4 pt-14 pb-10 sm:px-9">
      <div className="mx-auto grid max-w-[1120px] gap-10 md:grid-cols-[1.4fr_1fr_1fr_1.6fr]">
        <div>
          <Logo size={16} />
          <p className="mt-3 max-w-[240px] text-sm text-muted">{footer.blurb}</p>
        </div>
        {footer.columns.map((c) => (
          <nav key={c.title} aria-label={c.title}>
            <h2 className="text-sm font-semibold text-fg">{c.title}</h2>
            <ul className="mt-2 grid">
              {c.links.map((l) => (
                <li key={l.label}>
                  <a
                    href={l.href}
                    className="inline-flex h-10 items-center gap-1 text-sm text-muted hover:text-fg"
                    {...(l.href.startsWith('#') ? {} : { target: '_blank', rel: 'noreferrer' })}
                  >
                    {l.label}
                    {!l.href.startsWith('#') && <ArrowUpRight aria-hidden className="size-3.5 text-faint" />}
                  </a>
                </li>
              ))}
            </ul>
          </nav>
        ))}
        <div className="min-w-0">
          <h2 className="text-sm font-semibold text-fg">{footer.quickstart.title}</h2>
          <pre className="mt-3 min-w-0 rounded-sm bg-navy px-3.5 py-3 text-[12px] leading-5 text-[#dfe4ef] whitespace-pre-wrap [overflow-wrap:anywhere]">
            <code>{footer.quickstart.command}</code>
          </pre>
          <p className="mt-2.5 text-sm text-muted">{footer.quickstart.note}</p>
          <a
            href={footer.quickstart.docs.href}
            target="_blank"
            rel="noreferrer"
            className="mt-1 inline-flex items-center gap-0.5 text-sm font-semibold text-brand hover:underline"
          >
            {footer.quickstart.docs.label}
            <ArrowUpRight aria-hidden className="size-3.5" />
          </a>
        </div>
      </div>
      <p className="mx-auto mt-12 max-w-[1120px] border-t border-line-soft pt-6 text-xs text-muted">{footer.legal}</p>
    </footer>
  )
}
