<script>
  import Reveal from '../components/Reveal.svelte'
  import { logos, stats } from '../content.js'
</script>

<!-- Continues the navy band under the mock: provider wordmarks and numbers. -->
<section aria-label="Supported providers" class="bg-navy px-4 pt-16 pb-20 text-white sm:px-9 sm:pt-20 sm:pb-24">
  <div class="mx-auto max-w-[1018px]">
    <p class="text-center text-sm font-medium text-[#b4bacb]">JioTV, JioTV+, Fastway and four beta integrations</p>
    <ul class="mt-8 grid grid-cols-2 items-center gap-x-6 gap-y-6 sm:grid-cols-3 lg:grid-cols-6">
      {#each logos as logo, i (logo)}
        <li
          class="text-center text-[19px] font-bold tracking-[-0.03em] text-[#9aa3b8] transition-colors duration-200 hover:text-white"
          style="{i % 3 === 1 ? 'font-style: italic;' : ''}{i % 3 === 2 ? 'letter-spacing: 0.08em;' : ''}"
        >
          {i % 3 === 2 ? logo.toUpperCase() : logo}
        </li>
      {/each}
    </ul>
    <div class="mt-16 grid grid-cols-2 gap-px overflow-hidden rounded-md bg-navy-line lg:grid-cols-4">
      {#each stats as stat, i (stat.label)}
        <Reveal delay={i * 0.06} class="bg-navy-2 px-6 py-7">
          <p class="text-[28px] leading-10 font-bold tracking-[-0.03em] text-white">{stat.value}</p>
          <p class="mt-1 text-sm text-[#b4bacb]">{stat.label}</p>
        </Reveal>
      {/each}
    </div>
  </div>
</section>
