---
category: Repo policy / workflow reminders
---

# Whole-corpus work in a test body is a timeout on a delay: build it once at import and share it

**What went wrong.** Four consecutive A1 PRs (Telugu, Kannada, Persian) each
failed CI's `build` job on a `human-language-data` test that timed out at the
30s budget. Each test passed in isolation, in 16-23s. Each timeout came from
whole-corpus work repeated inside a test body:

- `modality-manifest` parsed the corpus twice in one test.
- `chapter-intro` rendered all 23 books a second time in its test.
- `chapter-modality-book` and `book-tex` rendered every book inside tests.
- `level-gate` built its gap report lazily, so the first test paid ~20s.
- `integration` ran three track evidence modules in one test, each building the
  same gap report.

Every content PR grows the corpus, so each of these was going to cross the
budget sooner or later. The only question was which PR would trip it.

**The fix.** Do the expensive, read-only work once at module import, or in a
`beforeAll`, and share the result. Import time carries no per-test budget, and
`vitest.config.ts` explicitly says not to raise the 30s ceiling. No assertion
changed in any of these fixes.

**Do differently.**
- When a test reads the real corpus, parses it (`loadEverything`), renders the
  books (`generatedBookOutputs`) or builds the gap report
  (`buildCurriculumGapReport`), hoist that work to the top of the file.
  Give tests the shared value, never a fresh build.
- When adding a hook that fans out to modules (evidence, inventories), pass
  the shared artifacts in the context rather than letting each module rebuild
  them.
- Before a content PR, glance at the slowest tests:
  `npx vitest run --reporter=verbose | sort` by the trailing `ms`. Anything
  over ~12s without its own explicit budget is the next CI failure.
