import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // jsdom provides customElements, shadow DOM, and HTMLElement
    // for testing MosaicHostElement + defineMosaicElement.
    environment: "jsdom",
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      thresholds: {
        lines: 80,
        functions: 80,
        branches: 80,
        statements: 80,
      },
    },
  },
});
