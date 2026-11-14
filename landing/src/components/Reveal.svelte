<script>
  import { reveal } from '../lib/reveal.js'

  let { as = 'div', delay = 0, y = 18, class: className = '', children } = $props()
</script>

{#if as === 'li'}
  <li class={className} use:reveal={{ y, delay }}>{@render children()}</li>
{:else}
  <div class={className} use:reveal={{ y, delay }}>{@render children()}</div>
{/if}
