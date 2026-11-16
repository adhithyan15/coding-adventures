## Unreleased

- **Security review of WD-100.** `renderJson` now refuses what the reader could
  never produce: `undefined`, functions and symbols, `NaN`, `Infinity`, fractions,
  `1e+21`, `-0` and integers beyond 2^53 - 1 throw a typed `CorpusError`
  (`render`) instead of a bare `TypeError` or an invalid document. `bigint`
  (`U64`) still renders as plain decimal.
- `relativeInside` refuses a POSIX path with exactly two leading slashes (and
  collapses three or more), matching the other lanes and VER04i 9.1.
- The fixture grew to 397 cases; the tagged test treats `unsupported-type` from
  `parseType` as a rejection (`List<T>`/`Option<T>` are outside the closed v1
  profile).
- `runner.src_sha256` changed; the `ct-compare` manifests are regenerated.
