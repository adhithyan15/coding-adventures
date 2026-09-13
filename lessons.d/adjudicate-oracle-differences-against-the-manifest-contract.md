# Adjudicate oracle differences against the manifest contract

A full Haskell-versus-Go Rust graph comparison initially showed two Haskell-only
edges. Deleting them to match the reference engine would have hidden valid Cargo
dependencies: each entry used a local source alias plus an authoritative
`package = "..."` published-name override. A reference engine is evidence, not
the specification. When parity comparison finds an extra edge, inspect the real
manifest and shared behavior contract before classifying it as false. If the
oracle is incomplete, add a language-neutral fixture and repair every affected
engine in the same dependency-shaped slice.
