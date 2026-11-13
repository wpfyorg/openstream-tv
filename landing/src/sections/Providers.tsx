import { Badge } from '../components/Badge'
import { Card } from '../components/Card'
import { Reveal } from '../components/Reveal'
import { SectionHeading } from '../components/SectionHeading'
import { providers } from '../content'

/** Provider support, replacing the template's testimonials. */
export function Providers() {
  return (
    <section id="providers" tabIndex={-1} className="scroll-mt-20 px-4 py-24 outline-none sm:px-9 sm:py-28">
      <SectionHeading eyebrow={providers.eyebrow} title={providers.title} sub={providers.sub} />
      <ul className="mx-auto mt-14 grid max-w-[1120px] gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {providers.items.map((p, i) => (
          <Reveal as="li" key={p.name} delay={(i % 3) * 0.07} className="h-full">
            <Card rule={p.status === 'Stable'} interactive className="flex h-full flex-col p-6">
              <div className="flex items-center justify-between gap-3">
                <h3 className="text-lg font-bold tracking-[-0.02em] text-ink">{p.name}</h3>
                <Badge tone={p.status === 'Stable' ? 'free' : 'amber'} dot>
                  {p.status}
                </Badge>
              </div>
              <p className="mt-2.5 text-[15px] leading-6 text-muted">{p.body}</p>
            </Card>
          </Reveal>
        ))}
      </ul>
    </section>
  )
}
