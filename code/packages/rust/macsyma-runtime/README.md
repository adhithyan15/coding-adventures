# coding-adventures-macsyma-runtime

Rust MACSYMA runtime session facade over the statically linked MACSYMA compiler
and `symbolic-vm`.

This first slice is intentionally small: it compiles MACSYMA source, evaluates
statements through the Rust symbolic VM, preserves `;` versus `$` display
metadata, and records in-memory `%i`/`%o`-style history for a future REPL/WASM
facade.
