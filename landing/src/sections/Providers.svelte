<script>
  import Badge from '../components/Badge.svelte'
  import Reveal from '../components/Reveal.svelte'
  import SectionHeading from '../components/SectionHeading.svelte'
  import { providers } from '../content.js'
  import { cardClass } from '../lib/ui.js'
</script>

<section id="providers" tabindex="-1" class="scroll-mt-20 px-4 py-24 outline-none sm:px-9 sm:py-28">
  <SectionHeading eyebrow={providers.eyebrow} title={providers.title} sub={providers.sub} />
  <ul class="mx-auto mt-14 grid max-w-[1120px] gap-4 sm:grid-cols-2 lg:grid-cols-3">
    {#each providers.items as provider, i (provider.name)}
      <Reveal as="li" delay={(i % 3) * 0.07} class="h-full">
        <div class={cardClass({ rule: provider.status === 'Stable', interactive: true, extra: 'flex h-full flex-col p-6' })}>
          <div class="flex items-center justify-between gap-3">
            <h3 class="text-lg font-bold tracking-[-0.02em] text-ink">{provider.name}</h3>
            <Badge tone={provider.status === 'Stable' ? 'free' : 'amber'} dot>{provider.status}</Badge>
          </div>
          <p class="mt-2.5 text-[15px] leading-6 text-muted">{provider.body}</p>
        </div>
      </Reveal>
    {/each}
  </ul>
</section>
