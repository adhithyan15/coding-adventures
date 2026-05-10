# Layer 07q — MIPS R2000 (1985) Behavioral Simulator

## Overview

The MIPS R2000 (1985) is the first commercially successful RISC processor and
the machine that proved the "Reduced Instruction Set Computer" philosophy viable
at scale.  Designed by John Hennessy and his Stanford team (later co-founders of
MIPS Computer Systems), it shipped in SGI IRIS workstations and DEC DECstations,
and its architecture spawned the MIPS family used in PlayStation 1, PlayStation 2,
Nintendo 64, and countless embedded systems.

MIPS stands for **Microprocessor without Interlocked Pipeline Stages**.  The name
captures its design philosophy: a five-stage pipeline (IF→ID→EX→MEM→WB) with no
hardware interlocks — the programmer (or compiler) is responsible for scheduling
instructions to avoid data hazards, inserting NOP instructions when needed.

**Historical significance:**
- First commercial RISC processor (1985)
- Foundation of Patterson & Hennessy *Computer Organization and Design* textbook
- 32-bit word-addressable big-endian architecture
- Used in PlayStation 1, PlayStation 2, Nintendo 64, Cisco routers, many more
- Directly influenced later RISC designs including SPARC, PA-RISC, and Alpha

---

## Architecture

### Registers

| Name      | Number | ABI Role                        |
|-----------|--------|---------------------------------|
| $zero     | R0     | Hardwired zero (writes ignored) |
| $at       | R1     | Assembler temporary             |
| $v0–$v1   | R2–R3  | Return values                   |
| $a0–$a3   | R4–R7  | Function arguments              |
| $t0–$t7   | R8–R15 | Temporaries (caller-saved)      |
| $s0–$s7   | R16–R23| Saved registers (callee-saved)  |
| $t8–$t9   | R24–R25| More temporaries                |
| $k0–$k1   | R26–R27| Kernel reserved                 |
| $gp       | R28    | Global pointer                  |
| $sp       | R29    | Stack pointer                   |
| $fp ($s8) | R30    | Frame pointer                   |
| $ra       | R31    | Return address (set by JAL)     |

In addition to the 32 GPRs:

| Name | Description                                     |
|------|-------------------------------------------------|
| PC   | 32-bit program counter                          |
| HI   | High 32 bits of MULT/MULTU result; DIV remainder|
| LO   | Low 32 bits of MULT/MULTU result; DIV quotient  |

**R0 is always zero.** Any write to R0 is silently discarded.

### Memory

The MIPS R2000 addresses a 32-bit (4 GB) byte-addressed memory space.  The CPU
is big-endian: multi-byte values are stored most-significant-byte first.

For this simulator, we expose a flat **64 KB** memory array (addresses 0x0000–0xFFFF)
for simplicity.  Programs are loaded at address 0x0000.  The PC wraps modulo 64 KB.

Alignment rules:
- **LW/SW**: address must be 4-byte aligned (addr & 3 == 0)
- **LH/LHU/SH**: address must be 2-byte aligned (addr & 1 == 0)
- **LB/LBU/SB**: no alignment requirement

Misaligned accesses raise `ValueError`.

### Instruction Formats

All instructions are exactly 32 bits wide, enabling single-cycle fetch.

```
R-type:  [op:6][rs:5][rt:5][rd:5][shamt:5][funct:6]
I-type:  [op:6][rs:5][rt:5][imm16:16]
J-type:  [op:6][target26:26]
```

- **R-type** (op=0): ALU operations.  The `funct` field selects the operation.
- **I-type**: loads, stores, branches, immediate ALU ops.  `imm16` is sign-extended
  to 32 bits for arithmetic/memory instructions; zero-extended for ANDI/ORI/XORI.
- **J-type**: unconditional jumps.  Target = `(PC+4)[31:28] | (target26 << 2)`.

### Branch Target Calculation

For branch instructions (BEQ, BNE, BLEZ, BGTZ, BLTZ, BGEZ, BLTZAL, BGEZAL):

```
target = (PC + 4) + (sign_extend16(imm16) << 2)
```

The `PC + 4` reflects the address of the *next* instruction (the delay slot on real
hardware; in this simulator there are **no delay slots** — see Simplifications below).

### Jump Target Calculation (J / JAL)

```
target = (PC + 4)[31:28] | (target26 << 2)
```

### HI:LO Registers

MULT / MULTU compute a 64-bit product: HI holds bits 63:32, LO holds bits 31:0.
DIV / DIVU compute quotient in LO and remainder in HI.

---

## Instruction Set

### R-type (op = 0, decoded by funct)

| Funct | Mnemonic | Operation                                   |
|-------|----------|---------------------------------------------|
| 0x00  | SLL      | rd = rt << shamt (logical)                  |
| 0x02  | SRL      | rd = rt >> shamt (logical)                  |
| 0x03  | SRA      | rd = rt >>> shamt (arithmetic)              |
| 0x04  | SLLV     | rd = rt << (rs & 31)                        |
| 0x06  | SRLV     | rd = rt >> (rs & 31) (logical)              |
| 0x07  | SRAV     | rd = rt >>> (rs & 31) (arithmetic)          |
| 0x08  | JR       | PC = rs                                     |
| 0x09  | JALR     | rd = PC+4; PC = rs                          |
| 0x0C  | SYSCALL  | **HALT sentinel** (see below)               |
| 0x0D  | BREAK    | Raise ValueError (software breakpoint)      |
| 0x10  | MFHI     | rd = HI                                     |
| 0x11  | MTHI     | HI = rs                                     |
| 0x12  | MFLO     | rd = LO                                     |
| 0x13  | MTLO     | LO = rs                                     |
| 0x18  | MULT     | HI:LO = signed(rs) × signed(rt)            |
| 0x19  | MULTU    | HI:LO = unsigned(rs) × unsigned(rt)        |
| 0x1A  | DIV      | LO = signed(rs)/signed(rt); HI = remainder |
| 0x1B  | DIVU     | LO = unsigned(rs)/unsigned(rt); HI = rem   |
| 0x20  | ADD      | rd = rs + rt (signed overflow → ValueError)|
| 0x21  | ADDU     | rd = rs + rt (wraps, no overflow check)     |
| 0x22  | SUB      | rd = rs − rt (signed overflow → ValueError)|
| 0x23  | SUBU     | rd = rs − rt (wraps, no overflow check)     |
| 0x24  | AND      | rd = rs & rt                                |
| 0x25  | OR       | rd = rs \| rt                               |
| 0x26  | XOR      | rd = rs ^ rt                                |
| 0x27  | NOR      | rd = ~(rs \| rt)                            |
| 0x2A  | SLT      | rd = (signed(rs) < signed(rt)) ? 1 : 0     |
| 0x2B  | SLTU     | rd = (unsigned(rs) < unsigned(rt)) ? 1 : 0 |

### REGIMM (op = 1, decoded by rt field)

| rt   | Mnemonic | Operation                                                  |
|------|----------|------------------------------------------------------------|
| 0x00 | BLTZ     | if (signed(rs) < 0): PC = target                          |
| 0x01 | BGEZ     | if (signed(rs) >= 0): PC = target                         |
| 0x10 | BLTZAL   | $ra = PC+4; if (signed(rs) < 0): PC = target              |
| 0x11 | BGEZAL   | $ra = PC+4; if (signed(rs) >= 0): PC = target             |

### I-type and J-type (op ≥ 2)

| op   | Mnemonic | Operation                                                   |
|------|----------|-------------------------------------------------------------|
| 0x02 | J        | PC = (PC+4)[31:28] \| (target26 << 2)                     |
| 0x03 | JAL      | $ra = PC+4; PC = (PC+4)[31:28] \| (target26 << 2)         |
| 0x04 | BEQ      | if (rs == rt): PC = target                                  |
| 0x05 | BNE      | if (rs != rt): PC = target                                  |
| 0x06 | BLEZ     | if (signed(rs) <= 0): PC = target                          |
| 0x07 | BGTZ     | if (signed(rs) > 0): PC = target                           |
| 0x08 | ADDI     | rt = rs + sext(imm16) (signed overflow → ValueError)       |
| 0x09 | ADDIU    | rt = rs + sext(imm16) (wraps, no overflow check)           |
| 0x0A | SLTI     | rt = (signed(rs) < signed(sext(imm16))) ? 1 : 0           |
| 0x0B | SLTIU    | rt = (unsigned(rs) < unsigned(sext(imm16))) ? 1 : 0       |
| 0x0C | ANDI     | rt = rs & zero_extend(imm16)                               |
| 0x0D | ORI      | rt = rs \| zero_extend(imm16)                              |
| 0x0E | XORI     | rt = rs ^ zero_extend(imm16)                               |
| 0x0F | LUI      | rt = imm16 << 16 (loads upper 16 bits)                     |
| 0x20 | LB       | rt = sign_extend(mem8[rs + sext(imm16)])                   |
| 0x21 | LH       | rt = sign_extend(mem16[rs + sext(imm16)])                  |
| 0x23 | LW       | rt = mem32[rs + sext(imm16)]                               |
| 0x24 | LBU      | rt = zero_extend(mem8[rs + sext(imm16)])                   |
| 0x25 | LHU      | rt = zero_extend(mem16[rs + sext(imm16)])                  |
| 0x28 | SB       | mem8[rs + sext(imm16)] = rt[7:0]                           |
| 0x29 | SH       | mem16[rs + sext(imm16)] = rt[15:0]                         |
| 0x2B | SW       | mem32[rs + sext(imm16)] = rt                               |

---

## HALT Convention

Opcode 0, funct 0x0C is `SYSCALL`.  On real MIPS, SYSCALL triggers a trap to
the OS kernel.  In this simulator, **any SYSCALL instruction halts execution**
and sets `halted=True`.  The mnemonic for StepTrace is `"HALT"`.

```python
HALT = bytes([0x00, 0x00, 0x00, 0x0C])  # SYSCALL (big-endian)
```

`BREAK` (funct=0x0D) raises `ValueError` (software breakpoint, signals a bug).

---

## Simplifications

This is a **behavioral** simulator, not a cycle-accurate one:

1. **No branch delay slots.** On real MIPS, the instruction immediately after a
   branch (the "delay slot") executes unconditionally before the branch takes
   effect.  This simulator does not model delay slots: branches take effect
   immediately after the branch instruction, and the next instruction fetched
   is at the branch target.  Programs must not rely on delay slot semantics.

2. **No pipeline hazards.** Loads followed immediately by dependent instructions
   work without inserting NOPs.

3. **64 KB memory.** The 32-bit address space is modelled as a 64 KB flat array.
   PC and addresses are masked to 16 bits.

4. **No coprocessor 0 (CP0).** Exception vectors, TLB, and status registers are
   not implemented.  Invalid operations raise Python `ValueError` rather than
   trapping to an exception vector.

5. **No floating point (COP1).** The FPU coprocessor is not implemented.

6. **Signed overflow on ADD/ADDI/SUB raises ValueError.** Real MIPS raises a
   hardware exception; here we raise `ValueError` with a descriptive message.
   Use ADDU/ADDIU/SUBU for wrapping arithmetic.

---

## SIM00 Protocol

```python
class MIPSSimulator(Simulator[MIPSState]):
    def reset(self) -> None: ...
    def load(self, program: bytes) -> None: ...
    def step(self) -> StepTrace: ...
    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult: ...
    def get_state(self) -> MIPSState: ...
```

`MIPSState` is a `frozen=True` dataclass:

```python
@dataclass(frozen=True)
class MIPSState:
    pc:     int              # 32-bit program counter
    regs:   tuple[int, ...]  # 32 unsigned 32-bit general-purpose registers
    hi:     int              # HI register (unsigned 32-bit)
    lo:     int              # LO register (unsigned 32-bit)
    memory: tuple[int, ...]  # 65536 bytes
    halted: bool
```

Convenience properties: `.sp` (R29), `.ra` (R31), `.v0` (R2), `.a0` (R4).

---

## Package Layout

```
code/packages/python/mips-r2000-simulator/
├── pyproject.toml
├── README.md
├── CHANGELOG.md
├── src/
│   └── mips_r2000_simulator/
│       ├── __init__.py       (exports MIPSSimulator, MIPSState)
│       ├── py.typed
│       ├── state.py          (MIPSState dataclass + constants)
│       └── simulator.py      (MIPSSimulator — ~800 lines)
└── tests/
    ├── test_protocol.py      (SIM00 compliance)
    ├── test_instructions.py  (per-instruction correctness)
    ├── test_programs.py      (end-to-end programs)
    └── test_coverage.py      (edge cases, overflow, alignment errors)
```

---

## Design Notes

### Why SYSCALL as HALT?

SYSCALL is the natural OS-entry instruction on MIPS.  Using it as the HALT
sentinel matches real MIPS Linux convention ($v0=4001 = SYS_exit), and keeps
the opcode table semantically clean — there are no arbitrary "undefined" opcodes
on MIPS (unlike x86's 0xF4 HALT or 8051's 0xA5).

### Big-Endian Memory Layout

MIPS R2000 is big-endian by default.  For a 32-bit value `0x12345678`:
```
addr+0: 0x12  (most significant)
addr+1: 0x34
addr+2: 0x56
addr+3: 0x78  (least significant)
```

Programs encoded as `bytes([op_hi, op_mid_hi, op_mid_lo, op_lo])` are correct
for big-endian MIPS.  For convenience, the test helper `w32(v)` packs a 32-bit
instruction as 4 big-endian bytes.

### R0 is Always Zero

Writing to R0 is silently discarded.  This is enforced in `_set_reg()`:
```python
def _set_reg(self, rd: int, val: int) -> None:
    if rd != 0:
        self._regs[rd] = val & 0xFFFFFFFF
```

### SLT / SLTU Distinction

`SLT` interprets both operands as signed 32-bit integers.
`SLTU` interprets both as unsigned.  In particular, `SLTU $t0, $zero, $t1`
tests whether $t1 is non-zero (since $zero=0 and unsigned(anything) ≥ 0).

### Overflow on Signed Arithmetic

ADD, ADDI, SUB raise `ValueError` on signed overflow (matching hardware trap
behavior, useful for catching bugs).  ADDU, ADDIU, SUBU silently wrap.  This
distinction is important: `ADDIU $sp, $sp, -8` for stack allocation should use
ADDIU (or ADDU with a negated offset in register).

---

## Test Plan

| Test module         | What it covers                                           |
|---------------------|----------------------------------------------------------|
| test_protocol.py    | isinstance, all 5 methods callable, return types         |
| test_protocol.py    | reset() → PC=0, all regs=0, HI=LO=0, memory zeroed      |
| test_protocol.py    | load() → places bytes, raises on overflow, resets first  |
| test_protocol.py    | execute() → returns ExecutionResult with correct fields  |
| test_protocol.py    | step() → returns StepTrace with pc_before/after          |
| test_protocol.py    | get_state() → frozen, snapshot not mutated by step       |
| test_instructions.py| All R-type: SLL, SRL, SRA, SLLV, SRLV, SRAV            |
| test_instructions.py| JR, JALR (sets $ra)                                     |
| test_instructions.py| MFHI, MFLO, MTHI, MTLO                                  |
| test_instructions.py| MULT, MULTU (64-bit result in HI:LO)                    |
| test_instructions.py| DIV, DIVU (quotient in LO, remainder in HI)             |
| test_instructions.py| ADD (overflow raises), ADDU (wraps), SUB, SUBU          |
| test_instructions.py| AND, OR, XOR, NOR                                       |
| test_instructions.py| SLT, SLTU (signed vs unsigned comparison)               |
| test_instructions.py| BLTZ, BGEZ, BLTZAL, BGEZAL (taken and not-taken)       |
| test_instructions.py| J, JAL (target calculation)                             |
| test_instructions.py| BEQ, BNE, BLEZ, BGTZ (all 4 conditions)                |
| test_instructions.py| ADDI (overflow), ADDIU, SLTI, SLTIU, ANDI, ORI, XORI  |
| test_instructions.py| LUI (upper 16 bits)                                     |
| test_instructions.py| LB, LH, LW, LBU, LHU (sign/zero extension)             |
| test_instructions.py| SB, SH, SW (byte-order in memory)                       |
| test_programs.py    | Sum 1 to 10 using ADDU + BNE loop                       |
| test_programs.py    | Factorial 5! using MULT                                  |
| test_programs.py    | Fibonacci (8 terms) in memory                           |
| test_programs.py    | Subroutine call/return using JAL/JR                      |
| test_programs.py    | Byte copy loop via LB/SB                                |
| test_programs.py    | Word copy loop via LW/SW                                |
| test_coverage.py    | R0 always zero (write to R0 is no-op)                   |
| test_coverage.py    | Misaligned LW/SW raises ValueError                      |
| test_coverage.py    | Misaligned LH/SH raises ValueError                      |
| test_coverage.py    | ADD signed overflow raises ValueError                    |
| test_coverage.py    | DIV by zero raises ValueError                           |
| test_coverage.py    | BREAK raises ValueError                                  |
| test_coverage.py    | SLTU: 0xFFFFFFFF > 0x00000001 unsigned                  |
| test_coverage.py    | SLT: -1 < 1 signed                                      |
| test_coverage.py    | LB sign extension: 0xFF → -1 (0xFFFFFFFF)               |
| test_coverage.py    | LBU zero extension: 0xFF → 255 (0x000000FF)             |
| test_coverage.py    | NOR: ~(rs\|rt)                                          |
| test_coverage.py    | SRA: arithmetic right shift preserves sign               |
| test_coverage.py    | MULTU: 0xFFFFFFFF × 0xFFFFFFFF → correct HI:LO          |
| test_coverage.py    | max_steps exceeded returns ok=False                      |
