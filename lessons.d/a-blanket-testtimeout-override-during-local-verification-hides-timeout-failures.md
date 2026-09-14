# A blanket `--testTimeout` override during local verification hides timeout failures

**Context:** `human-language-data`, PR #10043 (second core-verb tranche, +24 lessons).

**What happened:** I verified locally with `npx vitest run --testTimeout=180000` and
reported "368 tests pass". CI then failed on `tests/cli.test.ts` — *"Test timed out in
5000ms"*. The override I passed to give slow corpus-walking tests room had silently
raised the ceiling for **every** test in the run, including the one that had no explicit
timeout and was relying on vitest's 5,000 ms default. I had made the very failure mode
invisible to the check that was supposed to catch it.

**Why it bit here:** `runCurriculumGapReport` builds the entire gap report **twice** (once
`--format json`, once `--format text`) over the whole corpus, and the report had just
gained a continuity section on top of modality, levels, verbs, chapters and ramp. At 1,249
lessons it runs ~5.08 s locally — already *over* the line on CI's slower runner. It was not
specific to this PR: the next content PR to land would have broken it instead, because the
corpus only ever grows.

**Fix (two parts):**
1. Give a test that legitimately needs more than the default an **explicit per-test**
   timeout — `it("...", { timeout: 60_000 }, () => {...})` — so the budget lives next to
   the test that needs it and travels with it, rather than being supplied by whoever
   happens to run the suite.
2. **Verify at default timeouts.** Run the suite once with no `--testTimeout` flag before
   claiming it passes. Use a per-file override only when actively iterating on one slow
   test, never as the final check. A local run whose flags differ from CI's is not
   evidence about CI.

**Generalisation:** any CLI flag that loosens a threshold globally (`--testTimeout`,
`--bail=0`, `--maxWorkers`, coverage thresholds) makes a local run *less* like CI, not
more. If you pass one to get a green run, the green means less than it looks like — and
the final verification pass should always be the one with no flags at all.
