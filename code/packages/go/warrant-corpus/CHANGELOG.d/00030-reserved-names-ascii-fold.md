## Unreleased

- **WD-100b: reserved device names and ASCII-only win32 folding.** `relativeInside`
  compared win32 names with `strings.EqualFold` (Unicode simple folding: U+017F
  equalled `s`, U+212A equalled `k`) and lower-cased the drive and UNC anchor with
  `strings.ToLower`. Both now use `asciiLower` (`A`..`Z` only), so a non-ASCII case
  variant is "not inside" (fails closed). A win32 child with a reserved device name
  (`CON`, `PRN`, `AUX`, `NUL`, `COM1`..`COM9`, `LPT1`..`LPT9`, with or without an
  extension) or a component ending in a dot or space is outside (`refusedComponent`).
- The shared fixture grew to 440 cases (43 new containment cases); all pass.
- `runner.src_sha256` changed; the `ct-compare` manifest is regenerated.
