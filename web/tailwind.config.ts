import type { Config } from 'tailwindcss'

export default {
  darkMode: ['class', '[data-theme="dark"]'],
  content: ['./app/**/*.{ts,tsx}', './components/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        bg: 'var(--bg)',
        'bg-1': 'var(--bg-elev-1)',
        'bg-2': 'var(--bg-elev-2)',
        surface: 'var(--surface)',
        text: 'var(--text)',
        'text-dim': 'var(--text-dim)',
        muted: 'var(--muted)',
        border: 'var(--border)',
        ring: 'var(--ring)',
        accent: {
          50: 'var(--accent-50)',
          100: 'var(--accent-100)',
          200: 'var(--accent-200)',
          300: 'var(--accent-300)',
          400: 'var(--accent-400)',
          500: 'var(--accent-500)',
          600: 'var(--accent-600)',
          700: 'var(--accent-700)',
          800: 'var(--accent-800)',
          900: 'var(--accent-900)'
        }
      },
      borderRadius: { sm: '6px', md: '10px', lg: '14px', xl: '18px' },
      boxShadow: {
        card: 'inset 0 1px 0 rgba(0,0,0,.3), 0 0 0 1px var(--border)',
        pop: '0 8px 24px rgba(0,0,0,.42)'
      },
      transitionTimingFunction: { std: 'cubic-bezier(.2,.8,.2,1)' }
    }
  },
  plugins: []
} satisfies Config

