import { CalendarClock, ListVideo, Monitor, Server, ShieldCheck, Tv } from 'lucide-react'

import { IconChip } from '../components/Badge'
import { Card } from '../components/Card'
import { Reveal } from '../components/Reveal'
import { SectionHeading } from '../components/SectionHeading'
import { features } from '../content'

const icons = { tv: Tv, guide: CalendarClock, playlist: ListVideo, shield: ShieldCheck, monitor: Monitor, server: Server }

export function Features() {
  return (
    <section id="features" tabIndex={-1} className="scroll-mt-20 px-4 py-24 outline-none sm:px-9 sm:py-28">
      <SectionHeading eyebrow={features.eyebrow} title={features.title} sub={features.sub} />
      <ul className="mx-auto mt-14 grid max-w-[1120px] gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {features.items.map((f, i) => {
          const Icon = icons[f.icon]
          return (
            <Reveal as="li" key={f.title} delay={(i % 3) * 0.07} className="h-full">
              <Card interactive className="group h-full p-6 hover:-translate-y-0.5 transition-transform duration-200">
                <IconChip tone={f.tone} size="lg">
                  <Icon className="transition-transform duration-300 group-hover:scale-110" />
                </IconChip>
                <h3 className="mt-5 text-lg font-bold tracking-[-0.02em] text-ink">{f.title}</h3>
                <p className="mt-1.5 text-[15px] leading-6 text-muted">{f.body}</p>
              </Card>
            </Reveal>
          )
        })}
      </ul>
    </section>
  )
}
