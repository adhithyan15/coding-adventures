# coding-adventures-macsyma-wasm

JSON-oriented support facade over `coding-adventures-macsyma-runtime` for the
browser/WASM binding package.

This crate keeps the boundary intentionally simple: callers provide MACSYMA
source text and receive JSON strings containing display metadata, pretty
MACSYMA text, Lisp-style IR text, recursive IR JSON, and session history
indices. The actual `wasm-bindgen` exports live in
`code/packages/wasm/macsyma-runtime`.
