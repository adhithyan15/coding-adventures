# Changelog — unicode-math

All notable changes to the unicode-math pluggable frontend.

## [0.1.0] — 2026-06-30

### Added — PFE01 frontend #3: a Unicode plain-math reader

- New `unicode-math` crate: the **third** pluggable `MathFrontend` (after `latex` and
  `asciimath`). It reads the math people and models actually type with real glyphs and produces
  the **same** neutral `MathExpr`, so every existing consumer gains the notation with **zero**
  change — PFE01 pluggability now demonstrated across three genuinely different notations.
- **Tokenizer** (`token.rs`) — a total, panic-free **codepoint** scanner recording byte spans:
  numbers; single-letter variables; Greek / constant glyphs (`π`→`pi`, `Σ`→`Sigma`, `∞`→`infinity`,
  canonicalized to match the AsciiMath table); Unicode superscript runs (`x²`, `x⁻¹`, `x¹⁰`) and
  subscript runs (`a₁`); vulgar-fraction glyphs (`½ ⅓ ¼ ⅔ …`); operators `+ − × ⋅ ÷ ± ∓` and roots
  `√ ∛ ∜`; relations `= ≠ < ≤ > ≥ ≈ ≡`; brackets. An out-of-scope glyph (e.g. `∑`) is a clean
  spanned error, never a panic.
- **Parser** (`parser.rs`) — precedence-climbing recursive descent (relation < add < mul < frac <
  unary < script < atom), mirroring the AsciiMath frontend: implicit multiplication by
  juxtaposition (`2x`, `πα`), `MAX_DEPTH` nesting guard, and loop-built left-assoc chains so
  adversarial input fails cleanly instead of overflowing the stack.
- `capabilities()` advertises exactly the PR-1 surface — `fractions` (both `a/b` and vulgar
  glyphs), `roots`, `powers`, `relations`, `implicit_mul`, `plusminus` — and declares
  `functions`/`big_operators`/`matrices`/`text` **off** (PR-2), enforced by the shared
  `check_frontend` conformance harness.
- 23 unit + 1 doc test pass (incl. the conformance corpus); clippy `-D warnings` clean;
  `#![forbid(unsafe_code)]`. Spec: `PFE01-pluggable-parser-frontends.md`.
- **Out of scope (PR-2):** big operators (`∑ ∏ ∫`), named functions, matrices, embedded `\text`.
