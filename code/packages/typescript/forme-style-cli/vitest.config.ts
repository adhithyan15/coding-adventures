import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      reporter: ["text", "lcov"],
      include: ["src/**/*.ts"],
      // bin.ts is a 3-line shim around `run(process.argv, …)` — its
      // sole purpose is to exist as the npm `bin` entry; testing it
      // would require spawning a subprocess.  All real logic lives
      // in cli.ts and is exhaustively covered.
      exclude: ["src/bin.ts"],
    },
  },
});
