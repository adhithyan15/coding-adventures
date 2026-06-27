# Changelog — latex

All notable changes to the full-fidelity LaTeX parser crate.

## [0.1.0] — 2026-06-26

### Added — LTX01 L0: crate scaffold + catcode tokenizer

- New standalone, **zero-dependency** crate `latex` (added to the Rust workspace members).
  A full-fidelity LaTeX parser for documents *and* math; first frontend of the
  `math-frontend` framework.
- **`catcode(c)`** — TeX category codes (default plain-LaTeX assignments): Escape,
  BeginGroup, EndGroup, MathShift, AlignTab, EndLine, Parameter, Superscript, Subscript,
  Space, Letter, Other, Active, Comment.
- **`tokenize(&str) -> Result<Vec<Token>, LexError>`** — a catcode-driven, **text-mode-
  primary** state machine:
  - mode stack: Text (primary) ↔ Math (pushed by `$`/`\(`/`\[`, display via `$$`/`\[`,
    popped by the matching close); whitespace is significant in text, ignored in math;
  - control words (`\`+letters, with TeX space-absorption) and control symbols
    (`\`+non-letter, incl. `\\` line break, `\{`, `\,`, …);
  - groups `{ }`, math on/off (`MathOn`/`MathOff` with inline/display flag), `&`, `#`,
    `^`, `_`, active `~`, comments (`%` to end of line, eating the newline);
  - whitespace: a run collapses to one `Space`; a blank line (≥2 newlines) is `Par`;
  - ordinary characters emitted one-per-`Char` (faithful to TeX; the parser coalesces).
- **`Token` / `TokenKind` / `Span`** — every token carries a half-open byte span.
- **`LexError`** — spanned; the scanner **never panics** (trailing `\` → spanned error;
  a stray `\)`/`$` in text mode does not underflow the mode stack).
- 20 unit + 1 doc test; `cargo clippy -- -D warnings` clean; no `unsafe`.

### Notes

- Scope is full LaTeX surface; the Turing-complete TeX tail is the documented asymptote
  (see LTX01). The structural parser (L1), math AST (L2), environments (L3), macros (L4),
  text breadth (L5), and the `MathFrontend` adapter (L6) arrive in subsequent layers.
