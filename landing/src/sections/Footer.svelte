<script>
  import { ArrowUpRight } from 'lucide-svelte'

  import Logo from '../components/Logo.svelte'
  import { footer } from '../content.js'
</script>

<footer class="border-t border-line px-4 pt-14 pb-10 sm:px-9">
  <div class="mx-auto grid max-w-[1120px] gap-10 md:grid-cols-[1.4fr_1fr_1fr_1.6fr]">
    <div>
      <Logo size={16} />
      <p class="mt-3 max-w-[240px] text-sm text-muted">{footer.blurb}</p>
    </div>
    {#each footer.columns as column (column.title)}
      <nav aria-label={column.title}>
        <h2 class="text-sm font-semibold text-fg">{column.title}</h2>
        <ul class="mt-2 grid">
          {#each column.links as link (link.label)}
            <li>
              <a
                href={link.href}
                class="inline-flex h-10 items-center gap-1 text-sm text-muted hover:text-fg"
                target={link.href.startsWith('#') ? undefined : '_blank'}
                rel={link.href.startsWith('#') ? undefined : 'noreferrer'}
              >
                {link.label}
                {#if !link.href.startsWith('#')}<ArrowUpRight size={14} aria-hidden="true" class="text-faint" />{/if}
              </a>
            </li>
          {/each}
        </ul>
      </nav>
    {/each}
    <div class="min-w-0">
      <h2 class="text-sm font-semibold text-fg">{footer.quickstart.title}</h2>
      <pre
        class="mt-3 min-w-0 rounded-sm bg-navy px-3.5 py-3 text-[12px] leading-5 text-[#dfe4ef] whitespace-pre-wrap [overflow-wrap:anywhere]"><code>{footer.quickstart.command}</code></pre>
      <p class="mt-2.5 text-sm text-muted">{footer.quickstart.note}</p>
      <a
        href={footer.quickstart.docs.href}
        target="_blank"
        rel="noreferrer"
        class="mt-1 inline-flex items-center gap-0.5 text-sm font-semibold text-brand hover:underline"
      >
        {footer.quickstart.docs.label}
        <ArrowUpRight size={14} aria-hidden="true" />
      </a>
    </div>
  </div>
  <p class="mx-auto mt-12 max-w-[1120px] border-t border-line-soft pt-6 text-xs text-muted">{footer.legal}</p>
</footer>
