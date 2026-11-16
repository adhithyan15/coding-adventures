## Unreleased

- **WD-100b: reserved device names and ASCII-only win32 folding.** `relativeInside`
  no longer calls `path.relative` (`path.win32.relative` compares with the full
  Unicode `toLowerCase`, so the Kelvin sign U+212A equalled `k`). It normalises both
  paths with `resolve`, splits them with `parse` into an anchor and whole components
  and compares those itself; under win32 only `A`..`Z` fold (`asciiFold`). A win32
  child with a reserved device name (`CON`, `PRN`, `AUX`, `NUL`, `COM1`..`COM9`,
  `LPT1`..`LPT9`, with or without an extension) or a component ending in a dot or
  space is outside (`refusedComponent`). `PathModule` now needs `parse`, `resolve`
  and `sep` only. The relative text keeps its original case and the module's separator.
- The shared fixture grew to 440 cases (43 new containment cases); all pass.
- `runner.src_sha256` changed; the `ct-compare` manifest is regenerated.
