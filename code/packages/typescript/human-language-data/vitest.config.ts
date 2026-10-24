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
    // `plan-cli.test.ts` copies the curriculum into a temp dir per case and
    // deletes it again in `afterEach`; that recursive delete is thousands of files,
    // and it is the hook, not the test, that pays for it. Vitest's hook budget
    // defaults to 10s and never moved when `testTimeout` did, so the growing corpus
    // reached it here first — the file passes in isolation and fails only under
    // full-suite parallel load, which is exactly the signature described above.
    // Raising this is the same declarative fix for the same cause, not a new waiver:
    // the hook is doing real filesystem work that scales with the corpus. That test
    // now skips the build products the plan never reads (about 43% of the files)
    // and states its own hook budget. The copy still grows with every lesson.
    hookTimeout: 30_000,
    // The budgets above are not what keeps the suite inside them; doing less
    // work is. About 200 test files read the real corpus, and each one used to
    // parse all ~38,000 lessons from scratch in its own worker — measured at
    // 123 full-corpus parses and ~1,250 s of CPU per run, half of everything
    // the suite did. `src/lesson-cache.ts` stores each track's parsed lessons
    // under a hash of the lesson files' bytes and the parser's own source, so
    // a lesson edit can never be answered from a stale entry.
    //
    // `globalSetup` parses the real corpus once, in the main process, before
    // any worker starts. `setupFiles` runs inside every worker and turns the
    // cache on for that test file's module graph. CLI processes never run
    // either, so `generate:*` and `check:*` keep parsing from disk. (A CLI's
    // `run…()` function called INSIDE a test does read through the cache —
    // that is how the in-process `--check` cases got faster — and what it reads
    // is byte-for-byte what a fresh parse returns, which
    // `tests/lesson-cache.test.ts` checks for every real track.)
    globalSetup: ["./tests/setup/warm-lesson-cache.ts"],
    // A 4 GB old-space ceiling for every test worker, stated rather than
    // inherited. Node sizes its default heap from the machine's RAM: about
    // 4 GB on a 16 GB Linux runner but only about 2 GB on the 7 GB
    // `macos-latest` runner. With ~42,000 lessons a corpus-wide worker crosses
    // 2 GB, so on macOS one worker died with "Reached heap limit ... JavaScript
    // heap out of memory", vitest reported "Worker exited unexpectedly", and
    // the cases queued behind it timed out (#17522, #17972). Linux never got
    // near its larger default, which is why only macOS failed. Pinning the
    // ceiling makes every platform run the suite under the same limit.
    execArgv: ["--max-old-space-size=4096"],
    setupFiles: ["./tests/setup/lesson-cache.ts"],
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
