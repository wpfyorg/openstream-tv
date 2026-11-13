import { motion, useReducedMotion } from 'motion/react'
import { ArrowRight, BookOpen } from 'lucide-react'

import { ButtonLink } from '../components/Button'
import { hero } from '../content'
import { duration, ease } from '../lib/motion'
import { HeroMock } from './HeroMock'

export function Hero() {
  const reduce = useReducedMotion()
  const rise = (delay: number) => ({
    initial: reduce ? false : { opacity: 0, y: 16 },
    animate: { opacity: 1, y: 0 },
    transition: { duration: duration.reveal, delay, ease },
  })
  return (
    <section className="relative overflow-hidden">
      <div className="mx-auto max-w-[1440px] px-4 pt-[52px] text-center sm:px-9 sm:pt-[107px]">
        <motion.h1 {...rise(0)} className="text-[44px] leading-[1.05] font-bold tracking-[-0.04em] text-ink sm:text-[60px] lg:text-[72px] lg:leading-[88px]">
          {hero.headline}{' '}
          <span className="relative ml-[0.3em] inline-block whitespace-nowrap">
            <motion.span
              aria-hidden
              className="absolute -inset-x-[0.27em] top-[0.02em] -bottom-[0.04em] origin-left rounded-[7px] bg-brand-soft"
              initial={reduce ? false : { scaleX: 0 }}
              animate={{ scaleX: 1 }}
              transition={{ duration: 0.7, delay: 0.25, ease }}
            />
            <span className="relative text-brand-bright">{hero.highlight}</span>
          </span>
        </motion.h1>
        <motion.p {...rise(0.1)} className="mx-auto mt-[38px] max-w-[660px] text-[17px] leading-7 text-muted sm:text-lead sm:leading-[32px] lg:text-[21px] lg:leading-[32px] text-pretty">
          {hero.sub}
        </motion.p>
        <motion.div {...rise(0.18)} className="mt-[42px] flex flex-wrap items-center justify-center gap-3">
          <ButtonLink
            href={hero.cta.href}
            size="lg"
            className="h-[45px] w-[248px] rounded-[2px] text-[17px]"
            iconRight={<ArrowRight className="size-[18px] transition-transform duration-200 group-hover:translate-x-0.5" aria-hidden />}
          >
            {hero.cta.label}
          </ButtonLink>
          <ButtonLink
            href={hero.secondary.href}
            variant="outline"
            size="lg"
            className="h-[45px] w-[248px] rounded-[2px] text-[17px]"
            icon={<BookOpen className="size-[18px]" aria-hidden />}
          >
            {hero.secondary.label}
          </ButtonLink>
        </motion.div>
      </div>
      <div className="relative mt-[59px] px-4 sm:px-9">
        <div aria-hidden className="absolute inset-x-0 top-[126px] bottom-0 bg-navy" />
        <HeroMock />
      </div>
    </section>
  )
}
