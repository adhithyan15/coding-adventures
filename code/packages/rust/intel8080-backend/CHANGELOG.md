# Changelog — intel8080-backend

## v0.1.0 — 2026-08-17 — third lane of the 9-architecture expansion

Initial release. Minimal viable `Backend` trait impl over CIR.
Covers `const_*` + `ret_*` — compiles the trivial IIR program
`const 42; ret` to `[0x3E, 0x2A, 0x76]` (`MVI A, 42; HLT`), verified
byte-for-byte and by actually executing the bytes in
`intel8080-simulator`.

12 unit/integration tests pin every byte sequence and edge case.
