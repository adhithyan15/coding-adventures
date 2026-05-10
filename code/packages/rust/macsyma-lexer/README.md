# coding-adventures-macsyma-lexer

MACSYMA tokenizer backed by `code/grammars/macsyma/macsyma.tokens`, compiled to
Rust and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it suitable
for a future WASM facade.
