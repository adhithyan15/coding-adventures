# Changelog

## 0.1.0 (2026-05-16)

Initial release — Layer 07u2 in the coding-adventures simulator series.

### Added

- `PowerPC601GateLevelSimulator` implementing the SIM00 protocol:
  - `reset()`, `load()`, `step()`, `execute()`, `get_state()`
  - I/O stubs: `set_input_port()`, `get_output_port()`, `interrupt()`, `nmi()`
- `PowerPC601State` frozen dataclass with:
  - 32-element `gpr` tuple (GPR0–GPR31)
  - `lr`, `ctr`, `xer`, `cr`, `pc` (CIA)
  - `halted` flag and full `memory` tuple (65536 bytes)
- `StepTrace` local dataclass with `pc_before`, `pc_after`, `mnemonic`, `detail`
- **Gate-level data path**: all arithmetic routes through AND/OR/XOR/NOT gates
  and `ripple_carry_adder` from the `logic_gates` library — no Python `+/-/*`
  operators in the execution data path
- **`RegisterFilePPC`** (register_file.py):
  - 32 GPRs + LR, CTR, XER, CR, CIA as LSB-first 32-bit lists (flip-flop banks)
  - `set_cr_field()` using gate-level AND/OR/NOT for 4-bit nibble update
  - `get_cr_bit()` using gate-level AND isolation
  - `increment_cia()` using `ripple_carry_adder`
- **`alu.py`** — gate-level ALU operations:
  - `add32`, `sub32`, `adde32`, `neg32`
  - `and32`, `or32`, `xor32`, `nand32`, `nor32`, `eqv32`, `andc32`, `orc32`
  - `slw32`, `srw32`, `sraw32`, `srawi32`, `cntlzw32`
  - `mul32_lo`, `mul32_hi_unsigned`, `mul32_hi_signed` (64-bit shift-and-add)
  - `divwu`, `divws` (binary long-division)
  - `compare_signed`, `compare_unsigned`
- **`bits.py`** — gate-level bit utilities:
  - `int_to_bits`, `bits_to_int` (LSB-first conversion)
  - `add_32bit` wrapping `ripple_carry_adder`
  - `shl_32`, `shr_32_logical`, `shr_32_arithmetic`
  - `compute_zero`, `rlw32` (rotate-left-word)
  - `sext16`, `sext32`
- **`decoder.py`** — combinational 32-bit instruction decoder:
  - All PowerPC 601 instruction formats: I, B, D, X, XO, XFX, XL, M
  - SPR split-field decoding
  - Mnemonic assignment for all implemented opcodes
- **Full PowerPC 601 instruction set**:
  - Arithmetic: ADD, ADDC, ADDE, ADDME, ADDZE, ADDI, ADDIS, ADDIC, ADDIC.
  - Subtract: SUBF, SUBFC, SUBFE, SUBFME, SUBFZE, SUBFIC
  - Multiply: MULLW, MULHW, MULHWU
  - Divide: DIVW, DIVWU
  - Negate: NEG
  - Compare: CMP, CMPI, CMPL, CMPLI
  - Logic: AND, OR, XOR, NAND, NOR, EQV, ANDC, ORC + immediate variants
  - Shift/rotate: SLW, SRW, SRAW, SRAWI, CNTLZW, RLWIMI, RLWINM, RLWNM
  - Load: LWZ, LWZU, LWZX, LWZUX, LBZ, LBZU, LBZX, LBZUX, LHZ, LHZU,
           LHZX, LHZUX, LHA, LHAU, LHAX, LHAUX, LMW
  - Store: STW, STWU, STWX, STWUX, STWCX., STB, STBU, STBX, STBUX,
           STH, STHU, STHX, STHUX, STMW
  - Branch: B, BL, BA, BLA, BC, BCLR, BCCTR
  - CR logical: CRAND, CRNAND, CROR, CRNOR, CRXOR, CREQV, CRANDC, CRORC, MCRF
  - SPR: MTSPR, MFSPR (LR=8, CTR=9, XER=1), MFCR, MTCRF
  - Sync: ISYNC (NOP), LWARX (load-and-reserve, reserve flag set)
- Halt sentinel: 32-bit word `0x00000000`
- XER SO/OV/CA flag propagation through overflow-enabled arithmetic (OE=1)
- Rc=1 support: CR0 updated after arithmetic/logic results
- 65536-byte flat big-endian memory
- Full test suite: protocol tests, instruction unit tests, program tests,
  and cross-equivalence tests vs. behavioral simulator (316 tests, 87% coverage)
- `py.typed` marker for PEP 561 compliance
- Literate code comments throughout with gate-level diagrams and analogies
