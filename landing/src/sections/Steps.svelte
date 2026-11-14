<script>
  import Reveal from '../components/Reveal.svelte'
  import SectionHeading from '../components/SectionHeading.svelte'
  import { steps } from '../content.js'
</script>

<section id="how" tabindex="-1" class="scroll-mt-20 bg-canvas px-4 py-24 outline-none sm:px-9 sm:py-28">
  <SectionHeading eyebrow={steps.eyebrow} title={steps.title} />
  <div class="relative mx-auto mt-14 max-w-[1020px]">
    <span
      aria-hidden="true"
      class="absolute top-6 right-[16%] left-[16%] hidden h-px bg-[repeating-linear-gradient(90deg,var(--color-line)_0_6px,transparent_6px_12px)] md:block"
    ></span>
    <ol class="relative grid gap-8 md:grid-cols-3 md:gap-6">
      {#each steps.items as step, i (step.title)}
        <Reveal as="li" delay={i * 0.12} class="relative text-center">
          <span
            class="relative mx-auto grid size-12 place-items-center rounded-full bg-surface text-lg font-bold text-brand shadow-soft ring-1 ring-line"
          >
            {i + 1}
          </span>
          <h3 class="mt-5 text-lg font-bold tracking-[-0.02em] text-ink">{step.title}</h3>
          <p class="mx-auto mt-1.5 max-w-[280px] text-[15px] leading-6 text-muted">{step.body}</p>
        </Reveal>
      {/each}
    </ol>
  </div>
</section>
