import { motion, useReducedMotion } from 'motion/react'
import type { ReactNode } from 'react'

import { duration, ease } from '../lib/motion'

/** Fade-and-rise on scroll. Stagger siblings with `delay`; `as="li"` inside lists. */
export function Reveal({ children, delay = 0, className, y = 18, as = 'div' }: { children: ReactNode; delay?: number; className?: string; y?: number; as?: 'div' | 'li' }) {
  const reduce = useReducedMotion()
  const Comp = as === 'li' ? motion.li : motion.div
  return (
    <Comp
      className={className}
      initial={reduce ? false : { opacity: 0, y }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '0px 0px -8% 0px' }}
      transition={{ duration: duration.reveal, delay, ease }}
    >
      {children}
    </Comp>
  )
}
