### Fixed — curriculum validation is no longer quadratic

- `src/curriculum.ts`: `validateCurriculum()` on the real corpus (about
  29,500 lessons and 5,800 book chapters) fell from about 34 s to 13 s on
  the same machine, and gives an identical answer. The integration test
  "has a valid shared spine" runs this check, and its 30 s timeout was
  failing locally.
  - Knowledge closure: each lesson re-walked its whole prerequisite chain,
    about 18 million steps over the corpus. Closures are now memoised as
    per-language bitsets and computed bottom-up with an explicit stack, so
    no chain is deep enough to overflow the call stack. A corpus containing
    a prerequisite cycle, a cross-language prerequisite or a duplicated
    lesson id falls back to the original walk, which still defines the
    answer.
  - "Does a lesson exist for this book chapter?" was a scan of every lesson
    for every chapter; it is now one lookup in a set built once.
  - The concepts a track realises are gathered once per curriculum instead
    of once per spine node.
- `tests/curriculum.test.ts`: three new cases check that the memoised
  closure matches the walk:
  - through a diamond and a schema-v1 lesson;
  - down a 4,000-lesson chain;
  - on the cross-language fallback.
