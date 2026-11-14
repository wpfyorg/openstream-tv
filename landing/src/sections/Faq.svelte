<script>
  import { Plus } from 'lucide-svelte'

  import SectionHeading from '../components/SectionHeading.svelte'
  import { faq } from '../content.js'

  let open = $state(0)
  const id = $props.id()
</script>

<section id="faq" tabindex="-1" class="scroll-mt-20 px-4 py-24 outline-none sm:px-9 sm:py-28">
  <SectionHeading title={faq.title} />
  <div class="mx-auto mt-12 max-w-[760px] divide-y divide-line border-y border-line">
    {#each faq.items as item, i (item.q)}
      {@const isOpen = open === i}
      <div>
        <h3>
          <button
            type="button"
            aria-expanded={isOpen}
            aria-controls="{id}-{i}"
            onclick={() => (open = isOpen ? null : i)}
            class="flex w-full items-center justify-between gap-6 py-5 text-left text-[17px] font-semibold text-ink transition-colors hover:text-brand"
          >
            {item.q}
            <Plus
              aria-hidden="true"
              size={20}
              class="shrink-0 text-muted transition-transform duration-300 {isOpen ? 'rotate-45 text-brand' : ''}"
            />
          </button>
        </h3>
        <div id="{id}-{i}" role="region" class="faq-panel {isOpen ? 'open' : ''}">
          <div>
            <p class="pb-5 text-[15px] leading-6 text-muted">{item.a}</p>
          </div>
        </div>
      </div>
    {/each}
  </div>
</section>
