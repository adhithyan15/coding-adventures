# Changelog — z80-backend

## Unreleased — WORD02 two-live arithmetic

- Allocate two same-width live values in `A`/`D` or `HL`/`DE` and reuse dead
  operand slots; reject a third live value and mixed live widths.
- Execute wrapping byte/word add, subtract, and bitwise operations in the Z80
  simulator, including `ADD HL,DE`, carry/borrow, and both result bytes.
- Validate CIR result types and instruction shapes, including Boolean return
  provenance, before emitting code.

## Unreleased — WORD01 fixed-width result ABI

- Lower `const_u8`/`ret_u8` and `const_bool`/`ret_bool` through `A`.
- Lower `const_u16`/`ret_u16` through `HL` using `LD HL,nn`.
- Track the current value's width and reject mismatched typed returns while
  retaining the single-live-value restriction.
- Execute the discriminating `0x1234` result proof in `z80-simulator` and
  inspect both `H` and `L`; also pin the `u8` boundary and overflow.

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
