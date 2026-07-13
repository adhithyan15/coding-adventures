# Changelog — `coding-adventures-oct-parser`

## 0.2.0 — 2026-07-13

### Fixed — recursion-depth guard against native stack overflow (DoS)

`create_oct_parser` built its `GrammarParser` with no recursion-depth cap,
even though `oct-dap` compiles whatever file is open in the editor being
debugged — a real, not theoretical, attack surface. Deeply-nested input
(`((((...))))`) would recurse until it overflowed the native thread stack —
an uncatchable process abort — before this crate's own `Result`-returning
entry points ever got a chance to report anything.

Rather than reuse `nib-parser`'s bespoke `MAX_RULE_DEPTH` unmeasured — this
crate's own module doc already notes it "mirrors the Nib parser's structure"
without claiming byte-identical compiled shape — this crate's floor was
measured independently the same way: binary-searching an *uncapped* parser
against increasing real nesting depth on a default-stack worker thread
(crashes at 31 levels, safe at 30; in rule-frame terms, safe through 285,
crashes at 290 on the same 5000-level adversarial input, matching
`nib-parser`'s measured floor closely). `nib-parser`'s value (200) was then
confirmed safe here too.

- Added `MAX_RULE_DEPTH: usize = 200` and wired it into `create_oct_parser`
  via `.with_max_depth(...)`.
- 3 new regression tests, mirroring `nib-parser`'s own: deep adversarial
  input on an enlarged-stack thread returns a clean `Err`, input at the
  measured real-nesting boundary (20 levels) still parses one level past it
  doesn't, and the cap trips before the native stack would overflow even on
  a default-stack thread.

No change to behaviour for any input that nests below the cap.

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
