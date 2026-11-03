# HL112 — Corpus tests fit their budgets by holding less

## Status

Implemented with this spec. It changes no lesson, generated artifact, report
schema, timeout or heap ceiling. It changes what four test files of
`code/packages/typescript/human-language-data` hold in memory and how often they
repeat whole-corpus work, and adds two small exported functions to `src/` that
make that possible.

## Problem

The corpus is about 62,000 lessons across 23 tracks (≈270 MB of Markdown). Four
test files outgrew the budgets `vitest.config.ts` states, and they fail
unrelated PRs whenever this package is rebuilt:

- `tests/figure-targets.test.ts` held five whole parsed copies of the corpus in
  one fork worker. The shared real-corpus fixture loaded one, and three cases
  each loaded another (one of them twice) inside a `describe` callback, where it
  stays reachable for the life of the worker. The worker peaked at about 3.8 GB
  RSS. On PR #17737 one died with "Reached heap limit … JavaScript heap out of
  memory".
- `tests/level-gate.test.ts` re-measured the level gate's inputs
  (`summarizeLevels`, `measureRamp`, `measureContinuity`) in about a dozen test
  bodies and rebuilt the entire gap report in one more. Six cases took 11–15 s
  each, alone, on a quiet machine.
- `tests/modality.test.ts` derived the modality of every lesson four times in
  test bodies, on top of holding the whole parsed corpus for the file's life.
- `tests/book-cli.test.ts` re-read the sharded book-generation config it had
  already parsed, read every committed chapter file in full to compare a
  17-byte banner, and looked up declarations with a scan per ledgered chapter.

The config's rule stands: a test that needs more than its budget is a signal
about the test. Neither the timeout nor the heap ceiling is raised.

## Contract

### Streaming the corpus

`src/loader.ts` gains `forEachTrackLessons(root, visit, onCacheKey?)`. It is the
walk `loadLessons` always did — same track order, same symlink and track-id
refusals, same lesson cache — but it hands each track's parsed lessons to
`visit` and keeps nothing itself. `loadLessons` is reimplemented as that walk
with an appending visitor, so the two cannot drift.
`tests/loader-guards.test.ts` pins the equivalence on a fixture root, including
the empty visit of a track-shaped directory with no `lessons/` and the refusal
of an unusable track id.

A test that derives something small from each lesson reads the corpus through
`forEachTrackLessons` and keeps only that derivation. At most one track's parsed
lessons are then alive at a time.

### Summaries over derived entries

`src/modality.ts` gains `summarizeLessonModalities(entries, options)`: the body
of `summarizeModality`, with the per-lesson derivation hoisted out.
`summarizeModality(lessons, options)` is now defined as
`summarizeLessonModalities(lessonModalities(lessons, options), options)`. A
caller that derives entries per track and concatenates them therefore gets the
identical summary. `tests/modality.test.ts` pins that on a two-track fixture at
both table widths.

### Per-file rules

- **figure-targets.** `tests/figure-targets/real-corpus-fixture.ts` makes the
  suite's only pass over the corpus. For each track it keeps:
  - the lessons that yield a filmstrip candidate or own a declared target;
  - every lesson id;
  - which lesson ids' bodies match the two strip-prose patterns.

  Those patterns now live in `tests/figure-targets/strip-prose.ts`, because
  the fixture must test each body while its track is in memory. The resolved
  targets equal `resolvedFigureTargets(root, loadLessons(root))`. That function
  reads lessons only through `filmstripCandidates`, which judges each lesson
  alone and sorts its result by id. No case calls `loadLessons`.
- **modality.** The corpus-regression block streams the corpus once. While each
  track is in memory it derives every lesson's modality at the shipped width
  and at width 0, and the structural partition counts. Each of the three
  partition groups keeps its own predicate, so a renamed field still breaks
  their sum.
- **level-gate.** The gate's inputs (`levels`, `ramp`, `continuity`) are read
  off the gap report built once at import. The report computes them with the
  same calls the tests used to repeat. The report text is rendered once. A test
  that needs a variant spreads a copy. The "absent without a policy" case builds
  its two reports over one real track, not the whole corpus. That track is the
  smallest one that attains a rung, so the case now shows both halves: no gate
  without the policy, and the same rung with only the policy added.
- **book-cli.** The banner check reads 17 bytes per committed chapter, not the
  whole file. The title/label case reuses the config its `describe` already
  parsed. The ledger-coverage case indexes declarations once, first one winning,
  which is what the `find` it replaces returned.

No assertion is removed or loosened. Where a case's input changed from "the
parsed corpus" to a projection of it, the projection carries exactly the fields
the assertion reads.

## Measurement

These numbers are from macOS arm64 (12 cores, Node 22) with a warm lesson
cache, running each file alone at default settings. The "before" column is a
clean `origin/main` checkout at `42bc1f6399c`.

Peak RSS is sampled every 0.2 s from the vitest fork **worker**. Run
`/usr/bin/time -l` on vitest and it reports about 1.5 GB for every file. That
figure is the main process, where `globalSetup` reads the whole corpus to warm
the cache, and it hides the worker.

| File | Worker peak RSS | Test bodies | Whole file |
| --- | --- | --- | --- |
| figure-targets | 3,817 MB → 448 MB | 0.8 s → 0.5 s | 58.1 s → 31.4 s (import 42.3 s → 14.6 s) |
| level-gate | 1,445 MB → 1,317 MB | 118.6 s → 2.3 s | 180.7 s → 80.2 s |
| modality | 1,838 MB → 764 MB | 6.8 s → 0.7 s | 38.8 s → 43.1 s |
| book-cli | 1,599 MB → 1,500 MB | 7.9 s → 6.3 s | 63.4 s → 70.7 s |

The slowest single case in level-gate went from 14.9 s to 1.2 s. Nothing in
modality now takes more than 0.4 s, and nothing in figure-targets more than
0.3 s. Whole-file wall time for modality and book-cli is within run-to-run
noise: their remaining cost is import-time work the claims need. That is the
real corpus, plus book-cli's full render of every book (about 45–50 s). In
book-cli's test bodies, what remains is mostly reading the 12,227 committed
chapter files (about 2 s) and the chapter ledgers (about 1.3 s). Both are what
those assertions inspect.

## Verification

- An uncommitted probe compared, on the real corpus, every streamed value with
  the whole-corpus derivation it replaces. It checked the 1,180 filmstrip
  targets, their owner lessons, all lesson ids and both phrase-match sets. It
  checked the modality summaries at both widths and the per-lesson entries.
  It checked that the report's `levels`, `ramp` and `continuity` deep-equal
  fresh measurements. All matched.
- Mutations were made and then reverted:
  - patterns that match nothing or everything failed two figure-target cases;
  - modality counters that drop a track or count the wrong block type, and a
    width-0 summary taken at the shipped width, failed three modality cases;
  - an emptied `continuity.reinforcement`, and a policy-less report that was
    handed a policy, failed four level-gate cases.
- Each file passed alone at default settings. `check:doc-shards` passed for the
  changelog shard.
- The full package suite (`npx vitest run --coverage`, as BUILD runs it) was run
  on this branch and on clean `origin/main` on the same machine. All four files
  passed under full parallel load on this branch. Neither run was green. This
  branch had 14 failed / 4,430 passed, and `main` had 16 failed / 4,425 passed,
  each in 14 files. Every failure was a 30–180 s timeout in a whole-corpus
  case of some other file: `chapters-shards`, `curriculum-shards`,
  `gentle-ramp`, `plan-cli`, `track-progress`, `modality-manifest`'s
  `beforeAll`, and others. The two lists mostly overlap, and the cases that
  differ are ones close to their budget on both trees. They are the same class
  of problem as this spec, in other files, and are out of its scope.
