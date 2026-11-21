## [Unreleased]

### Security

- Fixed CodeQL `js/polynomial-redos` alert (`#253`) in `src/book.ts` by
  replacing `/^(\d+)\.\s+(.*)$/.exec(line)` with an O(N) prefix match
  (`/^\d+\.\s+/`) and `line.slice(orderedPrefix[0].length)` (guarded against
  `\n`/`\r` for exact `$` parity).
- Added ReDoS regression test (`"0.\t" + "\t\t".repeat(25_000)`) in
  `tests/book.test.ts`.
