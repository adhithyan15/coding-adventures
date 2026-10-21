### Changed — four corpus test files load only what they read, once

A follow-up to 08620. These cases timed out at 30 s on a loaded macOS runner:
`track-progress.test.ts` (the `--report` case), `verbs.test.ts` (both corpus
cases), `script-closure.test.ts` and `standalone-book.test.ts`. No timeout was
raised.

- **Measured, not assumed.** None of these files builds the gap report. With
  the lesson cache warm, the repeated cost was `loadEverything()`, about 4 s on
  a quiet 4-core machine, called in a test body up to four times per file:
  - `loadLessons`, a cache hit: 2.2 s;
  - `loadLanguageCurricula`: 1.4 s, roughly 39,000 membership owners at two
    `lstat`s and one read each;
  - `loadBookCorpus`: 0.4 s.

  The verb and script-closure reports read only lessons (and the taxonomy), and
  the standalone-book walk reads only lessons.
- **The fix.** Each file now loads exactly the inputs its derivation reads,
  through the same loaders `loadEverything` calls, and derives the pure report
  at most once in a lazy memo. Every case only reads the result.
  - `verbs.test.ts`: `realReport()` is `verbCoverage(loadLessons(),
    loadTaxonomy())`, shared by both corpus cases.
  - `script-closure.test.ts`: `realClosure()` is shared by the collection-time
    corpus block and the three cases that measured the corpus again.
  - `standalone-book.test.ts`: one `LESSONS` load, shared by `SURFACES` and the
    frontmatter case. The frontmatter case still scans the raw
    `canonicalLessonSource`, exactly as before.
- **Drift still fails.** Three edits were made and then reverted:
  - retagging Spanish `VERB-HELP` failed the verb baseline;
  - citing `ES-C14` in a German gloss failed the frontmatter case;
  - deleting a Tamil `romanization:` failed both script-closure debt cases.
- **`track-progress.test.ts` is unchanged.** Each of its two corpus cases is
  one end-to-end `runTrackProgress` over the real root, and that load is the
  thing under test. Its cost is filesystem-bound: lessons plus the curriculum
  owners' per-file `lstat`s. Making it cheaper means changing the guarded
  ledger reader, which is a security path and is left for its own change.
- **Security-review follow-ups to 08620.**
  - L1: `required_capabilities.json`'s top-level justification now names the
    parsed-lesson AND narration caches.
  - L2: the narrator import-graph guard now also follows `export … from` and
    side-effect imports. It refuses dynamic `import()`, `require()`, `process`,
    `globalThis`, `Date`, `Math.random`, `performance`, `eval()` and
    `new Function`. A new case tests the walker itself against each construct.
  - L3: the narration key now names `maxLinearisableTableColumns` and `titles`
    field by field instead of spreading the options. The options have one
    constructor, `corpusNarrationOptions`, and a new case pins its exact key
    set, so an added field fails until the key covers it.
- **Second security-review pass** (all Low):
  - `narrateCorpusByTrack` copies the two option fields into one fresh object
    and hands that same object to the key AND to `narrationChapters`. A field
    added to the caller's options can no longer reach the narrator without also
    reaching the key.
  - The narrator guard also refuses `eval` in any form (indirect
    `(0, eval)(…)` too), `Function(…)` with or without `new`, `global`,
    `crypto`, `import.meta` and `Reflect`, each with a probe case. The guard is
    fail-closed and reads comments, so four comments in the narrator's graph
    were reworded: "global pass/scan/replace" became "whole-text" wording, and
    `import.meta.glob` became "Vite's glob import". No code changed there.
- **Measured** on the same 4-core machine. Per-case seconds:

  | case | alone before → after | full suite before → after (cold / warm) |
  |---|---|---|
  | verbs: core verb baseline | 4.3 → 2.6 | 5.3 → 2.6 / 2.7 |
  | verbs: explicit authoring list | 4.0 → 0.0 | 4.8 → 0.0 / 0.0 |
  | script-closure: three corpus cases | 4.6–5.1 each → 0.0 | 5.6–6.4 each → 0.0 / 0.0 |
  | standalone-book: frontmatter | 7.0 → 1.5 | 7.6 → 1.8 / 1.7 |
  | track-progress: `--report` | 4.5 → 5.1 (unchanged code) | 4.6 → 5.1 / 5.1 |

  Each memo is filled the first time a file uses it. Script-closure fills it
  at collection time, as before, at the cost of one lessons-only load. Full
  suite: 4,356 tests pass, in 489 s on a cold cache and 467 s warm. The
  previous measurement was 479 s cold and 460 s warm.
