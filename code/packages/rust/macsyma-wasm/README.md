# coding-adventures-macsyma-wasm

JSON-oriented support facade over `coding-adventures-macsyma-runtime` for the
browser/WASM binding package.

This crate keeps the boundary intentionally simple: callers provide MACSYMA
source text and receive JSON strings containing display metadata, pretty
MACSYMA text, Lisp-style IR text, recursive IR JSON, and session history
indices. The actual `wasm-bindgen` exports live in
`code/packages/wasm/macsyma-runtime`.

## Robustness

`eval_json`/`eval_source_json` wrap `MacsymaSession::eval_source` in
`catch_parser_panics` (`catch_unwind`), so a panic anywhere in the reused
evaluation stack — `macsyma-runtime`'s own `eval_source` has no
`catch_unwind` boundary of its own — becomes a clean `{"ok": false,
"error": {...}}` JSON response rather than crashing the process. This is
where that boundary actually lives for Macsyma's real deployed surface.

This includes the shared `symbolic-vm::handlers::assign_handler`'s
self-referential-reassignment guard (`a: a * a` / `a: a + a`, repeated,
doubling the bound value's node count and/or nesting depth every step — a
security audit's finding; see `symbolic-vm`'s own README/changelog for the
full mechanism): Macsyma's own `:` assignment lowers straight to
`symbolic_ir::ASSIGN` and evaluates through that shared handler with no
crate-specific bypass, so it is protected automatically and surfaces here
as an ordinary clean error, not a crash.
