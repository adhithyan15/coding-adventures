# Changelog — intel8086-encoder

## [0.1.0] - 2026-08-17

### Added

- Pure-Rust encoder re-exporting the `encode_*` helpers from
  `intel8086_simulator::encoding` (the in-tree source of truth for this
  lane's curated Intel 8086 opcode subset): `encode_mov_reg_imm16`,
  `encode_mov_reg_imm8`, `encode_mov_reg_reg16`, `encode_add_ax_imm16`,
  `encode_sub_ax_imm16`, `encode_inc_reg16`, `encode_dec_reg16`,
  `encode_nop`, `encode_hlt`, `assemble`.
- `HALT_BYTE` constant (`0xF4`) — the `HLT` opcode byte.
- `REG_AX` re-export — the accumulator register index, so
  `intel8086-backend` doesn't need a direct `intel8086-simulator`
  dependency.
- 6 unit tests + 1 doctest verifying the constants and canonical byte
  sequences (`MOV AX,42` → `[0xB8, 42, 0x00]`, `HLT` → `[0xF4]`,
  register-to-register `MOV` ModRM encoding).

Ninth and final lane of the 9-architecture expansion following the
pattern documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
