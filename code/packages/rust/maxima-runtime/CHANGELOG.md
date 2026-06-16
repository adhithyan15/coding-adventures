# Changelog — coding-adventures-maxima-runtime

All notable changes to this package are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
Semantic Versioning.

## [0.1.0] — 2026-06-16

### Added

- Initial release. `MaximaSession` — a presentation façade that wraps a
  `macsyma_runtime::MacsymaSession` and exposes Maxima's string-in/string-out
  console contract. `feed(src)` evaluates a chunk of Maxima source and returns
  one `(%o«n») «text»` echo line per **displayed** result (`;` displays, `$`
  suppresses but still advances the `%o` history counter). A surface/parse error
  is returned as `Err(String)` via the macsyma evaluator's `Display`.
- One-shot `eval(src)` convenience on a fresh session.
- **Panic-safety at the trust boundary:** `feed` runs evaluation inside
  `catch_unwind`, converting the underlying `macsyma-lexer`'s panic-on-bad-input
  into a clean `Err` so a single stray character cannot abort an interactive
  session. Documented as a defensive shim pending an upstream lexer fix.

### Notes

- Maxima is the GPL descendant of DOE Macsyma with an identical algebraic surface
  for the supported subset, so this crate adds **no evaluation logic of its own**
  — it reuses the entire macsyma frontend/VM/`cas-*` stack unchanged (the
  symbolic-CAS analogue of Octave-over-MATLAB). It therefore inherits exactly
  macsyma-runtime's evaluation power. See `code/specs/MA03-maxima-language.md`.
