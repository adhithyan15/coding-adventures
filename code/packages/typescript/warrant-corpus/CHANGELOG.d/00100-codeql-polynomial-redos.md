## [Unreleased]

### Security

- Fixed CodeQL `js/polynomial-redos` alert (`#310`) in
  `src/containment.ts` (`refusedComponent`) by replacing `.replace(/ +$/, "")`
  with an O(N) ASCII-space-only trailing-trim loop (`charCodeAt === 32`),
  preserving exact semantics (non-space whitespace such as `\t` is not
  stripped).
- Added ReDoS linear-time and tab-preservation regression tests in
  `tests/containment.test.ts`.
