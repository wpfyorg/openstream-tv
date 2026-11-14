<script>
  import { onMount } from 'svelte'
  import { ArrowRight, BookOpen } from 'lucide-svelte'

  import DiscordIcon from '../components/DiscordIcon.svelte'
  import { hero } from '../content.js'
  import { buttonClass } from '../lib/ui.js'
  import HeroMock from './HeroMock.svelte'

  let barIn = $state(false)

  // The highlight bar needs one painted frame at scaleX(0) before the class
  // change can transition it in.
  onMount(() => {
    const frame = requestAnimationFrame(() => (barIn = true))
    return () => cancelAnimationFrame(frame)
  })
</script>

<section class="relative overflow-hidden">
  <div class="mx-auto max-w-[1440px] px-4 pt-[52px] text-center sm:px-9 sm:pt-[107px]">
    <h1
      class="rise text-[44px] leading-[1.05] font-bold tracking-[-0.04em] text-ink sm:text-[60px] lg:text-[72px] lg:leading-[88px]"
    >
      {hero.headline}{' '}
      <span class="relative ml-[0.3em] inline-block whitespace-nowrap">
        <span
          aria-hidden="true"
          class="highlight-bar absolute -inset-x-[0.27em] top-[0.02em] -bottom-[0.04em] rounded-[7px] bg-brand-soft {barIn
            ? 'is-in'
            : ''}"
        ></span>
        <span class="relative text-brand-bright">{hero.highlight}</span>
      </span>
    </h1>
    <p
      class="rise mx-auto mt-[38px] max-w-[660px] text-[17px] leading-7 text-muted sm:text-lead sm:leading-[32px] lg:text-[21px] lg:leading-[32px] text-pretty"
      style="--rise-delay: 0.1s"
    >
      {hero.sub}
    </p>
    <div class="rise mt-[42px] flex flex-wrap items-center justify-center gap-3" style="--rise-delay: 0.18s">
      <a href={hero.cta.href} class={buttonClass('primary', 'lg', 'h-[45px] w-[248px] rounded-[2px] text-[17px]')}>
        {hero.cta.label}
        <ArrowRight size={18} aria-hidden="true" />
      </a>
      <a href={hero.secondary.href} class={buttonClass('outline', 'lg', 'h-[45px] w-[248px] rounded-[2px] text-[17px]')}>
        <BookOpen size={18} aria-hidden="true" />
        {hero.secondary.label}
      </a>
      <a
        href={hero.discord.href}
        target="_blank"
        rel="noreferrer"
        class="inline-flex h-[45px] items-center gap-2 px-3 text-[17px] font-semibold text-muted transition-colors hover:text-fg"
      >
        <DiscordIcon size={20} class="text-[#5865F2]" />
        {hero.discord.label}
      </a>
    </div>
  </div>
  <div class="relative mt-[59px] px-4 sm:px-9">
    <div aria-hidden="true" class="absolute inset-x-0 top-[126px] bottom-0 bg-navy"></div>
    <HeroMock />
  </div>
</section>
