### Changed — Spine validation stops copying knowledge closures

- `validateCurriculum` no longer turns each lesson's memoised
  prerequisite-knowledge bitset back into a `Set<string>`, and no longer copies
  that Set twice more for the `available` and block-frontier checks. Late in a
  track a learner knows thousands of atoms, so every one of the 60,202 lessons
  built three large Sets to answer a few `has` questions. That cost lessons x
  atoms work plus the garbage collection after it. The closure now answers
  `has(atom)` with one Map lookup and a bit test against each prerequisite's
  memoised closure. The two derived Sets are now unions of two lookups.
- The relocation ledger is built from a per-curriculum concept index instead of
  a full-corpus filter inside the curriculum x spine-node loop. Lessons are
  grouped by language once, path segments by `spine_node` once, extension ids
  are collected into a Set, and "does language L realise the spine?" is
  answered from a single pass. The relocation index keeps corpus positions, so
  each ledger is rebuilt in the original order with the original last-wins
  overwrite, and the JSON comparison sees the same bytes.
- Behaviour is unchanged: same issues, same codes, same messages, same order.
  A differential harness ran the pre-change and post-change compiled
  validators on 169 randomly mutated copies of the real corpus: 166 multi-track
  subsets plus 3 full copies, with 31 mutation kinds and 36 distinct issue
  codes raised and compared the Issue arrays with
  `isDeepStrictEqual`. There were 0 mismatches. New unit tests pin the
  relocation ledger's order and overwrite semantics, and the multi-word
  bitset query, including the sign bit and an atom known only through the
  second prerequisite. Both new tests pass against the old implementation
  too.
- Timings on the real corpus, measured on a 4-core machine. The integration
  case "has a valid shared spine covering every registered language" went
  from 45.3 s, which overran its 30 s budget, to 7.4 s. A standalone
  `validateCurriculum` with the duration cache warm went from about 40 s to
  about 8 s. Cold, it went from 45.7 s to 18.4 s, and 10 s of that is the
  unchanged `estimateLessonDuration`. Full suite with `--maxWorkers=2` on a
  shared machine whose load average stayed above the core count: before, 4,785
  s, with 19 cases in 13 files timing out (the spine case alone took 114.5 s);
  after, 2,752 s, with all 250 files passing (the spine case took 6.5 s). The
  other 18 timeouts are in tests that never call this validator, so the
  suite-level gap partly reflects that load. No timeout was raised.
