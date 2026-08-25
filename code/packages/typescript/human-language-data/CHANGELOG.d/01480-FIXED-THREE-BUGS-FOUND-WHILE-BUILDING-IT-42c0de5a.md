### Fixed - three bugs found while building it

**The etymon namespace silently contributed zero.** The frontmatter keys are
flat and dotted -- `introduces.knowledge`, never a nested `introduces` object --
and reading them as nested returns `undefined` for every lesson in the corpus.
The ledger reported 1,966 roots instead of 2,717, which reads as "the corpus has
no etymon atoms" rather than "the reader is broken". `ramp.ts` already carried a
warning about this exact mistake, from when it made the chapter gates report all
279 authored chapters as broken; this module now uses that file's shared
`frontmatterList` rather than its own reader.

**A composite key that could merge two roots.** `${language} ${namespace}
${root}` lets `("es", "roots", "a b")` and `("es", "roots a", "b")` collide and
silently sum two roots' payoff counts. Now length-prefixed. No collision existed
in the current corpus -- the counts are unchanged -- but a root slug is
author-written and may contain anything.

**NUL bytes in a source file.** The spaces inside a template literal were
written to disk as U+0000: `${language}\0${namespace}`. The file still compiled,
`grep` silently found nothing in it, and an exact-match edit could not touch the
line. A NUL in source is always a write accident, never intent, so
`tests/root-ledger.test.ts` now asserts that no file in `src/` contains one --
cheaper to assert than to rediscover.


