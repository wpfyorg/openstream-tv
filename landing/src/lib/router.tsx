import { useEffect, useSyncExternalStore, type AnchorHTMLAttributes, type MouseEvent } from 'react'

/*
  A tiny client-side router: history.pushState plus a popstate listener.
  Enough for a handful of pages; swap in a real router if the site grows.
*/

const listeners = new Set<() => void>()

function subscribe(listener: () => void) {
  listeners.add(listener)
  window.addEventListener('popstate', listener)
  return () => {
    listeners.delete(listener)
    window.removeEventListener('popstate', listener)
  }
}

export function navigate(to: string, { replace = false } = {}) {
  const url = new URL(to, window.location.href)
  if (url.origin !== window.location.origin) {
    window.location.assign(url)
    return
  }
  const same = url.pathname === window.location.pathname
  if (replace) window.history.replaceState(null, '', url)
  else window.history.pushState(null, '', url)
  listeners.forEach((l) => l())
  if (url.hash) {
    // Wait a frame so the new page has rendered its anchors.
    requestAnimationFrame(() => scrollToHash(url.hash))
  } else if (!same) {
    window.scrollTo(0, 0)
  }
}

/** Scrolls to `#id` (and focuses it when the target opts in with tabIndex). */
export function scrollToHash(hash: string) {
  if (!hash || hash === '#') return
  const target = document.getElementById(decodeURIComponent(hash.slice(1)))
  if (!target) return
  target.scrollIntoView()
  if (target.hasAttribute('tabindex')) target.focus({ preventScroll: true })
}

export function usePath(): string {
  return useSyncExternalStore(subscribe, () => window.location.pathname, () => '/')
}

export function useSearch(): string {
  return useSyncExternalStore(subscribe, () => window.location.search, () => '')
}

/** Sets document.title while a page is mounted. */
export function useTitle(title: string) {
  useEffect(() => {
    const previous = document.title
    document.title = title
    return () => {
      document.title = previous
    }
  }, [title])
}

type LinkProps = AnchorHTMLAttributes<HTMLAnchorElement> & { href: string }

/** An <a> that navigates in-app for same-origin paths. */
export function Link({ href, onClick, target, ...rest }: LinkProps) {
  function handle(event: MouseEvent<HTMLAnchorElement>) {
    onClick?.(event)
    if (
      event.defaultPrevented ||
      event.button !== 0 ||
      event.metaKey ||
      event.ctrlKey ||
      event.shiftKey ||
      event.altKey ||
      target === '_blank' ||
      !href.startsWith('/')
    ) {
      return
    }
    event.preventDefault()
    navigate(href)
  }
  return <a href={href} target={target} onClick={handle} {...rest} />
}
