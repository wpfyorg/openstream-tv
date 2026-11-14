<script>
  import { ArrowUpRight } from 'lucide-svelte'

  import Badge from '../components/Badge.svelte'
  import Reveal from '../components/Reveal.svelte'
  import SectionHeading from '../components/SectionHeading.svelte'
  import { install } from '../content.js'
  import { cardClass } from '../lib/ui.js'
</script>

<section id="install" tabindex="-1" class="scroll-mt-20 bg-canvas px-4 py-24 outline-none sm:px-9 sm:py-28">
  <SectionHeading eyebrow={install.eyebrow} title={install.title} sub={install.sub} />
  <ul class="mx-auto mt-12 grid max-w-[1120px] gap-4 lg:grid-cols-3">
    {#each install.options as option, i (option.name)}
      <Reveal as="li" delay={i * 0.07} class="h-full min-w-0">
        <div class={cardClass({ extra: 'flex h-full min-w-0 flex-col p-6' })}>
          <div class="flex items-center justify-between gap-3">
            <h3 class="text-lg font-bold tracking-[-0.02em] text-ink">{option.name}</h3>
            <Badge tone={i === 0 ? 'brand' : 'slate'}>{option.badge}</Badge>
          </div>
          <p class="mt-2 text-[15px] leading-6 text-muted">{option.body}</p>
          <pre
            class="mt-5 min-w-0 rounded-sm bg-navy px-3.5 py-3 text-[12.5px] leading-5 text-[#dfe4ef] whitespace-pre-wrap [overflow-wrap:anywhere]"><code>{option.command}</code></pre>
        </div>
      </Reveal>
    {/each}
  </ul>
  <p class="mx-auto mt-8 max-w-[720px] text-center text-sm text-muted">
    {install.note}{' '}
    <a href={install.docs.href} class="inline-flex items-center gap-0.5 font-semibold text-brand hover:underline">
      {install.docs.label}
      <ArrowUpRight size={14} aria-hidden="true" />
    </a>
  </p>
</section>
