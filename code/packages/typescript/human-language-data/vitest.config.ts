import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // 30s, not vitest's 5s default. This package's tests walk the WHOLE corpus — 1,385
    // lessons across 22 tracks — and several build the entire gap report (modality,
    // levels, verbs, chapters, ramp, continuity, level-gate) to assert one number on it.
    // The corpus grows with every content PR, so the 5s default has been overrun five
    // times, once per authoring wave, each time by a different test and each time only
    // under full-suite parallel load on CI while passing in isolation locally.
    //
    // THIS IS NOT THE `--testTimeout` ANTIPATTERN recorded in lessons.md, and the
    // difference is the whole point. That entry warns against passing a timeout FLAG on
    // the command line, because it makes a local run behave unlike CI and hides the
    // failure it was meant to catch. A value here is declarative and applies identically
    // to `npm test`, a bare `npx vitest run`, and CI — it removes the divergence instead
    // of creating one. Per-test budgets still override this where a case wants to state
    // its own cost explicitly.
    //
    // If a test ever legitimately needs more than 30s, that is a signal about the test,
    // not about this number. Do not raise it to rescue a genuinely slow assertion.
    testTimeout: 30_000,
    // Hooks need the same treatment, for a sharper version of the same reason.
    // `plan-cli.test.ts` copies the WHOLE curriculum into a temp dir per case and
    // deletes it again in `afterEach`; that recursive delete is thousands of files,
    // and it is the hook, not the test, that pays for it. Vitest's hook budget
    // defaults to 10s and never moved when `testTimeout` did, so the growing corpus
    // reached it here first — the file passes in isolation and fails only under
    // full-suite parallel load, which is exactly the signature described above.
    // Raising this is the same declarative fix for the same cause, not a new waiver:
    // the hook is doing real filesystem work that scales with the corpus. The
    // durable fix is for that test to stop copying the entire corpus per case.
    hookTimeout: 30_000,
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      // The direct-invoke guard in cli.ts only runs as a standalone process.
      exclude: ["src/index.ts"],
      thresholds: {
        lines: 85,
      },
    },
  },
});
