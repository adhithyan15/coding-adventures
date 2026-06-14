# coding-adventures-macsyma-parser

MACSYMA parser backed by `code/grammars/macsyma/macsyma.grammar`, compiled to
Rust and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it suitable
for a future WASM facade.
