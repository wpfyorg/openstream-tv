import { Reveal } from '../components/Reveal'
import { SectionHeading } from '../components/SectionHeading'
import { steps } from '../content'

export function Steps() {
  return (
    <section id="how" tabIndex={-1} className="scroll-mt-20 bg-canvas px-4 py-24 outline-none sm:px-9 sm:py-28">
      <SectionHeading eyebrow={steps.eyebrow} title={steps.title} />
      <div className="relative mx-auto mt-14 max-w-[1020px]">
        <span aria-hidden className="absolute top-6 right-[16%] left-[16%] hidden h-px bg-[repeating-linear-gradient(90deg,var(--color-line)_0_6px,transparent_6px_12px)] md:block" />
        <ol className="relative grid gap-8 md:grid-cols-3 md:gap-6">
        {steps.items.map((s, i) => (
          <Reveal as="li" key={s.title} delay={i * 0.12} className="relative text-center">
              <span className="relative mx-auto grid size-12 place-items-center rounded-full bg-surface text-lg font-bold text-brand shadow-soft ring-1 ring-line">{i + 1}</span>
              <h3 className="mt-5 text-lg font-bold tracking-[-0.02em] text-ink">{s.title}</h3>
              <p className="mx-auto mt-1.5 max-w-[280px] text-[15px] leading-6 text-muted">{s.body}</p>
            </Reveal>
        ))}
      </ol>
      </div>
    </section>
  )
}
