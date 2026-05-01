# Changelog — twig-formatter

## [0.1.0] — 2026-04-30

Initial release.  Canonical Twig pretty-printer.  The first
authoring-experience deliverable: drop noisy whitespace arguments
at PR time, give every Twig file one canonical shape.

### Added

- `format(source) -> Result<String, FormatError>` — parses and
  formats.  The common case.
- `format_program(&Program, &Config) -> String` — formats an
  already-parsed AST.  Each top-level form is realised
  independently and joined with `\n\n` (intentional workaround for
  format-doc `fits()` lookahead through hardline separators).
- `program_to_doc(&Program) -> Doc` — emit a single Doc for the
  whole program (with documented caveat about per-form layout).
- `form_to_doc_pub(&Form) -> Doc` — emit Doc for one top-level
  form; useful for callers integrating into their own paint pipeline.
- `Config { print_width, indent_width }` — `Default` is 80 / 2.
- `FormatError` (`#[non_exhaustive]`, `Parse(TwigParseError)`).

### Style

- Atoms render as their source form (`42`, `#t`, `#f`, `nil`,
  `'foo`, `x`).
- Compound forms wrap content in `group()` so the realiser picks
  compact when it fits, indented multi-line otherwise.  Per-form
  indentation matches scheme/clojure conventions (`if` aligns
  children 4 cols past `(`; `let` aligns bindings 6 cols past `(`
  with body 2; `begin`/`lambda`/`define`/`apply` indent body or
  trailing args 2).
- Top-level forms separated by a single blank line.
- Output ends with a single trailing newline (POSIX text-file
  convention) for non-empty programs.

### Guarantees

- **Idempotency.**  `format(&format(s)?)? == format(s)?`.
- **Semantic preservation.**  `parse(&format(s)?)? == parse(s)?`
  (modulo source positions).
- **Determinism.**  Same input + same config → same output, byte-
  for-byte.

### Notes

- Pure data → text.  Two deps (`format-doc`, `twig-parser`), both
  capability-empty.  No I/O, no FFI, no unsafe.  See
  `required_capabilities.json`.
- Built on `format-doc`'s Wadler-style document algebra rather
  than hand-rolled layout heuristics — language-specific
  formatters that follow this pattern get prettier/rustfmt-quality
  decisions for free.
- 37 unit tests covering atoms, compact compound forms, block-form
  breaks for every Twig construct, `(define (f x) body)` lambda
  lowering round-trip, multi-form programs with blank-line
  separation, whitespace collapse, idempotency, semantic
  preservation, trailing-newline contract, narrow/wide/custom-
  indent config, unparseable-input error path, `program_to_doc`
  smoke test, factorial, nested let.
- Filed as follow-ups in the README roadmap: comment-preserving
  format (needs lexer trivia channel), format-range for LSP code
  actions, `twig fmt` CLI binary, `format-doc-to-paint`
  integration once the paint bridge ships.
- Security review: clean, no findings — recursion bounded by
  `twig-parser`'s `MAX_AST_DEPTH`, no panic surface, no
  capability leakage.
