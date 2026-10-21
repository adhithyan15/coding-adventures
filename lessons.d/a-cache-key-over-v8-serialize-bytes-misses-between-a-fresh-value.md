---
category: Testing & coverage
---

# A cache key over v8.serialize bytes misses between a fresh value and the same value read back from a cache

`human-language-data` gained a second test-only cache, for narrated chapters
(`src/narration-cache.ts`). The first version keyed each track on
`v8.serialize(parsedLessons)`: hash the value you are about to narrate, and a
stale entry is impossible. It was exact. It was also useless on every CI run.

`v8.serialize` writes how V8 happens to HOLD a value, not only what the value
is. A freshly parsed track's arrays are dense. The same arrays read back out of
the lesson cache are not, so the bytes differ. For German the two encodings were
23,680,372 and 23,916,753 bytes, and `isDeepStrictEqual` said the values were
equal. On a cold cache, which is every CI run after `npm ci`, the Vitest global
setup narrated freshly parsed lessons, while every worker narrated lessons
deserialized from the lesson cache. So the warm-up was stored under names no
worker ever looked up. Locally it looked fine on the second run, because by then
the global setup was reading from the lesson cache too. The tell was a cold run
where the `--check` case still took 21 s, after a global setup that had just
written every narration entry.

The fix keys the narration on the lesson cache's own key for the track. That key
is a SHA-256 of the lesson files' bytes, the script and the parser source. The
loader reports it (`loadLessons(root, onCacheKey)`) for the very bytes it parsed
or served, and one function loads and narrates without handing the array out in
between. A key over the SOURCE has no in-memory representation to disagree
about.

What to do differently:

- Do not use `v8.serialize` (or any structured-clone encoding) as a canonical
  form for hashing. Equal values can encode differently, depending on elements
  kind, Smi versus heap number, and string shape.
- When a cache is warmed in one process and read in another, test the cold path
  on purpose. Delete the cache, run once, and check that the consumer hits. A
  warm second run hides a key that differs between producer and consumer.
- Key derived data on the content address of its inputs' sources, when one
  exists, rather than on the derived objects.
