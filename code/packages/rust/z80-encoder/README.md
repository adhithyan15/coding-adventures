# z80-encoder

Pure-Rust Zilog Z80 instruction encoder. Re-exports the `encode_*`
helpers and register/opcode constants `z80-backend` needs from
`z80-simulator::encoding` / `::opcodes`, so the backend depends on a
small, IR-agnostic surface without pulling in the full simulator crate's
decode/execute internals. Seventh lane of the 9-architecture expansion
(mirror of `intel8080-encoder` / `mips-r2000-encoder`).

## Byte-identity with `intel8080-encoder`

`encode_ld_a_n(n)` and `HALT` are byte-identical to
`intel8080_encoder::encode_mvi_a(n)` / `intel8080_encoder::HLT` — the Z80
(1976) is a source- and binary-compatible superset of the Intel 8080
(1974), and `LD A,n` (`0x3E imm`) / `HALT` (`0x76`) are both part of the
shared 8080-legacy opcode set. See
[`code/specs/z80-encoder.md`](../../../specs/z80-encoder.md) for the full
byte-identity table.
