# intel4004-encoder

Pure-Rust Intel 4004 instruction encoder.  Mirror of
`ge225-encoder` / `aarch64-encoder` for the Intel 4004 — the
**world's first commercial microprocessor** (1971).

## What's in it

- Opcode constants for register operations plus RAM addressing and data I/O.
- `HALT_LOOP = [0x40, 0x00]` (canonical `JUN 0x000` self-loop —
  the 4004 has no formal HLT)
- Capacity consts (`GP_REGISTER_COUNT`, `LDM_MAX`,
  `LDM_MIN_SIGNED`)
- `encode_*` helpers for `LDM`, `LD`, `XCH`, `JUN`, `FIM`, `SRC`, `DCL`,
  main-memory `WRM`/`RDM`, and status-character `WR0..3`/`RD0..3`.

No IR knowledge.  No `jit-core` dependency.

## See also

- [`intel4004-backend`](../intel4004-backend) — Phase 4 of the migration
- [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md) — phase plan (`iir-to-intel4004`, the deprecated predecessor, was removed once the migration completed)
