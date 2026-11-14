<script>
  import { onMount } from 'svelte'

  import { brand } from './content.js'
  import Landing from './pages/Landing.svelte'
  import NotFound from './pages/NotFound.svelte'

  // Routes: / (landing) and a friendly 404. Everything lives on one page.
  let path = $state('/')
  let title = $derived(path === '/' ? `${brand.name}: ${brand.tagline}` : `Page not found · ${brand.name}`)

  onMount(() => {
    path = window.location.pathname
    const sync = () => {
      path = window.location.pathname
    }
    window.addEventListener('popstate', sync)

    // Internal links (the logo, "Back home") swap the page in place instead of
    // reloading; in-page anchors keep their native scrolling.
    const onClick = (event) => {
      if (event.defaultPrevented || event.button !== 0) return
      if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return
      const link = event.target instanceof Element ? event.target.closest('a[href]') : null
      if (!link || link.target || link.hasAttribute('download')) return
      const url = new URL(link.href, window.location.href)
      if (url.origin !== window.location.origin) return
      if (url.pathname === window.location.pathname) return
      event.preventDefault()
      history.pushState({}, '', url.pathname + url.search + url.hash)
      path = url.pathname
    }
    document.addEventListener('click', onClick)

    return () => {
      window.removeEventListener('popstate', sync)
      document.removeEventListener('click', onClick)
    }
  })

  $effect(() => {
    document.title = title
  })
</script>

{#if path === '/'}
  <Landing />
{:else}
  <NotFound {path} />
{/if}
