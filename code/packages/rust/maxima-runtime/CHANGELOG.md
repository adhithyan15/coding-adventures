# Changelog — coding-adventures-maxima-runtime

All notable changes to this package are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
Semantic Versioning.

## [0.1.1] — 2026-07-12

### Fixed

- `moderate_nesting_still_evaluates` used 40 levels of `(...)` grouping, which
  now exceeds the macsyma-parser recursion-depth guard (wired in upstream by
  the security fix in #7928 — the guard trips at ~14 real grouping levels).
  Lowered the test to 10 levels so it stays comfortably under the cap while
  still proving that modestly nested input evaluates normally.

## [0.1.0] — 2026-06-16

### Added

- Initial release. `MaximaSession` — a presentation façade that wraps a
  `macsyma_runtime::MacsymaSession` and exposes Maxima's string-in/string-out
  console contract. `feed(src)` evaluates a chunk of Maxima source and returns
  one `(%o«n») «text»` echo line per **displayed** result (`;` displays, `$`
  suppresses but still advances the `%o` history counter). A surface/parse error
  is returned as `Err(String)` via the macsyma evaluator's `Display`.
- One-shot `eval(src)` convenience on a fresh session.
- **Robustness at the trust boundary** (`feed` takes arbitrary user text):
  - *Unwinding panics* — evaluation runs inside `catch_unwind`, converting the
    underlying `macsyma-lexer`'s panic-on-bad-input into a clean `Err` so a stray
    character cannot abort the session.
  - *Stack-overflow from unbounded parser/VM recursion* — deeply nested input
    (nested `(`, prefix `-` runs, long `1+1+…` chains) would overflow the stack
    and **abort the process uncatchably**. Guarded by a total-size cap
    (`MAX_INPUT_LEN`), a per-statement token cap (`MAX_STATEMENT_TOKENS`) counted
    from the **real macsyma lexer** (so comment/string skip rules can't be used
    to bypass it), and a large-stack worker thread that also builds the echo, so
    bounded trees are created and dropped clear of the caller's stack.
  - *Mutex poisoning* — after any caught panic the wrapped session is rebuilt, so
    a panic inside a lock-holding macsyma handler can't permanently brick it.

### Notes

- Maxima is the GPL descendant of DOE Macsyma with an identical algebraic surface
  for the supported subset, so this crate adds **no evaluation logic of its own**
  — it reuses the entire macsyma frontend/VM/`cas-*` stack unchanged (the
  symbolic-CAS analogue of Octave-over-MATLAB). It therefore inherits exactly
  macsyma-runtime's evaluation power. See `code/specs/MA03-maxima-language.md`.
