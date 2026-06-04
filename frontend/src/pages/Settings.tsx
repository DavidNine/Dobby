// Settings — view behind the "Settings" sidebar tab.
//
// Currently hosts the Theme Settings section: four circular theme swatches that
// switch the app theme live. The active theme is owned by the App root
// (useTheme) and passed in, so selection here updates the whole app.

import type { ThemeId } from '../hooks/useTheme'
import { THEMES } from '../hooks/useTheme'

export interface SettingsProps {
  theme: ThemeId
  onChange: (id: ThemeId) => void
}

export default function Settings({ theme, onChange }: SettingsProps) {
  return (
    <div className="min-h-screen bg-background text-main">
      <div className="mx-auto max-w-6xl px-4 py-6 sm:px-6 lg:px-8">
        <header className="mb-6">
          <h1 className="text-2xl font-semibold">Settings</h1>
        </header>

        <section
          aria-labelledby="theme-settings-heading"
          className="glass-surface rounded-xl border border-border bg-card p-6 shadow-sm"
        >
          <h2 id="theme-settings-heading" className="text-lg font-semibold">
            Theme Settings
          </h2>
          <p className="mt-1 text-sm text-muted">
            Pick a look for Dobby. Your choice is saved on this device.
          </p>

          <div
            role="radiogroup"
            aria-label="Theme"
            className="mt-6 grid grid-cols-2 gap-4 sm:grid-cols-4"
          >
            {THEMES.map((t) => {
              const selected = t.id === theme
              return (
                <button
                  key={t.id}
                  type="button"
                  role="radio"
                  aria-checked={selected}
                  aria-label={t.label}
                  onClick={() => onChange(t.id)}
                  className="group flex flex-col items-center gap-3 rounded-lg p-3 text-center transition-colors hover:bg-background focus:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                >
                  {/* Circular color preview: theme background + accent dot. */}
                  <span
                    className={
                      'relative inline-flex h-16 w-16 items-center justify-center rounded-full border border-border shadow-inner transition-transform group-hover:scale-105 ' +
                      (selected ? 'ring-2 ring-accent ring-offset-2 ring-offset-card' : '')
                    }
                    style={{ background: t.preview }}
                  >
                    <span
                      aria-hidden
                      className="absolute bottom-1 right-1 h-5 w-5 rounded-full border-2 border-white/80 shadow"
                      style={{ background: t.accent }}
                    />
                    {selected && (
                      <svg
                        aria-hidden
                        viewBox="0 0 24 24"
                        className="h-6 w-6 drop-shadow"
                        style={{ color: t.accent }}
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="3"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      >
                        <path d="M5 13l4 4L19 7" />
                      </svg>
                    )}
                  </span>

                  <span className="flex flex-col gap-0.5">
                    <span
                      className={
                        'text-sm font-medium ' + (selected ? 'text-accent' : 'text-main')
                      }
                    >
                      {t.label}
                    </span>
                    <span className="text-xs text-muted">{t.description}</span>
                  </span>
                </button>
              )
            })}
          </div>
        </section>
      </div>
    </div>
  )
}
