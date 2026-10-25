### Add the pure Haskell Starlark opcode provider

- Added a missing non-Rust package with all 61 STP01 opcodes, all 195 decode
  holes, exact categories, immutable operator maps, and direct lookup helpers.
- Added a bounded Haskell reader that dynamically consumes all 359 neutral
  records and rejects hostile transport, shape, ordering, and semantic drift.
- Added native tests, production coverage, BUILD fronts, package documentation,
  and an explicit empty capability declaration without adding compiler or VM
  authority.
