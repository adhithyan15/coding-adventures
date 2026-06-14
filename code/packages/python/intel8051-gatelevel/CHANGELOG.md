# Changelog — coding-adventures-intel8051-gatelevel

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [1.0.0] — 2026-05-15

### Added

Initial release of the Intel 8051 gate-level behavioral simulator.

#### Core modules

- **`bits.py`** — The sole bridge layer between Python integers and gate-level
  bit arrays. Provides `int_to_bits`, `bits_to_int`, `add_8bit`, `add_16bit`,
  `invert_8bit`, `compute_parity`, and `compute_zero`. All Python arithmetic
  operators are confined to this file; no other source file in the data path
  uses `+`, `-`, `*`, `/`, `&`, `|`, `^`, or `~`.

- **`alu.py`** — Gate-level ALU implementing every 8051 data-path operation:
  - `add8` / `addc8` — 8-bit addition with optional carry-in, computes CY,
    AC, OV, and parity flags by routing through `ripple_carry_adder`.
  - `subb8` — 8-bit subtraction with borrow using two's-complement via NOT
    gates and an adder chain.
  - `anl8`, `orl8`, `xrl8` — Bitwise AND, OR, XOR through gate arrays.
  - `rl8`, `rr8`, `rlc8`, `rrc8` — All four 8-bit rotate operations.
  - `inc8`, `dec8` — Increment and decrement using the adder.
  - `da8` — Decimal Adjust Accumulator (BCD correction) with gate trees
    detecting nibbles > 9 via `AND(b3, OR(b2, b1))`.
  - `mul8` — 8×8 unsigned multiply by 8 iterations of conditional add
    (long multiplication in hardware).
  - `div8` — 8÷8 unsigned divide by repeated subtraction (restoring
    division).
  - OV flag detection for ADD/SUBB: runs two separate `ripple_carry_adder`
    calls (7-bit and 8-bit) and XORs the carry results.

- **`register_file.py`** — Harvard register file. Stores all 256 bytes of
  IRAM as bit arrays (simulating flip-flops) and maintains a 16-bit PC as a
  bit array. PC increment uses `add_16bit` from the bits bridge. Provides
  byte-level IRAM access, bit-addressable read/write, bulk load/dump, and
  PC read/write/increment.

- **`decoder.py`** — Gate-tree instruction decoder. Uses `AND`, `OR`, `NOT`
  gates on the 8 bits of each opcode to classify it into a mnemonic string.
  Design principle: fully-specified 8-bit patterns are matched before
  family-range patterns (which match only bits[7:3]). AJMP and ACALL are
  matched last to avoid false positives.

- **`simulator.py`** — `Intel8051GateLevelSimulator` implementing the
  `Simulator[I8051State]` protocol. Supports:
  - Harvard memory model: 64 KB code ROM, 256-byte IRAM, 64 KB XDATA.
  - All arithmetic instructions: ADD, ADDC, SUBB, INC, DEC, MUL, DIV, DA.
  - All logical instructions: ANL, ORL, XRL, CLR, CPL, RL, RR, RLC, RRC.
  - All data transfer instructions: MOV (register, direct, indirect,
    immediate), MOVX (@Ri, @DPTR), MOVC (@A+DPTR, @A+PC), XCH, XCHD,
    PUSH, POP, MOV DPTR.
  - All branch instructions: SJMP, LJMP, AJMP, JMP @A+DPTR, JZ, JNZ, JC,
    JNC, JB, JNB, JBC, CJNE, DJNZ.
  - Subroutine instructions: LCALL, ACALL, RET, RETI.
  - Bit operations: SETB, CLR, CPL (bit), ANL C, ORL C, MOV C (bit ↔ C).
  - HALT (0xA5) stops execution.
  - `execute()` catches instruction-level exceptions and stores them in
    `ExecutionResult.error`; `step()` propagates them directly.

#### Test suite — 370 tests, 90.4% coverage

- **`test_bits.py`** — 15 tests covering the bits bridge layer including
  edge cases (carry propagation, parity of all-zeros, zero detection).
- **`test_alu.py`** — 80 tests covering every ALU operation with flag
  verification. Includes OV flag edge cases (signed overflow boundary),
  DA A BCD correction (both nibbles, CY propagation), MUL/DIV with
  remainders and overflow flag.
- **`test_register_file.py`** — 25 tests covering IRAM read/write,
  PC increment/wraparound (0xFFFF→0x0000), bit-addressable regions in
  lower RAM (0x20-0x2F) and SFR space (0x80+), and bulk load/dump.
- **`test_decoder.py`** — 73 tests covering all instruction families and
  representative specific opcodes. Verifies that family patterns (Rn
  variants 0-7) all decode correctly and that AJMP/ACALL are not
  misidentified.
- **`test_equivalence.py`** — 10 cross-validation tests running identical
  programs on both `intel8051_simulator.I8051Simulator` (behavioral
  reference) and `Intel8051GateLevelSimulator`, asserting identical final
  state for ACC, IRAM, PC, and flags.
- **`test_programs.py`** — 20 end-to-end program tests including: sum
  1..10 via DJNZ loop, MUL 12×17=204, DIV 100÷7=14 remainder 2, DA A
  BCD addition, bit-addressable JB branching, PUSH/POP stack, LCALL/RET,
  CJNE, and MOVC table lookup.
- **`test_coverage.py`** — 81 targeted tests raising line coverage above
  80% by exercising: all MOV @Ri addressing modes, ADD/ADDC/SUBB with
  @Ri and direct operands, all logical op direct variants, DJNZ dir,
  CJNE Rn/#imm and @Ri/#imm, ACALL, RETI, XCHD, AJMP, JMP @A+DPTR,
  all bit operations (CPL, ANL C, ORL C, MOV C,bit), MOVC @A+PC,
  indirect-address error propagation through `step()` vs `execute()`.

### Design decisions and notes

- **Gate-level constraint**: No Python arithmetic operator (`+`, `-`, `*`,
  `/`, `//`, `%`, `&`, `|`, `^`, `~`) appears in `alu.py`,
  `register_file.py`, or the execution path of `simulator.py`. All
  arithmetic routes through `ripple_carry_adder` from the `arithmetic`
  package; all bitwise operations use `AND`, `OR`, `XOR`, `NOT` from
  `logic_gates`.

- **Overflow detection**: Rather than using a single 8-bit adder and
  checking bit positions, OV is computed by running two adder chains:
  a 7-bit add to obtain the carry INTO bit 7, and the normal 8-bit add
  for carry OUT of bit 7. `OV = XOR(carry_into_7, carry_out)`. This
  mirrors actual hardware.

- **DA A nibble detection**: The gate tree `AND(b3, OR(b2, b1))`
  identifies any 4-bit value in the range 10–15 (all have b3=1 and at
  least one of b2/b1 set). Values 0–9 have b3=0 or both b2=b1=0.

- **Negative branch offsets**: Python has arbitrary-precision integers,
  so there is no natural 16-bit wrapping. Negative relative jumps use
  `add_16bit(pc, 0x10000 + rel, 0)`, which produces the correct 16-bit
  result after masking with `& 0xFFFF`.

- **Decoder ordering**: The decoder function first checks all
  fully-specified 8-bit patterns (single-byte opcodes like 0xE4 CLR A)
  before entering family-range patterns that only constrain bits[7:3].
  This prevents, for example, 0xE4 from accidentally matching the
  MOV A,Rn family (0xE8-0xEF).

- **SUBB two's complement**: Subtraction A − B − borrow uses
  `NOT(B)` + 1 − borrow, implemented as
  `ripple_carry_adder(A, NOT(B), NOT(borrow))` where NOT(borrow)
  converts borrow-in to carry-in.
