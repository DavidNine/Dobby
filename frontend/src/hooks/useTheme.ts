// useTheme — owns the active theme, applies it to <html data-theme=…> and
// persists it to localStorage. Called once at the App root so the chosen theme
// is applied on every tab, not just the Settings page.
//
// The theme metadata (label / description / preview colors) lives here too so
// the Settings swatches and the hook share one definition.

import { useCallback, useEffect, useState } from 'react'

export type ThemeId = 'dark' | 'muji' | 'cyberpunk' | 'glass'

export interface ThemeMeta {
  id: ThemeId
  label: string
  description: string
  /** Preview swatch background (a CSS color or gradient). */
  preview: string
  /** Preview accent dot color. */
  accent: string
}

/** Display order in the Settings page. */
export const THEMES: ThemeMeta[] = [
  {
    id: 'dark',
    label: 'Default Dark',
    description: "Dobby's signature slate dark theme.",
    preview: '#0f172a',
    accent: '#3b82f6',
  },
  {
    id: 'muji',
    label: 'Muji Minimalist',
    description: 'Warm off-white with roasted-tea brown accents.',
    preview: '#f4f1ea',
    accent: '#9c6f4a',
  },
  {
    id: 'cyberpunk',
    label: 'Cyberpunk',
    description: 'Pure black with neon green / pink highlights.',
    preview: '#000000',
    accent: '#00ff9f',
  },
  {
    id: 'glass',
    label: 'Glassmorphism',
    description: 'Translucent cards with a frosted blur over a gradient.',
    preview: 'linear-gradient(135deg, #5b21b6 0%, #2563eb 45%, #0ea5e9 100%)',
    accent: '#67e8f9',
  },
]

const STORAGE_KEY = 'dobby-theme'
const THEME_IDS = THEMES.map((t) => t.id)

function isThemeId(value: unknown): value is ThemeId {
  return typeof value === 'string' && (THEME_IDS as string[]).includes(value)
}

/** Read the persisted theme, falling back to 'dark'. */
function readInitialTheme(): ThemeId {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (isThemeId(stored)) return stored
  } catch {
    // localStorage unavailable (private mode / SSR): use the default.
  }
  return 'dark'
}

export interface UseThemeResult {
  theme: ThemeId
  setTheme: (id: ThemeId) => void
}

export function useTheme(): UseThemeResult {
  const [theme, setThemeState] = useState<ThemeId>(readInitialTheme)

  // Apply to <html> and persist whenever the theme changes.
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
    try {
      localStorage.setItem(STORAGE_KEY, theme)
    } catch {
      // Ignore persistence failures; the in-memory theme still applies.
    }
  }, [theme])

  const setTheme = useCallback((id: ThemeId) => setThemeState(id), [])

  return { theme, setTheme }
}
