// useCssVar — read a CSS custom property off <html> and keep it in sync.
//
// Returns the computed value of `name` (e.g. '--color-accent') and re-reads it
// whenever the `data-theme` attribute on <html> changes, via a MutationObserver.
// This lets non-CSS consumers (the Chart.js line color) follow the active theme
// without prop-drilling the theme down from the App root.

import { useEffect, useState } from 'react'

function readVar(name: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim()
  return value || fallback
}

export function useCssVar(name: string, fallback = ''): string {
  const [value, setValue] = useState(() => readVar(name, fallback))

  useEffect(() => {
    const update = () => setValue(readVar(name, fallback))
    // Re-sync once on mount in case the theme attribute was set after the
    // initial render (e.g. by useTheme's effect).
    update()

    const observer = new MutationObserver(update)
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    })
    return () => observer.disconnect()
  }, [name, fallback])

  return value
}
