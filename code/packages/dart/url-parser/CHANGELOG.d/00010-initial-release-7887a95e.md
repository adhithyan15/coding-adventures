## 0.1.0

- Initial Dart lane of the url-parser identity: `Url.parse` (authority and
  `scheme:path` forms), RFC 1808 `resolve`, `effectivePort`/`authority`/
  `toUrlString`, identity `canonicalize`, and `percentEncode`/`percentDecode`
  with the six-variant `UrlError`. Zero runtime dependencies.
- Faithful quirks (all pinned by tests): raw-substring components, first-`#`
  fragment split, unthrown `emptyHost`/`relativeWithoutBase`, host-kept
  colons, lossy no-host serialization, merging query-only references, and
  input-covering equality. Dart adaptations: thrown errors, lone-surrogate
  rejection, and a BOM-safe percent decoder (Dart strips leading BOMs).
- Tests mirror the reference suite (87 unit tests), add 7 seeded property
  tests over the in-lane `pbt` package and a 2,000-iteration fuzz campaign
  over the in-lane `fuzz` package that caught one real pre-release bug (BOM
  stripping); 99.7% line, 99.1% branch, 97.1% function coverage.
