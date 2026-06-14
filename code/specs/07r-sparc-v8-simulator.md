# Layer 07r — SPARC V8 (1987) Behavioral Simulator

## Overview

The SPARC V8 (Scalable Processor ARChitecture, Version 8) is Sun Microsystems'
RISC architecture, introduced in 1987 and codified in the SPARC V8 architecture
manual (1992).  SPARC powered every Sun workstation from the late 1980s through
the mid-2000s, drove the development of Solaris/SunOS, and is still produced
by Fujitsu (SPARC64) and Oracle (SPARC M-series) today.

SPARC was designed as an *open* architecture: Sun published the full spec and
invited other vendors to build SPARC chips.  This produced a rich ecosystem of
compatible implementations and cemented SPARC's role in high-performance UNIX
computing.

**Historical significance:**
- Sun's answer to MIPS R2000 — another influential early RISC design (1987)
- First architecture to popularise **register windows** (overlapping call frames)
- Foundation of SunOS 4, Solaris 2, and early Java workstation environments
- Used in Sun-3, SPARCstation 1, SparcCenter, UltraSPARC (V9 successor)
- SPARC V8 is still the canonical RISC example in many systems textbooks

---

## Architecture

### Registers

SPARC V8's most distinctive feature is the **register window** mechanism.

#### Global Registers (always visible)

| Name  | Number | ABI Role                              |
|-------|--------|---------------------------------------|
| %g0   | r0     | Hardwired zero (writes discarded)     |
| %g1   | r1     | Caller-saved temporary                |
| %g2–%g3 | r2–r3 | Global (application/OS use)          |
| %g4–%g7 | r4–r7 | Reserved (global/OS use)             |

#### Windowed Registers (r8–r31 in current window)

At any time a program sees 24 *windowed* registers, divided into three groups:

| Name      | r-numbers | Role                                    |
|-----------|-----------|-----------------------------------------|
| %o0–%o7   | r8–r15    | **Out** registers (args to callees)     |
| %l0–%l7   | r16–r23   | **Local** registers (private to window) |
| %i0–%i7   | r24–r31   | **In** registers (args from callers)    |

Special windowed registers by ABI convention:

| Name | Alias | Role                                   |
|------|-------|----------------------------------------|
| %o6  | %sp   | Stack pointer (out register 6)         |
| %o7  | r15   | Caller's return address − 8 (JAL link) |
| %i6  | %fp   | Frame pointer (in register 6)          |
| %i7  | r31   | Return address − 8                     |

#### Register Windows Explained

Physical register file has `NWINDOWS × 16 + 8` registers.  This simulator uses
**NWINDOWS = 3** (three windows) for clarity, giving 3×16 + 8 = 56 physical
registers.

The **CWP** (Current Window Pointer, 0–NWINDOWS−1) selects the active window:

```
Window W sees:
  r0–r7   → global physical regs 0–7         (same in every window)
  r8–r15  → windowed physical[8 + W*16 + 0..7]   (%o0–%o7)
  r16–r23 → windowed physical[8 + W*16 + 8..15]  (%l0–%l7)
  r24–r31 → windowed physical[8 + ((W+1)%NWINDOWS)*16 + 0..7]  (%i0–%i7)
```

The **ins** of window W overlap with the **outs** of window W+1 — this is the
key insight: when the callee reads its `%i0`, it reads the same physical register
that the caller wrote as `%o0`, with no copying.

**SAVE** (procedure entry): `CWP = (CWP − 1) % NWINDOWS`.
The caller's `%o` registers become the callee's `%i` registers.

**RESTORE** (procedure exit): `CWP = (CWP + 1) % NWINDOWS`.
The callee's `%i` registers revert to the caller's `%o` registers.

Window overflow/underflow raises ValueError in this simulator (no trap handler).

#### Special Registers

| Name | Width | Description                                              |
|------|-------|----------------------------------------------------------|
| PC   | 32    | Program counter (current instruction address)            |
| nPC  | 32    | Next PC (for delay slots — see Simplifications)          |
| PSR  | 32    | Processor Status Register (condition codes + CWP + more) |
| Y    | 32    | Multiply/divide auxiliary register                       |
| WIM  | 32    | Window Invalid Mask (one bit per window)                 |

**PSR field layout (relevant bits):**

| Bits  | Field | Description                       |
|-------|-------|-----------------------------------|
| 23    | N     | Negative (result bit 31 was set)  |
| 22    | Z     | Zero (result was zero)            |
| 21    | V     | Overflow (signed overflow)        |
| 20    | C     | Carry (unsigned overflow/borrow)  |
| 4:0   | CWP   | Current Window Pointer (0 to N−1) |

Condition code instructions (`ADDcc`, `SUBcc`, etc.) update N, Z, V, C.

### Memory

SPARC V8 addresses a 32-bit byte-addressed space, big-endian.

This simulator uses **64 KB** flat memory (addresses 0x0000–0xFFFF).
Programs load at address 0x0000.  PC and effective addresses are masked to 16 bits.

Alignment:
- **LD / ST** (word): 4-byte aligned
- **LDSH / LDUH / STH** (halfword): 2-byte aligned
- **LDSB / LDUB / STB** (byte): any address

Misaligned accesses raise `ValueError`.

### Instruction Formats

SPARC V8 uses four fixed-width 32-bit instruction formats:

```
Format 1 (CALL):
  [op:2=01][disp30:30]

Format 2 (SETHI / Bicc / NOP):
  [op:2=00][rd:5][op2:3][imm22:22]

Format 3, register operand:
  [op:2][rd:5][op3:6][rs1:5][i:1=0][asi:8][rs2:5]

Format 3, immediate operand:
  [op:2][rd:5][op3:6][rs1:5][i:1=1][simm13:13]
```

`op` selects the format family:
- `op=00`: Format 2 (branches, SETHI)
- `op=01`: CALL
- `op=10`: ALU / control (Format 3)
- `op=11`: Memory loads/stores (Format 3)

---

## Instruction Set

### Format 2 — SETHI / Bicc

| op2 | Mnemonic | Operation                                      |
|-----|----------|------------------------------------------------|
| 0b100 | SETHI   | rd = imm22 << 10 (set high 22 bits)            |
| 0b100 | NOP     | SETHI 0, %g0 (no-op canonical encoding)        |
| 0b010 | Bicc    | Branch on integer condition code (see below)   |

**Bicc condition codes** (encoded in `rd` field bits 28:25):

| cond | Mnemonic | Condition                  |
|------|----------|----------------------------|
| 1000 | BA       | Branch always              |
| 0000 | BN       | Branch never               |
| 1001 | BNE      | Branch if not equal (Z=0)  |
| 0001 | BE       | Branch if equal (Z=1)      |
| 1010 | BG       | Branch if greater (signed) |
| 0010 | BLE      | Branch if ≤ (signed)       |
| 1011 | BGE      | Branch if ≥ (signed)       |
| 0011 | BL       | Branch if less (signed)    |
| 1100 | BGU      | Branch if > (unsigned)     |
| 0100 | BLEU     | Branch if ≤ (unsigned)     |
| 1101 | BCC      | Branch if carry clear (C=0)|
| 0101 | BCS      | Branch if carry set (C=1)  |
| 1110 | BPOS     | Branch if positive (N=0)   |
| 0110 | BNEG     | Branch if negative (N=1)   |
| 1111 | BVC      | Branch if overflow clear   |
| 0111 | BVS      | Branch if overflow set     |

Branch target: `PC + sign_extend(disp22) * 4`
(disp22 is imm22 field; `a` annul bit is ignored — see Simplifications)

### Format 1 — CALL

| Mnemonic | Operation                                            |
|----------|------------------------------------------------------|
| CALL     | %o7 = PC; PC = PC + sign_extend(disp30) * 4         |

### Format 3 — ALU (op=10)

| op3  | Mnemonic | cc? | Operation                                        |
|------|----------|-----|--------------------------------------------------|
| 0x00 | ADD      |     | rd = rs1 + rs2_or_simm13                         |
| 0x10 | ADDcc    | ✓   | rd = rs1 + rs2; update N,Z,V,C                   |
| 0x08 | ADDX     |     | rd = rs1 + rs2 + C (add with carry)              |
| 0x18 | ADDXcc   | ✓   | rd = rs1 + rs2 + C; update N,Z,V,C              |
| 0x04 | SUB      |     | rd = rs1 − rs2_or_simm13                         |
| 0x14 | SUBcc    | ✓   | rd = rs1 − rs2; update N,Z,V,C                   |
| 0x0C | SUBX     |     | rd = rs1 − rs2 − C (subtract with borrow)        |
| 0x1C | SUBXcc   | ✓   | rd = rs1 − rs2 − C; update N,Z,V,C              |
| 0x01 | AND      |     | rd = rs1 & rs2_or_simm13                         |
| 0x11 | ANDcc    | ✓   | rd = rs1 & rs2; update N,Z,V,C                   |
| 0x05 | ANDN     |     | rd = rs1 & ~rs2_or_simm13                        |
| 0x15 | ANDNcc   | ✓   | rd = rs1 & ~rs2; update N,Z,V,C                  |
| 0x02 | OR       |     | rd = rs1 \| rs2_or_simm13                        |
| 0x12 | ORcc     | ✓   | rd = rs1 \| rs2; update N,Z,V,C                  |
| 0x06 | ORN      |     | rd = rs1 \| ~rs2_or_simm13                       |
| 0x16 | ORNcc    | ✓   | rd = rs1 \| ~rs2; update N,Z,V,C                 |
| 0x03 | XOR      |     | rd = rs1 ^ rs2_or_simm13                         |
| 0x13 | XORcc    | ✓   | rd = rs1 ^ rs2; update N,Z,V,C                   |
| 0x07 | XNOR     |     | rd = ~(rs1 ^ rs2_or_simm13)                      |
| 0x17 | XNORcc   | ✓   | rd = ~(rs1 ^ rs2); update N,Z,V,C                |
| 0x25 | SLL      |     | rd = rs1 << (rs2 & 31)                            |
| 0x26 | SRL      |     | rd = rs1 >> (rs2 & 31) (logical, zero-fill)       |
| 0x27 | SRA      |     | rd = rs1 >>> (rs2 & 31) (arithmetic, sign-fill)   |
| 0x3C | SAVE     |     | rd = rs1 + rs2; CWP = (CWP−1) % NWINDOWS         |
| 0x3D | RESTORE  |     | rd = rs1 + rs2; CWP = (CWP+1) % NWINDOWS         |
| 0x38 | JMPL     |     | rd = PC; PC = rs1 + rs2_or_simm13                 |
| 0x30 | WRY      |     | Y = rs1 ^ rs2_or_simm13                           |
| 0x28 | RDY      |     | rd = Y                                            |
| 0x24 | MULScc   | ✓   | One step of signed multiply (Y:rd shift)          |
| 0x0A | UMUL     |     | Y:rd = unsigned(rs1) × unsigned(rs2)              |
| 0x0B | SMUL     |     | Y:rd = signed(rs1) × signed(rs2)                  |
| 0x5A | UMULcc   | ✓   | Y:rd = unsigned(rs1) × unsigned(rs2); update cc   |
| 0x5B | SMULcc   | ✓   | Y:rd = signed(rs1) × signed(rs2); update cc       |
| 0x0E | UDIV     |     | rd = (Y:rs1) / unsigned(rs2) (64÷32 → 32)         |
| 0x0F | SDIV     |     | rd = (Y:rs1) / signed(rs2)                        |
| 0x5E | UDIVcc   | ✓   | Same as UDIV, update cc                           |
| 0x5F | SDIVcc   | ✓   | Same as SDIV, update cc                           |

**Ticc — Trap on integer condition (op3=0x3A):**
Used for HALT sentinel.  `ta 0` (trap always, software trap 0) = 0x91D02000.

### Format 3 — Memory (op=11)

| op3  | Mnemonic | Operation                                           |
|------|----------|-----------------------------------------------------|
| 0x00 | LD       | rd = mem32[rs1 + rs2_or_simm13]                    |
| 0x04 | ST       | mem32[rs1 + rs2_or_simm13] = rd                    |
| 0x01 | LDUB     | rd = zero_extend(mem8[ea])                          |
| 0x02 | LDUH     | rd = zero_extend(mem16[ea])                         |
| 0x09 | LDSB     | rd = sign_extend(mem8[ea])                          |
| 0x0A | LDSH     | rd = sign_extend(mem16[ea])                         |
| 0x05 | STB      | mem8[ea] = rd[7:0]                                  |
| 0x06 | STH      | mem16[ea] = rd[15:0]                                |

---

## HALT Convention

`ta 0` (Trap Always, software trap 0) halts the simulator:

```python
HALT = bytes([0x91, 0xD0, 0x20, 0x00])   # ta 0 — 0x91D02000 big-endian
```

Encoding: op=2, rd=8 (condition "always"), op3=0x3A (Ticc), rs1=0, i=1, simm13=0.

Any Ticc instruction with condition "always" (cond=8) halts execution and sets
`halted=True`.  Other trap conditions and software trap numbers raise ValueError.

---

## Condition Code Updates

For `cc` instructions (`ADDcc`, `SUBcc`, etc.), the PSR N/Z/V/C bits are updated:

```
result = full 32-bit unsigned result
N = result[31]            # high bit
Z = (result == 0)
V = signed overflow       # (for ADD: signs of both operands equal, sign of result differs)
C = unsigned carry/borrow # (for ADD: carry out of bit 31; for SUB: borrow)
```

---

## Simplifications

1. **No branch delay slots.** Real SPARC CPUs execute the instruction in the
   delay slot before the branch takes effect.  This simulator does not model
   delay slots.  Programs must not rely on delay-slot execution.

2. **No annul bit.** The `a` (annul) bit in branch instructions is ignored.

3. **No window overflow/underflow traps.** SAVE when CWP is already 0 or
   RESTORE when CWP is already NWINDOWS−1 raises ValueError instead of
   triggering a trap to a trap handler.

4. **No coprocessor / FPU.** Only the integer unit is implemented.

5. **64 KB memory.** 32-bit address space modelled as 64 KB flat array.

6. **Simplified PSR.** Only N, Z, V, C condition code bits and CWP[4:0] are
   implemented.  ET (enable traps), S (supervisor), PIL, etc. are ignored.

7. **No ASI.** Alternate Space Identifiers (the 8-bit asi field in Format 3 reg
   instructions) are ignored — all memory accesses use a single flat space.

8. **No MULScc detail.** MULScc performs the full multiply step as a simple
   shift-and-add; the exact PSR.V update matches the spec.

---

## SIM00 Protocol

```python
class SPARCSimulator(Simulator[SPARCState]):
    def reset(self) -> None: ...
    def load(self, program: bytes) -> None: ...
    def step(self) -> StepTrace: ...
    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult: ...
    def get_state(self) -> SPARCState: ...
```

`SPARCState` is a `frozen=True` dataclass:

```python
@dataclass(frozen=True)
class SPARCState:
    pc:      int              # 32-bit program counter
    npc:     int              # 32-bit next-PC
    regs:    tuple[int, ...]  # 56 physical registers (unsigned 32-bit)
    cwp:     int              # Current Window Pointer (0 to NWINDOWS-1)
    psr_n:   bool             # Negative flag
    psr_z:   bool             # Zero flag
    psr_v:   bool             # Overflow flag
    psr_c:   bool             # Carry flag
    y:       int              # Y register (multiply/divide)
    memory:  tuple[int, ...]  # 65536 bytes
    halted:  bool
```

Convenience properties: `.g0`–`.g7`, `.o0`–`.o7`, `.l0`–`.l7`, `.i0`–`.i7`,
`.sp` (%o6), `.fp` (%i6), `.o7` (link register).

---

## Package Layout

```
code/packages/python/sparc-v8-simulator/
├── pyproject.toml
├── README.md
├── CHANGELOG.md
├── src/
│   └── sparc_v8_simulator/
│       ├── __init__.py       (exports SPARCSimulator, SPARCState)
│       ├── py.typed
│       ├── state.py          (SPARCState dataclass + constants)
│       └── simulator.py      (SPARCSimulator — ~750 lines)
└── tests/
    ├── test_protocol.py      (SIM00 compliance)
    ├── test_instructions.py  (per-instruction correctness)
    ├── test_programs.py      (end-to-end programs)
    └── test_coverage.py      (edge cases, windows, overflow, alignment)
```

---

## Design Notes

### Why Register Windows?

Register windows solve the function call overhead problem without compiler
register allocation.  On each SAVE:

1. The hardware "rotates" the window — O(1), no copy
2. Caller's `%o` registers become callee's `%i` registers — argument passing is free
3. Callee has 8 fresh `%l` (local) registers that caller cannot touch
4. On RESTORE, the reverse happens — return value in `%i0` becomes caller's `%o0`

Compare to MIPS where caller must explicitly push/pop `$ra` and callee-saved
registers — SPARC moves this bookkeeping into hardware.

### Condition Codes vs. SLT

MIPS (07q) has no condition codes — comparisons write 0/1 to a GPR (SLT/SLTU).
SPARC has a traditional condition code register (PSR.N/Z/V/C).  Comparing these
two approaches in consecutive layers illustrates a fundamental RISC design debate:
condition codes enable compact branch code but require extra pipeline forwarding;
GPR results are uniform but cost a register and an instruction.

### Big-Endian Memory Layout

SPARC V8 is big-endian (matching MIPS R2000, PDP-11, and Motorola 68000 in this
series).  For a 32-bit value `0x12345678`:
```
addr+0: 0x12  (most significant byte)
addr+1: 0x34
addr+2: 0x56
addr+3: 0x78  (least significant byte)
```

### SETHI + OR Idiom

To load a full 32-bit constant (e.g., 0xDEADBEEF) into a register:
```
sethi %hi(0xDEADBEEF), %o0   ! %o0 = 0xDEAD0000  (top 22 bits << 10)
or    %o0, %lo(0xDEADBEEF), %o0  ! %o0 = 0xDEADBEEF
```
`%hi(x)` = x >> 10 (22 bits); `%lo(x)` = x & 0x3FF (10 bits fit in simm13).

---

## Test Plan

| Test module          | What it covers                                              |
|----------------------|-------------------------------------------------------------|
| test_protocol.py     | isinstance, all 5 methods, return types                     |
| test_protocol.py     | reset() → PC=0, all regs=0, cc=0, CWP=0, memory zeroed     |
| test_protocol.py     | load() → places bytes at 0x0000, raises on overflow         |
| test_protocol.py     | execute() → ExecutionResult with correct fields             |
| test_protocol.py     | step() → StepTrace with pc_before/pc_after                  |
| test_protocol.py     | get_state() → frozen snapshot, not mutated by step          |
| test_instructions.py | SETHI + OR idiom loads full 32-bit constant                 |
| test_instructions.py | ADD, ADDcc (condition codes N/Z/V/C)                        |
| test_instructions.py | SUB, SUBcc, SUBX (carry-in)                                 |
| test_instructions.py | AND, OR, XOR, ANDN, ORN, XNOR                               |
| test_instructions.py | SLL, SRL, SRA (shift by register and immediate)             |
| test_instructions.py | UMUL, SMUL (Y:rd result)                                    |
| test_instructions.py | UDIV, SDIV (64÷32 divide)                                   |
| test_instructions.py | RDY, WRY (Y register access)                                |
| test_instructions.py | LD, ST, LDSB, LDUB, LDSH, LDUH, STB, STH                   |
| test_instructions.py | JMPL (jump and link, %o7 set)                               |
| test_instructions.py | CALL (disp30 jump, %o7 = PC)                                |
| test_instructions.py | Bicc: BA, BN, BE, BNE, BG, BL, BGE, BLE, BGU, BCS, etc.   |
| test_instructions.py | SAVE / RESTORE register window rotation                     |
| test_coverage.py     | %g0 always zero (writes discarded)                          |
| test_coverage.py     | Misaligned LD/ST raises ValueError                          |
| test_coverage.py     | Window overflow/underflow raises ValueError                 |
| test_coverage.py     | Unknown op/op3 raises ValueError                            |
| test_coverage.py     | max_steps guard terminates infinite loop                    |
| test_coverage.py     | ADDcc N/Z/V/C set correctly for boundary values             |
| test_coverage.py     | SUBcc V/C for borrow and signed overflow                    |
| test_coverage.py     | SMUL negative × negative = positive (Y:rd)                  |
| test_programs.py     | Sum 1 to 10 using ADDcc + BNE loop                         |
| test_programs.py     | Factorial 5! using SMUL                                     |
| test_programs.py     | Fibonacci (8 terms) in memory                               |
| test_programs.py     | Subroutine call/return with CALL / JMPL / SAVE / RESTORE    |
| test_programs.py     | Bubble sort using SUBcc + BLE                               |
| test_programs.py     | Byte copy loop using LDSB/STB                               |
