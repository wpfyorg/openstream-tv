<script>
  import { ArrowLeft, BookOpen } from 'lucide-svelte'

  import Logo from '../components/Logo.svelte'
  import { brand, links } from '../content.js'
  import { buttonClass } from '../lib/ui.js'

  let { path = '/' } = $props()

  const noise = [0, 1, 2, 3, 4].map((i) => ({
    key: i,
    x: 140 + i * 10,
    y: 58 + (i % 3) * 10,
  }))
</script>

<div class="flex min-h-dvh flex-col bg-surface">
  <header class="px-5 py-6 sm:px-9">
    <a href="/" aria-label="{brand.name} home" class="inline-flex min-h-10 items-center rounded-sm">
      <Logo size={16} />
    </a>
  </header>
  <main class="grid flex-1 place-items-center px-5 pb-20 text-center">
    <div class="max-w-md">
      <svg viewBox="0 0 240 150" class="mx-auto w-56" aria-hidden="true">
        <ellipse cx="120" cy="136" rx="98" ry="7" fill="#f3f4f6" />
        <rect x="34" y="26" width="172" height="98" rx="8" fill="#0a0e1c" />
        <rect x="42" y="34" width="156" height="82" rx="4" fill="#141a2e" />
        <g stroke="#3a4363" stroke-width="4" stroke-linecap="round">
          <path d="M64 62h34" />
          <path d="M64 86h58" />
        </g>
        <g fill="#f24e1e" opacity="0.85">
          {#each noise as dot (dot.key)}
            <rect x={dot.x} y={dot.y} width="6" height="6" rx="1" />
          {/each}
        </g>
        <rect x="104" y="124" width="32" height="6" rx="3" fill="#e5e7eb" />
        <text
          x="120"
          y="76"
          text-anchor="middle"
          font-size="12"
          font-weight="700"
          fill="#626977"
          font-family="var(--font-sans)">NO SIGNAL</text
        >
      </svg>
      <p class="mt-6 text-sm font-semibold text-brand">404</p>
      <h1 class="mt-1 text-[32px] leading-10 font-bold tracking-[-0.03em] text-ink">No signal on this channel</h1>
      <p class="mt-2 text-base text-muted">
        Nothing lives at <code class="rounded-xs bg-subtle px-1 py-0.5 text-sm text-body">{path}</code>. It may have moved,
        or the link is wrong.
      </p>
      <div class="mt-7 flex flex-wrap justify-center gap-3">
        <a href="/" class={buttonClass('outline')}>
          <ArrowLeft size={16} aria-hidden="true" />
          Back home
        </a>
        <a href={links.docs} class={buttonClass('primary')}>
          <BookOpen size={16} aria-hidden="true" />
          Documentation
        </a>
      </div>
    </div>
  </main>
</div>
