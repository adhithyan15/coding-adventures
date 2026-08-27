import { defineConfig } from "vitest/config";
import path from "node:path";
import { humanLanguageLedgerPlugin } from "./human-language-ledger-plugin.ts";
import { scriptInventoryPlugin } from "@coding-adventures/script-ductus/script-inventory-plugin.ts";

// The curriculum — both the script JSON and the ~670 lesson markdown files —
// lives outside this package, at code/learning/human-languages/. We read those
// canonical files directly rather than copying them, so the app can never drift
// from the curriculum. Vite guards reads outside the project root, so the root
// must be declared legal here exactly as it is in vite.config.ts. (Two configs,
// one rule: vitest does NOT inherit vite.config.ts's server block, and without
// this the lesson glob fails with "Denied ID".)
const repoRoot = path.resolve(__dirname, "../../../..");
const curriculumRoot = path.join(repoRoot, "code", "learning", "human-languages");

export default defineConfig({
  plugins: [
    humanLanguageLedgerPlugin({ curriculumRoot }),
    scriptInventoryPlugin({ curriculumRoot }),
  ],
  server: {
    fs: { allow: [repoRoot] },
  },
  test: {
    environment: "jsdom",
    globals: true,
    coverage: {
      provider: "v8",
      // The pure logic is what we hold to a high bar; main.ts is the thin DOM
      // shell and data.ts is just JSON imports.
      include: [
        "src/core.ts",
        "src/drill.ts",
        "src/scheduler.ts",
        "src/interleave.ts",
        "src/concepts.ts",
        "src/lessons.ts",
        "src/progress.ts",
        "src/ductusview.ts",
      ],
    },
  },
});
