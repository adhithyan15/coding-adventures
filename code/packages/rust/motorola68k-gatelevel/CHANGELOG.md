# Changelog

## [0.1.0] — 2026-06-15

Initial release: gate-level Motorola 68000 (1979) simulator.

### Added

- `bits.rs` — integer ↔ LSB-first bit-vector conversion (8/16/32-bit);
  ripple-carry adders (`add_8bit_full`, `add_16bit_full`, `add_32bit_full`);
  flag helpers (`compute_v_from_carries`, `compute_n8/16/32`,
  `compute_z`/`compute_z8/16/32`); bitwise NOT helpers
  (`not_8bit`/`not_16bit`/`not_32bit`); NEG-specific helpers
  (`compute_c_neg`, `compute_v_neg`).

- `alu.rs` — `AluResult68K` struct (result + C/V/Z/N/X flags);
  `ShiftResult` struct (result + N/Z/V/C/X flags); size-dispatched ALU
  operations: `add8/16/32`, `sub8/16/32` (with borrow-in for SUBX),
  `neg8/16/32`, `negx8/16/32`, `and8/16/32`, `or8/16/32`, `xor8/16/32`,
  `not8/16/32_flags`, `cmp8/16/32`; shift/rotate `shift_op` covering
  ASL/ASR/LSL/LSR/ROXL/ROXR/ROL/ROR with ASL multi-bit V tracking.

- `registers.rs` — `RegisterFile68K` (D0–D7, A0–A7, PC, SR); sized
  read/write for Dn (byte/word/long, upper bits preserved); write_an
  with word sign-extension to 32 bits; CCR accessors
  (`flag_x/n/z/v/c`); flag update helpers (`set_ccr`, `set_nzvc_x`,
  `set_nz_clear_vc`, `negx_z`); `test_cc` for all 16 condition codes.

- `cpu.rs` — `Cpu68K` with 16 MB heap-allocated memory (`Vec<u8>`,
  big-endian byte order); big-endian memory helpers
  (`mem_read/write_byte/word/long`); fetch helpers
  (`fetch_word/long/word_signed`); stack helpers
  (`push_long/pop_long/push_word/pop_word`); all 14 effective-address
  mode resolver (`ea_address`/`ea_read`/`ea_write`/`ea_read_addr`);
  `exec_line0` through `exec_line_e` covering ~100 opcodes; `execute`
  (load + run up to N steps) and `step` (single instruction); 67 unit
  tests and 7 doc-tests (100% pass).

### Architecture notes

- Subtraction model: `A − B = A + NOT(B) + 1`; carry flag = NOT(carry-out)
  (borrow convention). SUBX injects X flag as borrow-in via `A + NOT(B) +
  NOT(X)`.
- ADDX/SUBX/NEGX Z-flag: AND(old_Z, result_Z) so Z is only cleared, never
  set by these instructions.
- NEG carry: `OR-reduction(result bits)` — C=1 iff result ≠ 0.
- Memory: 16 MB `Vec<u8>` (not a boxed array, to avoid stack overflow
  during construction).
- MUL/DIV: host arithmetic (gate-level ×16 booth multiplier out of scope).
- TRAP #15 used as soft halt; preserves SR so CCR flags remain readable
  after halting.
