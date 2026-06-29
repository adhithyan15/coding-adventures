# Changelog — asciimath

All notable changes to the AsciiMath pluggable frontend.

## [0.1.0] — 2026-06-28

### Added — ASM01 PR-1: the AsciiMath frontend (core subset)

- New standalone crate `asciimath` (added to the Rust workspace members), the **second**
  pluggable frontend after `latex` (see ASM01 / PFE01). Depends only on `math-frontend`.
- **`AsciiMath`** implements `math_frontend::MathFrontend`: `name() == "asciimath"`,
  `parse(src) -> Result<MathExpr, FrontendError>`, and an honest `capabilities()`. Free
  functions `parse` and `tokenize` are also public.
- **Tokenizer** (`token.rs`): numbers (exact), identifiers (maximal letter runs), the
  operators `+ - * / ^ _ = < >` and multi-char `<= >= != ~~ -= -:`, brackets `( ) [ ] { }`,
  and `"…"` text literals; whitespace skipped. Spanned, total, panic-free.
- **Parser** (`parser.rs`): precedence-climbing relation < add/sub < mul (incl. juxtaposition
  and `xx`/`cdot`/`-:`) < frac (`/`) < unary < scripts (`^`/`_`), over atoms: numbers,
  symbols, function application (`sin`, `ln`, …), `sqrt`/`root(n)(x)`, groups, and `"text"`.
  Identifiers classify as function / known constant (`pi`, `theta`, … , `oo`→`infinity`) /
  operator word / else a product of single-letter symbols (`xy` ⇒ `x·y`). Lowers directly to
  the neutral `MathExpr`; `1/2` ≡ LaTeX's `\frac{1}{2}`.
- **Total / panic-free / bounded:** every input yields `Ok` or a spanned `FrontendError`;
  recursion is depth-guarded (`MAX_DEPTH`) and left-associative chains are built with loops,
  so deep nesting and long chains can't overflow the parser stack.
- **Capabilities** advertised: `fractions, roots, powers, functions, relations, implicit_mul,
  text` (matrices / big-operators are off — PR-2). Conforms to the shared `check_frontend`
  harness (no panics, valid spans, no capability over-claim) + notation-specific goldens.
- Spec `code/specs/ASM01-asciimath-frontend.md`; documented PR-1 limitations + roadmap (§5).
- No `unsafe`; `cargo clippy -- -D warnings` clean.
