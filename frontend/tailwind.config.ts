import type { Config } from 'tailwindcss'

export default {
  darkMode: 'class',
  content: [
    './packages/**/*.{ts,tsx}',
    './workbench/**/*.{ts,tsx}',
  ],
  theme: {
    extend: {
      colors: {
        // Semantic, not aesthetic
        intent: {
          composing: 'oklch(0.65 0.15 240)',      // Blue - drafting
          compiling: 'oklch(0.6 0.18 280)',       // Purple - processing
          simulating: 'oklch(0.7 0.15 180)',      // Teal - running
          approved: 'oklch(0.55 0.15 145)',       // Green - ready
          executing: 'oklch(0.6 0.2 45)',         // Amber - live
          failed: 'oklch(0.55 0.2 25)',           // Red - error
        },
        risk: {
          none: 'oklch(0.55 0.15 145)',
          low: 'oklch(0.7 0.15 85)',
          medium: 'oklch(0.75 0.18 60)',
          high: 'oklch(0.7 0.2 30)',
          critical: 'oklch(0.55 0.2 25)',
        },
        system: {
          local: 'oklch(0.55 0.15 145)',
          cloud: 'oklch(0.65 0.15 240)',
          airgap: 'oklch(0.55 0.15 145)',
          degraded: 'oklch(0.7 0.15 30)',
        },
        // Surface tokens (dark-first, OKLCH for perceptual uniformity)
        surface: {
          base: 'oklch(0.12 0.01 280)',
          raised: 'oklch(0.16 0.01 280)',
          overlay: 'oklch(0.2 0.01 280)',
          border: 'oklch(0.25 0.01 280)',
        },
        text: {
          primary: 'oklch(0.95 0.01 280)',
          secondary: 'oklch(0.7 0.01 280)',
          muted: 'oklch(0.5 0.01 280)',
          inverse: 'oklch(0.12 0.01 280)',
        },
      },
      fontFamily: {
        sans: ['Inter Variable', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono Variable', 'monospace'],
        display: ['Inter Variable', 'system-ui', 'sans-serif'],
      },
      animation: {
        'simulation-pulse': 'pulse 2s ease-in-out infinite',
        'timeline-slide': 'slideIn 0.3s ease-out',
        'graph-layout': 'layout 1s ease-out',
        'voice-wave': 'wave 0.1s linear infinite',
      },
      spacing: {
        // 4px base, semantic scales
        'space-1': '4px',
        'space-2': '8px',
        'space-3': '12px',
        'space-4': '16px',
        'space-5': '24px',
        'space-6': '32px',
        'space-8': '48px',
        'space-10': '64px',
      },
    },
  },
  plugins: [],
} satisfies Config