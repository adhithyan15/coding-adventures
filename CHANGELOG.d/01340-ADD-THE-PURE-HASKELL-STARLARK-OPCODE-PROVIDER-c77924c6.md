### Add the pure Haskell Starlark opcode provider

- Added a missing non-Rust package with all 61 STP01 opcodes, all 195 decode
  holes, exact categories, immutable operator maps, and direct lookup helpers.
- Added a bounded Haskell reader that dynamically consumes all 359 neutral
  records and rejects hostile transport, shape, ordering, and semantic drift.
- Added native tests, production coverage, BUILD fronts, package documentation,
  and an explicit empty capability declaration without adding compiler or VM
  authority.
- Recovered Cabal 3.10.3.0 coverage planning by enabling coverage in the named
  project-package stanza. BUILD retains native coverage without the conflicting
  global flag; dependency bounds and inert production APIs remain unchanged.
- Added independent range, ordering, null-pair, depth and retained-map witnesses;
  all 13 native examples pass with production HPC at 447/447 expressions and
  61/61 alternatives. The Windows CI front's explicit skip is not native proof.
