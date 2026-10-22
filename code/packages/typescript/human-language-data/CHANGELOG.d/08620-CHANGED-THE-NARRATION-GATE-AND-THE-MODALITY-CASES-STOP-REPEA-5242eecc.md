### Changed — the narration gate and the modality cases stop repeating work

Two corpus-wide tests kept timing out on slow CI runners after the lesson cache
(08585) landed: `narration-cli.test.ts` "is in sync with the lessons it came
from" (60 s budget) and `modality-manifest.test.ts` "carries the marker into
the committed manifest" (30 s). No timeout was raised. Each now does less work,
and each still fails on drift between the lessons and the committed files. That
was checked by editing a Spanish lesson, which failed both gate cases, and by
dropping one `delivery: script` marker, which failed the marker case. Both edits
were then reverted.

- **Where the time went.** These were measured, not inferred, on a 4-core
  machine with the lesson cache warm.
  - The narration `--check` took 23.0 s. It was not re-parsing: `loadLessons`
    already read through the lesson cache (2.4 s). Of the rest, 12.8 s was
    narrating all 7,811 chapters, about 5 s was rendering and `fnv1a64`-hashing
    the 248 MB export, and about 1.5 s was the byte comparison.
    `narration.test.ts` narrated the whole corpus a second time, in another
    worker, at collection time.
  - The marker case took 5.2 s. That was two `loadModalityManifest()` calls,
    each opening, `lstat`ing and parsing all 39,484 owner files (2.3 s each).
    It did not rebuild the manifest or the gap report. The "in step" case read
    the same files a third time.
  - `runModalityManifest` built the manifest twice from the same lessons: once
    for itself, and once inside `generatedModalityOutputsFromLessons`.
- **New `src/narration-cache.ts`.** A test-only, content-addressed cache of
  each track's narrated chapters.
  - The key is a SHA-256 over a format version, V8, ICU, the collation locale,
    the package source digest, the track id, the policy table width, that
    track's chapter titles, and the lesson cache's key for that track's lessons.
    The loader reports that key through a new optional
    `loadLessons(root, onCacheKey)` / `loadTrackLessons(…, onCacheKey)`
    parameter, for the exact bytes it parsed or served.
  - The cache opens no file itself. `src/lesson-cache.ts` now exposes its
    validated store as `openEntryStore<T>`, and `openLessonCache` is that store
    plus the lesson key. The narration cache opens the same store on the fixed
    sibling directory `node_modules/.cache/human-language-data/narration`. So
    every rule applies to it unchanged: a private 0700 directory, private 0600
    entries, no links, the same-file check, the magic and SHA-256 check, atomic
    writes and the sweep. `hostCachePlatform(defaultDirectory)` names the one
    directory Windows accepts, so on Windows each cache is allowed in its own
    fixed directory only.
  - It is on only inside Vitest, only for the real curriculum root, and only
    while the lesson cache is on. `generate:narration` and `check:narration`
    never use it.
- **`narration-cli.ts`.** New `corpusNarration(root)` gathers the export's
  inputs (policy width, lessons, titles) in one place and narrates them.
  `narrationOutputs` renders its result. Inside the suite it narrates track by
  track through the cache. A comment explains why concatenating per-track
  results in `localeCompare` order equals one whole-corpus call. The two
  assumptions behind that (no `/` in an id, no two ids comparing equal) are
  checked on each call, and if either fails the corpus is narrated the plain
  way. `chapterTitles` is now exported.
- **Setup.** `tests/setup/lesson-cache.ts` turns the narration cache on in every
  worker. `tests/setup/warm-lesson-cache.ts` calls `corpusNarration` once, which
  warms both caches before any worker starts.
- **`narration.test.ts`.** "The whole corpus" now inspects `corpusNarration()`.
  That is the export's own narration: width 3 from the policy, as before, plus
  the authored titles. These assertions now cover exactly what
  `check:narration` gates, and the suite narrates the corpus once instead of
  twice.
- **`modality-cli.ts`.** New `generatedModalityOutputsFromManifest`.
  `runModalityManifest` renders the owner files from the manifest it already
  built, so it derives the manifest once instead of twice. The output bytes are
  the same.
- **`modality-manifest.test.ts`.** The committed manifest is read lazily, at
  most once per file, and shared by the marker case and the "in step" case.
  `--check` only reads, so the shared read sees the same bytes a fresh read
  after it would.
- **New `tests/narration-cache.test.ts`** (15 cases):
  - key sensitivity, including another track's titles being ignored, title
    insertion order and length-prefix forgery;
  - cached equal to uncached, in corpus order, across two interleaving tracks;
  - a hit writes nothing, and a lesson edit re-narrates only its own track and
    retires the old entry;
  - title and width edits are misses;
  - no entry when the lesson cache is off, and off for other roots;
  - the loader reports exactly the keys the lesson cache files tracks under;
  - group- and other-readable directories are refused, and Windows accepts only
    the default directory (simulated);
  - the suite configuration is checked, and a real track (Chinese) read through
    the cache equals a fresh narration;
  - a walk of `narration.ts`'s runtime import graph fails if the narrator ever
    imports anything outside `src/` or reads the environment or clock. Any such
    read would be an input the key cannot see.
- `required_capabilities.json` justifications now mention the narration
  directory. The target glob was already `node_modules/.cache/human-language-data/**`.
- **Measured.** Same 4-core machine, before → after.

  | | alone, before | alone, after | full suite, before | full suite, after (cold / warm) |
  |---|---|---|---|---|
  | narration "is in sync" | 23.0 s | 8.3 s | 24.3 s | 10.2 s / 10.6 s |
  | modality "carries the marker" | 5.2 s | 2.3 s | 5.0 s | 2.4 s / 2.7 s |
  | modality "in step" | 15.6 s | 10.3 s | 19.3 s | 12.6 s / 13.0 s |
  | full suite wall | | | 474 s | 479 s / 460 s |

  "Before" was measured warm. All 4,354 tests pass in both runs (4,339
  before, plus the 15 new ones). On a cold cache, as after `npm ci`, the global
  setup now also narrates the corpus once, serially, before the workers start.
  That costs about 12 s, and in exchange two workers each skip a full
  narration, which is why cold wall time is roughly unchanged. Disk use goes up
  by about 126 MB.
