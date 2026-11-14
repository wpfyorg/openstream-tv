<script>
  import { CalendarClock, ListVideo, Monitor, Server, ShieldCheck, Tv } from 'lucide-svelte'

  import Reveal from '../components/Reveal.svelte'
  import SectionHeading from '../components/SectionHeading.svelte'
  import { features } from '../content.js'
  import { cardClass, iconChipClass } from '../lib/ui.js'

  const icons = { tv: Tv, guide: CalendarClock, playlist: ListVideo, shield: ShieldCheck, monitor: Monitor, server: Server }
</script>

<section id="features" tabindex="-1" class="scroll-mt-20 px-4 py-24 outline-none sm:px-9 sm:py-28">
  <SectionHeading eyebrow={features.eyebrow} title={features.title} sub={features.sub} />
  <ul class="mx-auto mt-14 grid max-w-[1120px] gap-4 sm:grid-cols-2 lg:grid-cols-3">
    {#each features.items as feature, i (feature.title)}
      {@const Icon = icons[feature.icon]}
      <Reveal as="li" delay={(i % 3) * 0.07} class="h-full">
        <div class={cardClass({ interactive: true, extra: 'group h-full p-6 hover:-translate-y-0.5 transition-transform duration-200' })}>
          <span class={iconChipClass(feature.tone, 'lg')}>
            <Icon class="transition-transform duration-300 group-hover:scale-110" />
          </span>
          <h3 class="mt-5 text-lg font-bold tracking-[-0.02em] text-ink">{feature.title}</h3>
          <p class="mt-1.5 text-[15px] leading-6 text-muted">{feature.body}</p>
        </div>
      </Reveal>
    {/each}
  </ul>
</section>
