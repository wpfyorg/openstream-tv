import { ArrowUpRight } from 'lucide-react'

import { Badge } from '../components/Badge'
import { Card } from '../components/Card'
import { Reveal } from '../components/Reveal'
import { SectionHeading } from '../components/SectionHeading'
import { install } from '../content'

/** Installation options, replacing the template's pricing table. */
export function Install() {
  return (
    <section id="install" tabIndex={-1} className="scroll-mt-20 bg-canvas px-4 py-24 outline-none sm:px-9 sm:py-28">
      <SectionHeading eyebrow={install.eyebrow} title={install.title} sub={install.sub} />
      <ul className="mx-auto mt-12 grid max-w-[1120px] gap-4 lg:grid-cols-3">
        {install.options.map((option, i) => (
          <Reveal as="li" key={option.name} delay={i * 0.07} className="h-full min-w-0">
            <Card className="flex h-full min-w-0 flex-col p-6">
              <div className="flex items-center justify-between gap-3">
                <h3 className="text-lg font-bold tracking-[-0.02em] text-ink">{option.name}</h3>
                <Badge tone={i === 0 ? 'brand' : 'slate'}>{option.badge}</Badge>
              </div>
              <p className="mt-2 text-[15px] leading-6 text-muted">{option.body}</p>
              <pre className="mt-5 min-w-0 rounded-sm bg-navy px-3.5 py-3 text-[12.5px] leading-5 text-[#dfe4ef] whitespace-pre-wrap [overflow-wrap:anywhere]">
                <code>{option.command}</code>
              </pre>
            </Card>
          </Reveal>
        ))}
      </ul>
      <p className="mx-auto mt-8 max-w-[720px] text-center text-sm text-muted">
        {install.note}{' '}
        <a href={install.docs.href} className="inline-flex items-center gap-0.5 font-semibold text-brand hover:underline">
          {install.docs.label}
          <ArrowUpRight aria-hidden className="size-3.5" />
        </a>
      </p>
    </section>
  )
}
