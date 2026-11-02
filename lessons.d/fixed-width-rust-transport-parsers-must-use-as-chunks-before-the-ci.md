---
category: Rust
---

# Fixed-width Rust transport parsers must use as_chunks before the CI toolchain sees them

The PDP-10 transport decoder reached CI with `chunks_exact(5)` even though an
earlier simulator PR had already exposed Rust 1.99's
`chunks_exact_to_as_chunks` warning under `-D warnings`.  Fixing only the first
package left the same constant-width pattern available to fail a later macOS
lane.

After any toolchain-driven lint failure, search every changed Rust package for
the rejected pattern before repushing.  For a transport whose divisibility is
validated first, iterate `bytes.as_chunks::<N>().0`; that both satisfies the
new lint and expresses the fixed-width record in the item type.  A local
Clippy pass on an older toolchain is not enough—use the CI diagnostic as a
repo-wave search key and repair every matching touched package.
