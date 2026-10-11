### Fixed — the headline-word compose case runs one word per test

- `tests/headline-word.test.ts` › "composes words of letters whose printed
  headlines join" looped over nine words in one case. Each composition traces
  every stroke against the printed word's ink (about 0.1–0.26s idle), so the
  case took over 2s on an idle machine and crossed vitest's 5s per-test budget
  on a loaded runner while every word still composed. It is now one `it.each`
  case per word: each costs one word, and a failure names its word in the
  title. No timeout raised, no assertion changed.
