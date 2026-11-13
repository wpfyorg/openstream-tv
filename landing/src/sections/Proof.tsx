import { Reveal } from '../components/Reveal'
import { logos, stats } from '../content'

/** Continues the navy band under the mock: provider wordmarks and numbers. */
export function Proof() {
  return (
    <section aria-label="Supported providers" className="bg-navy px-4 pt-16 pb-20 text-white sm:px-9 sm:pt-20 sm:pb-24">
      <div className="mx-auto max-w-[1018px]">
        <p className="text-center text-sm font-medium text-[#b4bacb]">JioTV, JioTV+, Fastway and four beta integrations</p>
        <ul className="mt-8 grid grid-cols-2 items-center gap-x-6 gap-y-6 sm:grid-cols-3 lg:grid-cols-6">
          {logos.map((l, i) => (
            <li key={l} className="text-center text-[19px] font-bold tracking-[-0.03em] text-[#9aa3b8] transition-colors duration-200 hover:text-white" style={{ fontStyle: i % 3 === 1 ? 'italic' : undefined, letterSpacing: i % 3 === 2 ? '0.08em' : undefined }}>
              {i % 3 === 2 ? l.toUpperCase() : l}
            </li>
          ))}
        </ul>
        <div className="mt-16 grid grid-cols-2 gap-px overflow-hidden rounded-md bg-navy-line lg:grid-cols-4">
          {stats.map((s, i) => (
            <Reveal key={s.label} delay={i * 0.06} className="bg-navy-2 px-6 py-7">
              <p className="text-[28px] leading-10 font-bold tracking-[-0.03em] text-white">{s.value}</p>
              <p className="mt-1 text-sm text-[#b4bacb]">{s.label}</p>
            </Reveal>
          ))}
        </div>
      </div>
    </section>
  )
}
