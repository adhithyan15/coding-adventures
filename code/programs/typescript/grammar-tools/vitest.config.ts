import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    globals: true,
  },
  resolve: {
    alias: {
      "@coding-adventures/grammar-tools":
        new URL("../../../packages/typescript/grammar-tools/src/index.ts", import.meta.url)
          .pathname,
      "@coding-adventures/cli-builder":
        new URL("../../../packages/typescript/cli-builder/src/index.ts", import.meta.url)
          .pathname,
      "@coding-adventures/state-machine":
        new URL("../../../packages/typescript/state-machine/src/index.ts", import.meta.url)
          .pathname,
      "@coding-adventures/directed-graph":
        new URL("../../../packages/typescript/directed-graph/src/index.ts", import.meta.url)
          .pathname,
    },
  },
});
