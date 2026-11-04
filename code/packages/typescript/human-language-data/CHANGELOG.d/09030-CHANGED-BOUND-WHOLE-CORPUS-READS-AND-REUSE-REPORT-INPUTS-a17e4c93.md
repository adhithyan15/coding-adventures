### Changed — bound whole-corpus reads and reuse report inputs on Windows

- Add `loadEverythingAsync`, an awaitable equivalent of the complete loader
  that uses the existing 16-read bounded lesson path while retaining the same
  sorted order, source-byte cache key, metadata guards, and result shape.
- Let the gentle-ramp compatibility derivation accept already-loaded canonical
  inputs for its second independent full-report calculation. The historical
  no-argument loader still checks the retired snapshot path and reads all
  canonical owners itself.
- Use registry-only loading in the shared progression identity test instead
  of constructing a 63,000-lesson corpus that the assertion never reads.
- Keep the 30-second default, explicit per-case timeouts, every corpus
  assertion, and the 85% line-coverage threshold unchanged. Full Windows
  coverage and final-head CI remain required before release.
