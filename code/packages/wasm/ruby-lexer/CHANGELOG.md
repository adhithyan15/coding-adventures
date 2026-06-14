# Changelog

- 0.1.1 (2026-06-02): Fixed an unresolved-import build break. The Rust
  `ruby-lexer` crate is an era-aware state machine and no longer exposes the
  grammar-backed `create_ruby_lexer(...).tokenize()` shape; the wrapper now
  calls `tokenize_ruby(source)` (infallible, shares `lexer::token::Token`)
  before serializing to JSON.
- 0.1.0: Added WebAssembly bindings for the Rust ruby-lexer package.
