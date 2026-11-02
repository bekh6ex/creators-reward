import type { KnipConfig } from 'knip';

// tailwindcss and daisyui are used only from CSS (`@import "tailwindcss"`,
// `@plugin "daisyui"` in src/style.css). The compiler turns both directives
// into imports so knip counts those dependencies as used.
export default {
  project: ['src/**/*.{ts,tsx,css}'],
  compilers: {
    css: (text: string) =>
      [...text.matchAll(/@(?:import|plugin)\s+["']([^"']+)["']/g)]
        .map(([, specifier]) => `import "${specifier}";`)
        .join('\n'),
  },
} satisfies KnipConfig;
