<script>
  import Logo from '../components/Logo.svelte'
  import { nav } from '../content.js'
  import { buttonClass } from '../lib/ui.js'

  let scrolled = $state(false)
</script>

<svelte:window onscroll={() => (scrolled = window.scrollY > 8)} />

<header
  class="sticky top-0 z-40 transition-[background-color,box-shadow] duration-300 {scrolled
    ? 'bg-surface/90 shadow-[0_1px_0_var(--color-line)] backdrop-blur'
    : 'bg-surface'}"
>
  <a
    href="#main"
    class="sr-only focus:not-sr-only focus:absolute focus:top-3 focus:left-3 focus:z-50 focus:rounded-sm focus:bg-surface focus:px-3 focus:py-2 focus:text-sm focus:shadow-pop"
  >
    Skip to content
  </a>
  <div class="mx-auto flex h-[72px] max-w-[1440px] items-center gap-6 px-4 sm:px-9">
    <a href="/" aria-label="OpenStream TV home" class="inline-flex min-h-10 items-center rounded-sm">
      <Logo size={16} />
    </a>
    <nav aria-label="Sections" class="ml-6 hidden items-center gap-6 lg:flex">
      {#each nav.links as link (link.href)}
        <a href={link.href} class="text-[15px] font-medium text-muted transition-colors hover:text-fg">{link.label}</a>
      {/each}
    </nav>
    <div class="ml-auto flex items-center gap-2 sm:gap-[27px]">
      <a
        href={nav.secondary.href}
        class="hidden h-10 items-center rounded-sm px-2 text-[15px] font-medium text-fg transition-colors hover:text-brand sm:inline-flex"
      >
        {nav.secondary.label}
      </a>
      <a
        href={nav.cta.href}
        class={buttonClass('soft', 'md', 'h-[38px] rounded-[3px] px-[14px] text-[15px] font-medium sm:px-[14px]')}
      >
        {nav.cta.label}
      </a>
    </div>
  </div>
</header>
