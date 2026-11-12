## Unreleased - 2026-11-12 - Clippy 1.100 needless_bool fix

### Fixed

- Rewrote the `proc_call` standard-function guard so the early `return false`
  is no longer an `if`-returns-bool-literal-then-`true` pair (clippy
  `needless_bool`, Rust 1.100). Behaviour is unchanged.
