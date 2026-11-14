<script>
  import Reveal from './Reveal.svelte'

  let { eyebrow, title, sub, dark = false, class: className = '' } = $props()
</script>

<Reveal class="mx-auto max-w-[640px] text-center {className}">
  {#if eyebrow}
    <p class="text-sm font-semibold {dark ? 'text-[#ff8a63]' : 'text-brand'}">{eyebrow}</p>
  {/if}
  <h2
    class="mt-2 text-[30px] leading-[38px] font-bold tracking-[-0.035em] sm:text-h2 sm:leading-[48px] text-balance {dark
      ? 'text-white'
      : 'text-ink'}"
  >
    {title}
  </h2>
  {#if sub}
    <p class="mt-4 text-[17px] leading-7 text-pretty {dark ? 'text-[#b4bacb]' : 'text-muted'}">{sub}</p>
  {/if}
</Reveal>
