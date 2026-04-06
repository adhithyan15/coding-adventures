import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov'],
      // types.ts contains only TypeScript interface/type declarations — no
      // runtime code is emitted for it. Exclude it so it doesn't artificially
      // lower the coverage percentages. Also exclude the vitest config itself.
      exclude: ['**/types.ts', '**/dist/**', '**/node_modules/**', '**/vitest.config.ts'],
      thresholds: { lines: 80, functions: 80, branches: 80, statements: 80 }
    }
  }
});
