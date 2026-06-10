/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      // Semantic theme colors backed by the CSS variables in index.css.
      // These produce utilities like `bg-background`, `bg-card`, `text-main`,
      // `text-muted`, `bg-accent` / `text-accent` / `border-accent`, and
      // `border-border`. They change automatically with <html data-theme=…>.
      colors: {
        background: 'var(--color-bg)',
        card: 'var(--color-card)',
        main: 'var(--color-text-main)',
        muted: 'var(--color-text-muted)',
        accent: 'var(--color-accent)',
        'accent-contrast': 'var(--color-accent-contrast)',
        border: 'var(--color-border)',
      },
    },
  },
  plugins: [],
}

