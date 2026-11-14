/*
  `use:reveal` fades an element up the first time it scrolls into view.
  The observer disconnects after the first intersection, so long pages do
  not keep watching elements that have already appeared.
*/
export function reveal(node, { y = 18, delay = 0 } = {}) {
  node.classList.add('reveal')
  node.style.setProperty('--reveal-y', `${y}px`)
  node.style.setProperty('--reveal-delay', `${delay}s`)

  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)')
  if (reduce.matches || !('IntersectionObserver' in window)) {
    node.classList.add('is-in')
    return {}
  }

  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue
        entry.target.classList.add('is-in')
        observer.disconnect()
      }
    },
    { rootMargin: '0px 0px -8% 0px' },
  )
  observer.observe(node)

  return {
    destroy() {
      observer.disconnect()
    },
  }
}
