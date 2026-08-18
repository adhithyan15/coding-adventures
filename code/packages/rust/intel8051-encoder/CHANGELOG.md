# Changelog — intel8051-encoder

## v0.1.0 — 2026-08-17 — initial carve-out (fourth lane, 9-architecture expansion)

### Added

- Re-exports of `encode_mov_a_imm`, `encode_mov_rn_imm`, `encode_halt`
  from `intel8051_simulator::encoding`.
- Opcode constants: `MOV_A_IMM` (`0x74`), `HALT_OPCODE` (`0xA5`).
- Capacity constant: `IMM8_MAX` (255).

### Tests

7 unit tests pin every constant and the canonical
`MOV A, #42; HALT = [0x74, 0x2A, 0xA5]` byte sequence the
`intel8051-backend` e2e smoke test relies on.
