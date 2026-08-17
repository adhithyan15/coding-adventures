# Changelog — z80-backend

## v0.1.0 — 2026-08-17 — seventh lane of the 9-architecture expansion

Initial release. Minimal viable `Backend` trait impl over CIR.
Covers `const_*` + `ret_*` — compiles the trivial IIR program
`const 42; ret` to `[0x3E, 0x2A, 0x76]` (`LD A, 42; HALT`), verified
byte-for-byte and by actually executing the bytes in `z80-simulator`.

Byte-identical to `intel8080-backend`'s output for the same trivial CIR
program (asserted against a literal constant in `test_backend.rs`, since
the `intel8080-backend` crate is not yet present in this workspace
snapshot — see the `Cargo.toml` `[dev-dependencies]` note).

Termination checking tracks a real `HALT` emission via the CIR walk's
own control flow, never via a trailing-byte-value comparison against the
`HALT` opcode — sidesteps the Intel 8051 lane's bug class where a
`const_*` immediate numerically equal to the halt sentinel byte was
misread as "already terminated."

14 unit/integration tests pin every byte sequence and edge case,
including a `const 0x76` (the HALT opcode's own value) regression test.
