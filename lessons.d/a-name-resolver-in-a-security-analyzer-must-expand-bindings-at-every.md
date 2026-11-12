---
category: Security boundaries
---

# A name resolver in a security analyzer must expand bindings at every path segment to a fixed point, not only the head

The Rust capability analyzer (`code/packages/rust/ca-capability-analyzer`)
passed its round-3 review, and a second review then broke it with two-level
renames:

```rust
mod a { pub use std as s; }
use a::s as t;
t::env::var("HOME")          // read as a::s::env::var: pure
```

The binding table expanded a target only through its *first* segment
(`a` is not a binding), and a written path was substituted once. The spec
even claimed "same-named bindings elsewhere are followed" (G-8), which was
false for any target not headed by the binding name. The same review found
neighbouring holes of one kind: `Self` never resolved, `type P = (Path);`
bound nothing, an `ident` macro argument supplied `use`, and
`[lib] path = "src/lib.txt"` compiled a file the `.rs`-only scanner skipped.

Fix: close the table under substitution at every segment, to a fixed point.
Track, per spelling, which original bindings it came from and never reuse
one (rustc never resolves through the same import twice). Without that,
self-similar names such as `pub use self::sha256::sha256;` grow forever.
Check every cap inside the loop and turn overflow into CAP003. See VER08a
D34-D42 and `tests/test_secreview_round4.rs`.

Do differently: when a resolver's doc says "X is followed", write the
two-hop test (`a -> b -> std`) that proves it. For every silent `None`
or "no binding" branch in a parser of untrusted input, ask whether it
should be CAP003 instead. Sweep the whole repo before and after: here 1,480
crates kept their exit codes.
