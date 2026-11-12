---
category: Security boundaries
---

# A lexical security analyzer must lex exactly as the compiler does, or fail closed where it cannot

The Rust capability analyzer (`code/packages/rust/ca-capability-analyzer`)
passed its 350-case evasion corpus. A round-3 review then broke it 13 ways.
Each break was a place where the analyzer's reading of the source differed
from rustc's:

- a shebang line `#!/x"` opened a "string" that hid real code;
- `char::is_whitespace` is not Rust's `Pattern_White_Space`. U+200E split
  `std::fs::write`;
- `is_alphanumeric` is not XID. U+00B7 shortened an `extern crate` alias;
- `cr#"` is a C string only from edition 2021;
- a `tt` metavariable can be `::` or `#`, so `std $s fs` and `# $a` build a
  path or an attribute the walker never sees;
- `mod tests;` in a root-level `build.rs` reaches a directory the scanner
  skips;
- a glob re-export through a local module (`m::process::Command` with
  `mod m { pub use std::*; }`) was resolved only as `std::m::process`.

Fix: copy the compiler's rule exactly where it is small (`strip_shebang`,
`Pattern_White_Space`, edition-gated prefixes). Where the analyzer cannot
match rustc (XID/NFC tables, macro expansion, module files), emit `CAP003`
for the construct instead of guessing. See VER08a divergence D25-D33 and
`tests/test_secreview_round3.rs`.

Do differently: for every lexical decision, ask "what does rustc do with
this exact byte sequence?" and test it with `cargo check` on a throwaway
crate. When a sweep of every package shows the stricter rule costs nothing,
prefer it. Here, 1,480 crates were swept and no exit code changed.
