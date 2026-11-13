import { AnimatePresence, motion } from 'motion/react'
import { useId, useState } from 'react'
import { Plus } from 'lucide-react'

import { SectionHeading } from '../components/SectionHeading'
import { faq } from '../content'
import { cn } from '../lib/cn'
import { duration, ease } from '../lib/motion'

export function Faq() {
  const [open, setOpen] = useState<number | null>(0)
  const id = useId()
  return (
    <section id="faq" tabIndex={-1} className="scroll-mt-20 px-4 py-24 outline-none sm:px-9 sm:py-28">
      <SectionHeading title={faq.title} />
      <div className="mx-auto mt-12 max-w-[760px] divide-y divide-line border-y border-line">
        {faq.items.map((item, i) => {
          const isOpen = open === i
          return (
            <div key={item.q}>
              <h3>
                <button
                  type="button"
                  aria-expanded={isOpen}
                  aria-controls={`${id}-${i}`}
                  onClick={() => setOpen(isOpen ? null : i)}
                  className="flex w-full items-center justify-between gap-6 py-5 text-left text-[17px] font-semibold text-ink transition-colors hover:text-brand"
                >
                  {item.q}
                  <Plus aria-hidden className={cn('size-5 shrink-0 text-muted transition-transform duration-300', isOpen && 'rotate-45 text-brand')} />
                </button>
              </h3>
              <AnimatePresence initial={false}>
                {isOpen && (
                  <motion.div
                    id={`${id}-${i}`}
                    role="region"
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: 'auto', opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    transition={{ duration: duration.base, ease }}
                    className="overflow-hidden"
                  >
                    <p className="pb-5 text-[15px] leading-6 text-muted">{item.a}</p>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          )
        })}
      </div>
    </section>
  )
}
