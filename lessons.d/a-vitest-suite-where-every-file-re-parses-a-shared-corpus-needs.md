---
category: Testing & coverage
---

# A Vitest suite where every file re-parses a shared corpus needs a content-addressed parse cache, not a longer timeout

`human-language-data`'s suite started timing out on busy CI runners: a
`beforeAll` in `modality-manifest.test.ts`, the narration `--check` case in
`narration-cli.test.ts` and the live-corpus case in `root-slug-splits.test.ts`.
Each was a different test, and each passed in isolation. The cause was the
same every time. About 200 test files read the real corpus (38,088 lessons,
98 MB of Markdown), and Vitest gives every file a fresh module graph, so each
file parsed the whole corpus again. An instrumented run counted **123
full-corpus parses per `npx vitest run`, about 1,250 s of CPU**, which was half
of everything the suite did. A single `loadEverything()` took 12.5 s, and 8.3 s
of that was `parseLesson` (frontmatter 1.2 s, body blocks 3.5 s, `sourceHash`
3.8 s, file reads 0.7 s).

Raising the budgets would only have moved the next timeout one content PR
later. The fix was to stop repeating the work. `src/lesson-cache.ts` stores
each track's parsed lessons under a SHA-256 of the lesson files' **bytes**, the
resolved script, the package's own `src/` and the V8 version. A Vitest
`globalSetup` parses the corpus once, before any worker starts, and a
`setupFiles` entry turns the cache on inside each worker. After the fix the
real corpus is parsed twice per run: once in the global setup, and once on
purpose by the cache's own fidelity test. Every other read is a hit.
`loadLessons()` in a worker went from 8.6 s to about 2.4 s, and
`loadEverything()` from 10.7 s to about 4.9 s. What remains is deserialization
plus loaders that are bound by file IO. The full suite went from 836 s to about
560 s on a 4-core machine.

What to do differently:

- Before you tune a slow test, count how often the expensive input is built
  per run. A temporary `appendFileSync` counter in the loader showed the
  repetition in a single run. No per-test profile would have shown it.
- Key a test cache on content, not on `(path, size, mtime)`. An edit inside one
  timestamp tick that keeps the size, or a tool that preserves mtimes, would
  otherwise serve a stale parse. The `--check` tests that compare generated
  files with the lessons would then pass against the old lessons. Hashing the
  bytes cost 0.4 s more per full load and closes that hole.
- Put the parser's own source in the key. A cache keyed only on data keeps
  serving output from yesterday's parser after a code change.
- Keep it test-only and limited to one root. The CLIs never configure it, and
  temp fixture corpora bypass it, so they cannot evict the real entries.
- Measure the decode path before you pick a format. `v8.deserialize` of the
  316 MB payload cost about the same as `JSON.parse`, once the file read and
  digest were included. v8 also keeps `NaN` and other values exactly, so it
  needed no fix-ups.
