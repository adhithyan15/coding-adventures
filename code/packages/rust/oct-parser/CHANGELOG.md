# Changelog — `coding-adventures-oct-parser`

## 0.1.0 — 2026-05-20 (OCT02 phase 1)

Initial Rust port of the Oct parser.  Wraps the generic `GrammarParser`
over the auto-generated `oct.grammar` source (compiled to native Rust
data structures via the `grammar-tools` CLI).

This is the second half of OCT02 phase 1.  The
`coding-adventures-oct-lexer` crate produces the token stream and this
crate arranges it into a grammar AST rooted at `program`.  Subsequent
OCT02 phases consume this AST:

- Phase 2: `oct-type-checker` (Rust port of the Python type-checker).
- Phase 3: `oct-iir-compiler` (new — emits `interpreter_ir::IIRModule`).
- Phase 4: `lang-aot` wiring + end-to-end smoke test.

### Tests

10 unit tests cover:

- Minimal `fn main() {}`.
- `let` with type annotation.
- `return` with a binary expression.
- `if/else` blocks.
- `while` and `loop`/`break`.
- Intrinsic calls (`out(...)`).
- User-defined function calls.
- `static` declarations.
- Expression precedence (`+` below `==`).
- Syntax-error rejection (missing brace).
