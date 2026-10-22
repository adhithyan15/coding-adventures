---
category: Testing & coverage
---

# A corpus test that reads only lessons must not call loadEverything

On a loaded macOS runner, `human-language-data` timed out at 30 s in
`verbs.test.ts`, `script-closure.test.ts` and `standalone-book.test.ts`. The
follow-up request blamed corpus-wide gap-report builds. The profile showed
otherwise: none of those files builds a gap report. Each case called
`loadEverything()` in its own body, sometimes four times per file, to use only
`lessons` (plus `taxonomy` in `verbs`).

On a quiet 4-core machine with the lesson cache warm, one `loadEverything()`
cost about 4 s. Of that, `loadLessons` (a cache hit) took about 2.2 s,
`loadLanguageCurricula` 1.4 s and `loadBookCorpus` 0.4 s. The curricula load
reads about 39,000 membership owner files, with two `lstat`s and a read for
each, so it is bound by the filesystem and grows fastest on slow filesystems.
The tests never looked at curricula or books.

The fix is per file and exact. Each file loads exactly the inputs its
derivation reads, through the same loaders `loadEverything` calls. It derives
the pure report (`verbCoverage`, `measureScriptClosure`) at most once, in a
lazy memo, and every case only reads the result. The affected cases went from
4–7 s to 0–2.6 s, and a deliberately broken lesson still fails each of them.

What to do differently:

- Profile before you accept a diagnosis. "Gap report" was a plausible culprit
  and the wrong one; a `performance.now()` probe around each loader found the
  real cost in a minute.
- Treat `loadEverything()` as convenient but not free. In a test, ask which
  loaders the code under test actually reads, and call only those.
- When several cases in a file derive the same pure report from the real
  corpus, derive it once in the file and share it read-only.
