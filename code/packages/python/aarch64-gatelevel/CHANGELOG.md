# Changelog — coding-adventures-aarch64-gatelevel

## [0.1.0] — 2026-05-16

### Added

Initial release of the AArch64 (ARMv8-A, 2011) gate-level simulator.

**Package structure:**
- `src/aarch64_gatelevel/bits.py` — LSB-first bit-list bridge between integers and gate
  primitives; full 64-bit arithmetic (add, sub, and, or, xor, not), shifts, rotate,
  CLZ, multiply (shift-and-add), and division (restoring long division) — all via
  `AND`/`OR`/`XOR`/`NOT` gates and `ripple_carry_adder`
- `src/aarch64_gatelevel/alu.py` — 64-bit ALU surface (`ALUResult64`, `add64`, `sub64`,
  `add32`, `sub32`, logical ops, shifts, `clz64`/`clz32`, byte-reversal, multiply,
  divide) with NZCV flag computation
- `src/aarch64_gatelevel/register_file.py` — `RegisterFile` class: 32 × 64-bit GPRs
  as bit lists; XZR (index 31) hardwired zero; separate SP; W-register zero-extension
- `src/aarch64_gatelevel/decoder.py` — Combinational AArch64 instruction decoder:
  pure function, all encoding classes, bitmask-immediate decoder
- `src/aarch64_gatelevel/simulator.py` — `AArch64GateLevelSimulator` implementing
  `Simulator[AArch64State]`; complete instruction set dispatched through gate-level ALU

**Instructions implemented:**
- Data processing immediate: ADD, ADDS, SUB, SUBS, MOVZ, MOVN, MOVK
- Data processing register: ADD, ADDS, SUB, SUBS (shifted register)
- Logical immediate: AND, ORR, EOR, ANDS (bitmask immediate)
- Logical shifted register: AND, ORR, EOR, ANDS, BIC, ORN, EON, BICS
- Branches: B, BL, B.cond (all 14 conditions), CBZ, CBNZ, TBZ, TBNZ, BR, BLR, RET
- Load/Store unsigned offset: STRB, LDRB, LDRSB, STRH, LDRH, LDRSH, STR32, LDR32,
  LDRSW, STR, LDR (all sizes, signed and unsigned)
- 3-source: MADD, MSUB (MUL/MNEG as aliases with Ra=XZR)
- High multiply: SMULH, UMULH
- Division: UDIV, SDIV
- Variable shifts: LSLV, LSRV, ASRV, RORV
- Data processing 1-source: CLZ, RBIT, REV, REV16, REV32
- Conditional select: CSEL, CSINC, CSINV, CSNEG

**Gate-level constraint:**
- All ALU operations on register values route through `AND`/`OR`/`XOR`/`NOT` gates
  and `ripple_carry_adder` — no Python `+`, `-`, `&`, `|`, `^`, `~` on register values
- Registers stored as `list[int]` of bits (LSB-first flip-flop arrays)
- Python arithmetic used only for host bookkeeping (addresses, loop indices, PC)

**Tests (256 tests, 82% coverage):**
- `test_bits.py` — bit helpers, arithmetic, shifts, multiply, divide
- `test_alu.py` — all ALU operations with NZCV flag correctness
- `test_register_file.py` — XZR, W-register zero-extension, SP independence
- `test_decoder.py` — all encoding classes with explicit hand-coded encodings
- `test_programs.py` — multi-instruction programs including loops, branches, memory,
  multiply/divide, Fibonacci, conditional select, CLZ
- `test_equivalence.py` — 10 cross-validation programs against the behavioral
  `aarch64-simulator` asserting identical GPR/SP/NZCV/memory final state

**Bug fixes during development:**
- Fixed EOR register form: the dispatch heuristic incorrectly fell into the
  "immediate" path when Rm=0 (XZR) and shift_amount=0; fixed by using
  `bitmask_imm != 0` as the sole discriminant for logical-immediate vs register
- Removed duplicate `_exec_dp2` method definition (second, cleaner version kept)
- Fixed `test_w_register_zero_extends`: the test used `opc=0b10` (MOVZ) where
  `opc=0b11` (MOVK) was intended; corrected to use `MOVN X0, #0` instead

**Dependencies:**
- `coding-adventures-logic-gates` — AND, OR, XOR, NOT
- `coding-adventures-arithmetic` — ripple_carry_adder
- `coding-adventures-simulator-protocol` — Simulator, StepTrace, ExecutionResult
- `coding-adventures-aarch64-simulator` — AArch64State shared state dataclass
