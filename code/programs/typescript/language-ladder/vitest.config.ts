import { defineConfig } from "vitest/config";
import path from "node:path";
import { humanLanguageLedgerPlugin } from "./human-language-ledger-plugin.ts";
import { scriptInventoryPlugin } from "@coding-adventures/script-ductus/script-inventory-plugin.ts";

// The curriculum — both the script JSON and the ~7,500 lesson markdown files —
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
    // Vitest's defaults -- 5 s per test, 10 s per hook -- are sized for a suite
    // that has a machine to itself. This one does not, and it is unusually
    // expensive to set up: `beforeAll` in tests/curriculum.test.ts awaits
    // loadCurriculumPlans(), and the ledger plugin transforms the 7,485 lesson
    // markdown files under code/learning/human-languages/*/lessons/.
    // Measured on an idle 4-core box:
    //
    //   condition                          curriculum.test.ts   slowest test
    //   ---------------------------------  -------------------  ------------
    //   idle                                     5.0 s             ~4.3 s
    //   3x CPU oversubscription                  6.4 s             ~5.1 s
    //   CI, whole-repo build in parallel      >10 s (TIMEOUT)      4977 ms
    //
    // That last row is measured, not hypothetical: a repo-wide dependency bump
    // makes ~476 packages build at once, and on that run the hook blew the 10 s
    // limit and took all 7 of the file's tests down as "skipped" -- assertions
    // that never ran, reported as a pass-adjacent state. The slowest individual
    // test cleared the 5 s limit by 23 ms on the very same run. The same commit
    // passed on the ubuntu and windows lanes, which is the clearest sign that
    // the machine, not the code, was the variable.
    //
    // So these are not "make the red go away" numbers. Both budgets are ~12x the
    // idle cost, which is the headroom a loaded runner needs. Nothing here can
    // hang indefinitely -- loadCurriculumPlans() is a single-attempt
    // Promise.all over dynamic imports of build-time virtual modules, with no
    // network call and no retry loop -- and each CI job carries its own
    // timeout-minutes, so the worst case is bounded regardless.
    testTimeout: 60_000,
    hookTimeout: 60_000,
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
