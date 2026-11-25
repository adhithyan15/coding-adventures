## Unreleased

- Add `tests/arithmetic_proofs.rs` (VER04h, arithmetic), a copy of `logic_gates_proofs.rs` with the package constants changed: the exports live in two files, so it feeds `adders.rs` and `alu.rs` to the front end separately and looks each export up in both. The front end refuses both (the `(u8, u8)` return type, tuple patterns, the `enum` item), so every steps-bound obligation is recorded `not-live` with its own message (backlog ARITH-001); `depth` rows are `unchecked`.
