<script>
  import { ArrowRight, Star } from 'lucide-svelte'

  import Reveal from '../components/Reveal.svelte'
  import { cta } from '../content.js'
  import { buttonClass } from '../lib/ui.js'

  // Decorative channel grid behind the copy.
  const cells = Array.from({ length: 6 }, (_, row) =>
    Array.from({ length: 12 }, (_, col) => ({
      key: `${row}-${col}`,
      x: col * 34 + 4,
      y: row * 34 + 6,
      fill: (row * 7 + col * 3) % 5 === 0 ? '#f24e1e' : '#3a4363',
    })),
  ).flat()
</script>

<section class="px-4 pb-24 sm:px-9">
  <Reveal class="relative mx-auto max-w-[1120px] overflow-hidden rounded-xl bg-navy px-6 py-16 text-center sm:px-12">
    <svg aria-hidden="true" class="absolute inset-0 h-full w-full opacity-[0.18]" preserveAspectRatio="none" viewBox="0 0 400 200">
      {#each cells as cell (cell.key)}
        <rect x={cell.x} y={cell.y} width="26" height="18" rx="3" fill={cell.fill} />
      {/each}
    </svg>
    <div class="relative">
      <h2 class="text-[30px] leading-[38px] font-bold tracking-[-0.035em] text-white sm:text-h2 sm:leading-[48px] text-balance">
        {cta.title}
      </h2>
      <p class="mt-3 text-[17px] text-[#c4c9d6]">{cta.sub}</p>
      <div class="mt-8 flex flex-wrap items-center justify-center gap-3">
        <a href={cta.href} class={buttonClass('primary', 'lg')}>
          {cta.button}
          <ArrowRight size={18} aria-hidden="true" />
        </a>
        <a
          href={cta.secondary.href}
          class={buttonClass('outline', 'lg', 'bg-transparent text-white ring-white/25 hover:bg-white/10 hover:ring-white/40')}
        >
          <Star size={18} aria-hidden="true" />
          {cta.secondary.label}
        </a>
      </div>
    </div>
  </Reveal>
</section>
