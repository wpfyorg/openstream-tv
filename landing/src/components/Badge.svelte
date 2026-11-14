<script>
  import { badgeClass } from '../lib/ui.js'

  let { tone = 'slate', dot = false, class: className = '', children } = $props()
</script>

<span class={badgeClass(tone, className)}>
  {#if dot}<span aria-hidden="true" class="size-1.5 rounded-full bg-current"></span>{/if}
  {@render children()}
</span>
