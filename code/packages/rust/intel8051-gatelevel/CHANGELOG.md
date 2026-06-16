# Changelog

## [0.1.0] — 2026-06-16

Initial release: gate-level Intel 8051 (1980) microcontroller simulator.

### Added

- `bits.rs` — integer ↔ LSB-first bit-vector conversion (`int_to_bits8`,
  `int_to_bits16`, `bits_to_u8`, `bits_to_u16`); 8-bit ripple-carry adder
  (`add_8bit_full`) returning full carry chain for CY/AC/OV flag extraction;
  16-bit ripple-carry adder (`add_16bit_full`) for PC increment and DPTR
  arithmetic; `invert_8bit` (8 NOT gates in parallel); `compute_parity`
  (7-gate XOR tree, **odd** parity — 1 when ACC has an odd number of 1-bits,
  matching 8051 PSW.P semantics); `compute_zero` (NOR tree).

- `alu.rs` — `AluResult8051` struct (`result`, `cy`, `ac`, `ov`, `parity`);
  full arithmetic operations: `add8` (CY=carry[7], AC=carry[3],
  OV=XOR(carry[6],carry[7])), `subb8` (A+NOT(B)+NOT(borrow);
  CY=NOT(carry_out)=borrow, AC=NOT(nibble_carry)=nibble_borrow),
  `inc8`/`dec8` (INC/DEC return cy=ac=ov=0 — 8051 never sets these for
  INC/DEC); logical ops `anl8`/`orl8`/`xrl8` (8 AND/OR/XOR gates;
  cy=ac=ov=0); rotate operations `rl8`/`rr8`/`rlc8`/`rrc8` (circular and
  carry-through variants); `swap8` (wire-level nibble exchange, no flags);
  `da8` (BCD decimal adjust with nibble-gt9 gate-level comparator and
  conditional adds); `mul8` (shift-and-add, 8 iterations, returns hi/lo/ov);
  `div8` (repeated subtraction, returns quotient/remainder/ov).

- `registers.rs` — `RegisterFile8051` with flat `iram: [u8; 256]` (covers
  lower RAM 0x00-0x7F and SFRs 0x80-0xFF) and `pc: u16`; `read/write_iram8`
  byte access; `read/write_pc`, `increment_pc` via gate-level `add_16bit_full`;
  `resolve_bit_addr` implementing the 8051 dual-range bit-address mapping
  (0x00-0x7F → byte 0x20+(addr>>3), 0x80-0xFF → byte addr&0xF8);
  `read_bit`/`write_bit` for individual-bit access.

- `cpu.rs` — `Cpu8051` with Harvard memory model (`code: Vec<u8>` 64 KB,
  `xdata: Vec<u8>` 64 KB, `rf: RegisterFile8051`); `new()`/`reset()`/`load()`/
  `execute()`/`step()` lifecycle; all helper methods: `fetch8`/`fetch16`,
  `rn_addr`/`rn`/`set_rn`, `acc`/`set_acc`/`update_parity`, `cy`/`set_cy`,
  `apply_alu_result`, `set_flags_cy_ac_ov`, `dptr`/`set_dptr`,
  `direct_read`/`direct_write`, `indirect_read`/`indirect_write`,
  `read_bit_addr`/`write_bit_addr`, `push8`/`pop8`/`push_pc`/`pop_pc`,
  `sign_extend_rel8`/`branch_by`; instruction dispatch covering ~100 opcodes
  across MOV, MOVC, MOVX, ADD, ADDC, SUBB, INC, DEC, MUL, DIV, DA, ANL, ORL,
  XRL, CLR, CPL, RL/RR/RLC/RRC, SWAP, XCH, XCHD, PUSH/POP, bit operations,
  all conditional jumps (JZ/JNZ/JC/JNC/JB/JNB/JBC/CJNE/DJNZ), LJMP/SJMP/
  AJMP/JMP, LCALL/ACALL/RET/RETI, NOP; 69 unit tests, 23 doctests (100% pass).

### Architecture notes

- Harvard CPU: three separate address spaces (code 64 KB, IRAM 256 B,
  XDATA 64 KB), never overlapping.
- SUBB model: `A − B − borrow = A + NOT(B) + NOT(borrow)`;
  CY = NOT(carry_out) = borrow flag.
- INC/DEC never modify CY, AC, or OV — only ACC value and parity.
- SWAP A does not update any PSW flag, not even parity.
- PSW.P is **odd** parity: P=1 when ACC has an odd number of 1-bits.
  (Contrast with Intel 8086 PF which is even parity.)
- da8: BCD adjustment uses a gate-level nibble-gt9 comparator
  `N>9 = b3 AND (b2 OR b1)` and conditional 6/0x60 additions.
- Halt sentinel: opcode 0xA5 (reserved/undefined on real 8051) causes
  `halted=true`; preserves all registers/flags for post-halt inspection.
- Stack: post-increment on push (SP++ then write), pre-decrement on pop
  (read then SP--), matching 8051 hardware convention.
- ACALL/AJMP push/replace only the lower 11 bits of PC; upper 5 bits
  come from the PC after the opcode byte fetch.
