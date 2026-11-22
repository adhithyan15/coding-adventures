## Unreleased

- **WD-100b: reserved device names and ASCII-only win32 folding.** `relative_inside`
  no longer compares `PureWindowsPath` objects (whose equality folds with the full
  Unicode `lower`, so the Kelvin sign equalled `k`). It reduces each path to an
  anchor and whole components, normalised lexically, and under `win32` compares them
  after `ascii_fold` (`A`..`Z` to `a`..`z`, nothing else). A `win32` child with a
  reserved device name (`CON`, `PRN`, `AUX`, `NUL`, `COM1`..`COM9`, `LPT1`..`LPT9`,
  with or without an extension) or a component ending in a dot or space is outside.
  New public helpers: `ascii_fold`, `refused_component`, `RESERVED_NAMES`.
- The shared fixture grew to 440 cases (43 new containment cases); all pass, and a
  test drives the production (`HOST_FLAVOR = win32`) path.
- `runner.src_sha256` changed; the `ct-compare` manifest is regenerated.
- Not done here: the every-host artifact-path acceptance in the spec text (the lane
  has no artifact-path loader; the Rust loader is that consumer).
