import { ArrowRight, Star } from 'lucide-react'

import { ButtonLink } from '../components/Button'
import { Reveal } from '../components/Reveal'
import { cta } from '../content'

export function Cta() {
  return (
    <section className="px-4 pb-24 sm:px-9">
      <Reveal className="relative mx-auto max-w-[1120px] overflow-hidden rounded-xl bg-navy px-6 py-16 text-center sm:px-12">
        <svg aria-hidden className="absolute inset-0 h-full w-full opacity-[0.18]" preserveAspectRatio="none" viewBox="0 0 400 200">
          {Array.from({ length: 6 }, (_, r) =>
            Array.from({ length: 12 }, (_, c) => (
              <rect key={`${r}-${c}`} x={c * 34 + 4} y={r * 34 + 6} width="26" height="18" rx="3" fill={(r * 7 + c * 3) % 5 === 0 ? '#f24e1e' : '#3a4363'} />
            )),
          )}
        </svg>
        <div className="relative">
          <h2 className="text-[30px] leading-[38px] font-bold tracking-[-0.035em] text-white sm:text-h2 sm:leading-[48px] text-balance">{cta.title}</h2>
          <p className="mt-3 text-[17px] text-[#c4c9d6]">{cta.sub}</p>
          <div className="mt-8 flex flex-wrap items-center justify-center gap-3">
            <ButtonLink href={cta.href} size="lg" iconRight={<ArrowRight className="size-[18px]" aria-hidden />}>
              {cta.button}
            </ButtonLink>
            <ButtonLink
              href={cta.secondary.href}
              variant="outline"
              size="lg"
              className="bg-transparent text-white ring-white/25 hover:bg-white/10 hover:ring-white/40"
              icon={<Star className="size-[18px]" aria-hidden />}
            >
              {cta.secondary.label}
            </ButtonLink>
          </div>
        </div>
      </Reveal>
    </section>
  )
}
