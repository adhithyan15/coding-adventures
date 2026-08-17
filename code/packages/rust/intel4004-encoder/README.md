# intel4004-encoder

Pure-Rust Intel 4004 instruction encoder.  Mirror of
`ge225-encoder` / `aarch64-encoder` for the Intel 4004 — the
**world's first commercial microprocessor** (1971).

## What's in it

- 4 opcode high-nibble constants: `LDM_OPCODE` (0xD0), `LD_OPCODE`
  (0xA0), `XCH_OPCODE` (0xB0), `JUN_OPCODE` (0x40)
- `HALT_LOOP = [0x40, 0x00]` (canonical `JUN 0x000` self-loop —
  the 4004 has no formal HLT)
- Capacity consts (`GP_REGISTER_COUNT`, `LDM_MAX`,
  `LDM_MIN_SIGNED`)
- 4 `encode_*` helpers: `encode_ldm`, `encode_ld`, `encode_xch`,
  `encode_jun`

No IR knowledge.  No `jit-core` dependency.

## See also

- [`intel4004-backend`](../intel4004-backend) — Phase 4 of the migration
- [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md) — phase plan (`iir-to-intel4004`, the deprecated predecessor, was removed once the migration completed)
