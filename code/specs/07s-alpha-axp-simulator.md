# Layer 07s — DEC Alpha AXP 21064 (1992) Behavioral Simulator

## Overview

The DEC Alpha AXP 21064 (1992) was the first commercially successful **64-bit RISC**
processor and one of the fastest microprocessors ever designed.  Created by Richard
Sites and his team at Digital Equipment Corporation (DEC), it powered DEC's
workstations and servers running OpenVMS, Digital UNIX (later Tru64 UNIX), and
Windows NT on Alpha.

The Alpha stands in deliberate contrast to the other RISC processors in this series:

| Feature | MIPS R2000 (07q) | SPARC V8 (07r) | Alpha AXP (07s) |
|---------|-----------------|----------------|-----------------|
| Year | 1985 | 1987 | 1992 |
| Width | 32-bit | 32-bit | 64-bit throughout |
| GPRs | 32 (r0 = zero) | 56 physical, 8 global | 32 (r31 = zero) |
| Condition codes | None (SLT → GPR) | PSR N/Z/V/C | None (CMP → GPR) |
| Endianness | Big | Big | **Little** |
| Register windows | No | Yes | No |
| Delay slots | Yes | No (our sim) | **No** (by design) |
| Multiply result | Hi/Lo regs | Y register | In GPR (64-bit) |

Alpha was designed from the beginning for **multi-gigahertz clocks**.  Its
architects explicitly chose to forbid features that constrain clock speed:
no delay slots, no condition codes, no microcoded corner cases, no complex
addressing modes, no implicit register aliases.

**Historical significance:**
- First commercial 64-bit processor (1992); ran at 200 MHz when Pentium did 60 MHz
- Inspired the design of the DEC StrongARM, which itself led to modern ARM cores
- First architecture supported by Linux on a non-x86 platform (kernel 1.2)
- Used in Cray T3D/T3E supercomputers (thousands of Alpha nodes)
- Alive as Alpha architecture until HP shut it down in 2004

---

## Architecture

### Registers

| Name | Number | ABI Role |
|------|--------|----------|
| r0 | 0 | Return value / caller-saved |
| r1–r8 | 1–8 | Caller-saved (temporaries) |
| r9–r14 | 9–14 | Callee-saved |
| r15 | 15 | Frame pointer (callee-saved) |
| r16–r21 | 16–21 | Argument registers (caller-saved) |
| r22–r25 | 22–25 | Caller-saved temporaries |
| r26 | 26 | Return address (written by BSR/JSR) |
| r27 | 27 | Procedure value (indirect jump target) |
| r28 | 28 | Scratch / assembler temporary |
| r29 | 29 | Global pointer (gp) |
| r30 | 30 | Stack pointer (sp) |
| r31 | 31 | **Hardwired zero** (reads 0, writes discarded) |

All registers are 64 bits wide.  There is no separate condition-code register —
comparisons (`CMPEQ`, `CMPLT`, etc.) write 0 or 1 into a destination GPR.

### Program Counter

The PC is 64 bits wide (practically 16-bit for the 64 KiB address space in this
simulator).  `nPC` holds the next instruction address (PC+4 after each fetch).

### No Floating-Point (This Simulator)

Alpha has 32 floating-point registers (f0–f31) and a rich FP instruction set.
This simulator omits all FP instructions to stay focused on integer architecture.

---

## Instruction Formats

All instructions are **32 bits** fixed-length.

### Memory Format
```
 31      26 25    21 20    16 15              0
[ op : 6 ][ Ra : 5 ][ Rb : 5 ][ disp16 : 16 ]
```
- `Ra` = destination register (loads) or source register (stores)
- `Rb` = base address register; effective address = Rb + sign_extend(disp16)

### Branch Format
```
 31      26 25    21 20                    0
[ op : 6 ][ Ra : 5 ][     disp21 : 21     ]
```
- Target = (PC_of_branch + 4) + sign_extend(disp21) × 4
- Note: Alpha uses PC+4, not PC, as the branch base (no delay slot needed)

### Operate Format
```
 31      26 25    21 20    16 15 14      12 11       5 4      0
[ op : 6 ][ Ra : 5 ][ Rb : 5 ][ 0 ][ 0 ][ func : 7 ][ Rc : 5 ]
                                ↑
[ op : 6 ][ Ra : 5 ][ lit8: 8 ][ 1 ][ func : 7 ][ Rc : 5 ]
```
- When bit 12 = 0: operand is register Rb
- When bit 12 = 1: operand is 8-bit **zero-extended** literal (unsigned 0–255)
- Result written to Rc

### Jump Format
```
 31      26 25    21 20    16 15    14 13          0
[ 0x1A:6 ][ Ra : 5 ][ Rb : 5 ][ func:2 ][ hint:14 ]
```
- `func`: 00=JMP, 01=JSR, 10=RET, 11=JSR_COROUTINE
- Ra = written with PC+4 (the "link" register); discarded when Ra=r31
- Rb = target register; jumps to Rb & ~3

### PALcode Format
```
 31      26 25                          0
[ 0x00:6 ][ palcode : 26 ]
```
- `call_pal 0x0000` (= word 0x00000000) is the HALT instruction

---

## Memory Model

- **64 KiB flat address space** as a `bytearray`
- **Little-endian** byte order — unique in this simulator series; the prior
  MIPS, SPARC, Z80, and PDP-11 simulators are all big-endian
- Addresses wrap modulo 65536 (no segfaults)
- Alignment enforced: quadword (8-byte), longword (4-byte), word (2-byte) ops
  raise `ValueError` on misaligned effective addresses

Little-endian layout example: `STQ r1=0x01_02_03_04_05_06_07_08` at address 0x100:
```
addr  0x100: 0x08  ← least-significant byte
addr  0x101: 0x07
addr  0x102: 0x06
addr  0x103: 0x05
addr  0x104: 0x04
addr  0x105: 0x03
addr  0x106: 0x02
addr  0x107: 0x01  ← most-significant byte
```

---

## Instruction Set (Subset Implemented)

### Memory Instructions

| Opcode | Mnemonic | Operation |
|--------|----------|-----------|
| 0x28 | LDL | Ra = sign_extend32(mem32[ea]) |
| 0x29 | LDQ | Ra = mem64[ea] |
| 0x2A | LDL_L | same as LDL (no lock semantics) |
| 0x2B | LDQ_L | same as LDQ (no lock semantics) |
| 0x0A | LDBU | Ra = zero_extend8(mem8[ea]) |
| 0x0C | LDWU | Ra = zero_extend16(mem16[ea]) |
| 0x2C | STL | mem32[ea] = Ra[31:0] |
| 0x2D | STQ | mem64[ea] = Ra |
| 0x0E | STB | mem8[ea] = Ra[7:0] |
| 0x0D | STW | mem16[ea] = Ra[15:0] |

### Integer Arithmetic (opcode 0x10 = INTA)

"Longword" (L) variants operate on 32-bit halves and **sign-extend** the result
to 64 bits.  "Quadword" (Q) variants are full 64-bit.

| func | Mnemonic | Operation (Rc = …) |
|------|----------|--------------------|
| 0x00 | ADDL | sext32((Ra[31:0] + src[31:0]) & 0xFFFFFFFF) |
| 0x20 | ADDQ | (Ra + src) & MASK64 |
| 0x09 | SUBL | sext32((Ra[31:0] - src[31:0]) & 0xFFFFFFFF) |
| 0x29 | SUBQ | (Ra - src) & MASK64 |
| 0x18 | MULL | sext32((Ra[31:0] * src[31:0]) & 0xFFFFFFFF) |
| 0x38 | MULQ | (Ra * src) & MASK64 |
| 0x2D | CMPEQ | 1 if Ra == src else 0 |
| 0x4D | CMPLT | 1 if signed(Ra) < signed(src) else 0 |
| 0x6D | CMPLE | 1 if signed(Ra) <= signed(src) else 0 |
| 0x3D | CMPULT | 1 if Ra < src (unsigned) else 0 |
| 0x7D | CMPULE | 1 if Ra <= src (unsigned) else 0 |
| 0x40 | ADDLV | same as ADDL (overflow-trapping variant) |
| 0x60 | ADDQV | same as ADDQ |
| 0x49 | SUBLV | same as SUBL |
| 0x69 | SUBQV | same as SUBQ |
| 0x58 | MULLV | same as MULL |
| 0x78 | MULQV | same as MULQ |

### Integer Logic (opcode 0x11 = INTL)

| func | Mnemonic | Operation (Rc = …) |
|------|----------|--------------------|
| 0x00 | AND | Ra & src |
| 0x08 | BIC | Ra & ~src |
| 0x20 | BIS | Ra \| src (this is OR; mnemonic = Bit Set) |
| 0x28 | ORNOT | Ra \| ~src |
| 0x40 | XOR | Ra ^ src |
| 0x48 | EQV | Ra ^ ~src (XNOR) |
| 0x14 | CMOVLBS | src if (Ra & 1) else Rc (conditional move if low bit set) |
| 0x16 | CMOVLBC | src if !(Ra & 1) else Rc |
| 0x24 | CMOVEQ | src if Ra==0 else Rc |
| 0x26 | CMOVNE | src if Ra!=0 else Rc |
| 0x44 | CMOVLT | src if signed(Ra)<0 else Rc |
| 0x46 | CMOVGE | src if signed(Ra)>=0 else Rc |
| 0x64 | CMOVLE | src if signed(Ra)<=0 else Rc |
| 0x66 | CMOVGT | src if signed(Ra)>0 else Rc |

`BIS r31, lit, Rd` is the standard **load immediate** idiom (Ra=r31=0, BIS 0|lit = lit).

### Integer Shift and Byte Manipulation (opcode 0x12 = INTS)

Shift amount or byte-offset = src & 63 (low 6 bits).
Byte-position functions use `byte_offset = src & 7` (low 3 bits).

| func | Mnemonic | Operation |
|------|----------|-----------|
| 0x39 | SLL | Ra << (src & 63) |
| 0x34 | SRL | Ra >> (src & 63), zero-fill (logical) |
| 0x3C | SRA | arithmetic right shift |
| 0x06 | EXTBL | zero-extend byte at byte_offset in Ra |
| 0x16 | EXTWL | zero-extend 2 bytes at byte_offset |
| 0x26 | EXTLL | zero-extend 4 bytes at byte_offset |
| 0x36 | EXTQL | 8 bytes at byte_offset (= right-shift by byte_offset*8) |
| 0x0B | INSBL | insert byte Ra[7:0] at byte_offset |
| 0x1B | INSWL | insert 2 bytes at byte_offset |
| 0x2B | INSLL | insert 4 bytes at byte_offset |
| 0x3B | INSQL | insert 8 bytes at byte_offset |
| 0x02 | MSKBL | zero byte at byte_offset |
| 0x12 | MSKWL | zero 2 bytes at byte_offset |
| 0x22 | MSKLL | zero 4 bytes at byte_offset |
| 0x32 | MSKQL | zero 8 bytes at byte_offset |
| 0x30 | ZAP | zero bytes of Ra where mask bit SET |
| 0x31 | ZAPNOT | zero bytes of Ra where mask bit NOT set (keep byte where set) |
| 0x00 | SEXTB | sign-extend byte: Rc = sext8(Ra & 0xFF) |
| 0x01 | SEXTW | sign-extend word: Rc = sext16(Ra & 0xFFFF) |

### Integer Multiply (opcode 0x13 = INTM)

| func | Mnemonic | Operation |
|------|----------|-----------|
| 0x00 | MULL | sext32((Ra[31:0] * src[31:0]) & 0xFFFF_FFFF) |
| 0x20 | MULQ | (Ra * src) & MASK64 (lower 64 bits) |
| 0x30 | UMULH | upper 64 bits of unsigned Ra * src |
| 0x40 | MULLV | same as MULL |
| 0x60 | MULQV | same as MULQ |

### Branch Instructions

Target = (PC_of_branch + 4) + sign_extend(disp21) × 4.

| opcode | Mnemonic | Condition |
|--------|----------|-----------|
| 0x39 | BEQ | Ra == 0 |
| 0x3D | BNE | Ra != 0 |
| 0x3A | BLT | signed Ra < 0 |
| 0x3B | BLE | signed Ra ≤ 0 |
| 0x3F | BGT | signed Ra > 0 |
| 0x3E | BGE | signed Ra ≥ 0 |
| 0x38 | BLBC | (Ra & 1) == 0 |
| 0x3C | BLBS | (Ra & 1) == 1 |
| 0x30 | BR | always (Ra written with r31 = discarded) |
| 0x34 | BSR | always (Ra = PC_of_branch + 4, then branch) |

### Jump Instructions (opcode 0x1A)

| func | Mnemonic | Operation |
|------|----------|-----------|
| 0x00 | JMP | Ra = PC+4 (discard if Ra=r31); PC = Rb & ~3 |
| 0x01 | JSR | Ra = PC+4; PC = Rb & ~3 |
| 0x02 | RET | ra field unused (usually r31); PC = Rb & ~3 |
| 0x03 | JSR_COROUTINE | same as JMP |

Typical call/return pattern:
```
BSR  r26, target    ; call: r26 = return address
...subroutine...
RET  r31, (r26)     ; return: jump to r26
```

---

## HALT Convention

`call_pal 0x0000` halts execution.  This encodes as the **all-zeros 32-bit word**:

```python
HALT = bytes([0x00, 0x00, 0x00, 0x00])   # little-endian: word 0x00000000
```

Caution: because `call_pal` uses opcode 0x00, the instruction word 0x00000000 is
the HALT.  An uninitialized memory region (all zeros) will therefore halt
immediately when jumped into, which is convenient for the simulator.

Any `call_pal` with palcode ≠ 0 raises `ValueError` in this simulator.

---

## SIM00 Protocol

```python
class AlphaSimulator(Simulator[AlphaState]):
    def reset(self) -> None: ...          # zeros all state; PC=0, nPC=4
    def load(self, program: bytes) -> None: ...  # reset + copy to 0x0000 (max 64 KiB)
    def step(self) -> StepTrace: ...      # execute one instruction
    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult: ...
    def get_state(self) -> AlphaState: ... # frozen snapshot
```

`AlphaState` is a `frozen=True` dataclass:

```python
@dataclass(frozen=True)
class AlphaState:
    pc:     int               # program counter
    npc:    int               # next-PC
    regs:   tuple[int, ...]   # 32 registers (64-bit unsigned); regs[31] always 0
    memory: tuple[int, ...]   # 65536 bytes
    halted: bool
```

Convenience properties on `AlphaState`: `.r0`–`.r31`, `.sp` (= r30), `.ra`
(= r26), `.gp` (= r29), `.zero` (= r31, always 0).

---

## Package Layout

```
code/packages/python/alpha-axp-simulator/
├── pyproject.toml
├── README.md
├── CHANGELOG.md
├── src/
│   └── alpha_axp_simulator/
│       ├── __init__.py         (exports AlphaSimulator, AlphaState)
│       ├── py.typed
│       ├── state.py            (AlphaState dataclass + constants)
│       └── simulator.py        (AlphaSimulator — ~700 lines)
└── tests/
    ├── test_protocol.py        (SIM00 compliance)
    ├── test_instructions.py    (per-instruction correctness)
    ├── test_programs.py        (end-to-end programs)
    └── test_coverage.py        (edge cases, r31, little-endian, etc.)
```

---

## Design Notes

### Why No Condition Codes?

Alpha's designers surveyed modern compiler output and found condition codes were
almost always set immediately before a branch — never read multiple instructions
later.  They concluded condition codes add pipeline complexity without value:
the compiler writer must predict which operation sets which flag; the CPU must
preserve flags across unrelated operations; the microarchitect must track flag
dependencies.  Alpha instead uses compare-to-GPR (`CMPEQ`, `CMPLT`, etc.) and
branches on register value (`BEQ`, `BNE`, `BLT`, etc.).  This matches MIPS but
adds `BLT`/`BLE`/`BGT`/`BGE` to avoid the two-instruction SLT+BNE sequence.

### Why Little-Endian?

All prior simulators in this series (IBM 704, PDP-11, MIPS R2000, SPARC V8, Z80,
Motorola 68000) are big-endian.  Alpha chose little-endian to simplify porting of
DEC VAX software (the VAX was little-endian) and to ease interoperability with the
x86 world DEC was competing against.  The Alpha specification allows both byte
orders via system software, but DEC's implementations used little-endian.

### ADDL / SUBL / MULL — Sign-Extension to 64 bits

The "longword" arithmetic operations (ADDL, SUBL, MULL) perform their computation
in 32-bit arithmetic and then **sign-extend** the 32-bit result to fill all 64 bits.
This ensures that a correctly-written 32-bit program running on Alpha produces
sign-extended results, which other Alpha instructions interpret correctly as
negative numbers.

```
ADDL r1=0x7FFFFFFF, 1, r2:
  32-bit result: 0x80000000
  Sign-extended to 64 bits: 0xFFFFFFFF80000000  (negative)
```

Compare with ADDQ, which yields 0x0000000080000000 (positive).

### Byte Manipulation Instructions

The EXT/INS/MSK/ZAP/ZAPNOT instructions exist because Alpha memory accesses
are naturally aligned.  To access a sub-word quantity at an unaligned address,
code performs two aligned loads and uses byte-manipulation instructions to
extract the desired bytes.  This is more verbose than MIPS's unaligned load
(`LWL`/`LWR`) but preserves the clean "no unaligned hardware" design.

### SETHI Equivalent

Alpha has no SETHI-like instruction (SPARC's 22-bit upper-load).  To load a
64-bit constant, Alpha uses `LDAH` (load address high, an integer arithmetic
instruction with a 16-bit displacement scaled by 65536) combined with `LDA`
(add 16-bit signed offset) to build the constant word.  For this simulator's
64 KiB flat address space, `BIS r31, lit8, Rd` (8-bit immediate) and addition
chains are sufficient for test programs.

---

## Simplifications

1. **No FPU** — All f0–f31 registers and floating-point instructions are omitted.
2. **No TLB / MMU** — 64 KiB flat address space; no virtual memory.
3. **No lock semantics** — LDL_L/LDQ_L treated as LDL/LDQ; STL_C/STQ_C not
   implemented (no need without multi-processor context).
4. **Only HALT PALcode** — `call_pal 0x0000` halts; any other PALcode raises.
5. **No privilege levels** — No kernel/user mode distinction.
6. **No branch prediction** — Branches execute immediately; hint field ignored.
7. **No LDAH/LDA** — These two instructions load addresses from a 16-bit immediate
   scaled by 65536 or 1, intended for building 32-bit constants from two instructions.
   They use the Memory format (opcodes 0x09 and 0x08) and are useful in real
   programs, but since our test programs use `BIS r31, lit8, Rd` for small
   constants and direct address loads for larger ones, we omit them for simplicity.
   (LDAH = 0x09, LDA = 0x08 — not implemented; raise ValueError if encountered.)

---

## Test Plan

| Test module | What it covers |
|-------------|----------------|
| test_protocol.py | isinstance, all 5 methods, return types |
| test_protocol.py | reset() → PC=0, all regs=0, memory zeroed |
| test_protocol.py | load() → places bytes at 0x0000 (little-endian) |
| test_protocol.py | execute() → ExecutionResult with correct fields |
| test_protocol.py | step() → StepTrace with pc_before/pc_after |
| test_protocol.py | get_state() → frozen snapshot, not mutated by step |
| test_instructions.py | LDQ/STQ round-trip; LDL sign-extension |
| test_instructions.py | LDBU/LDWU zero-extension; STB/STW/STL |
| test_instructions.py | ADDQ/SUBQ/MULQ (64-bit) |
| test_instructions.py | ADDL/SUBL/MULL sign-extension of 32-bit result |
| test_instructions.py | CMPEQ/CMPLT/CMPLE/CMPULT/CMPULE (0/1 result) |
| test_instructions.py | AND/BIC/BIS/ORNOT/XOR/EQV |
| test_instructions.py | CMOV variants (condition true/false) |
| test_instructions.py | SLL/SRL/SRA by immediate and register |
| test_instructions.py | EXT*/INS*/MSK* byte manipulation |
| test_instructions.py | ZAPNOT/ZAP byte masking |
| test_instructions.py | SEXTB/SEXTW sign extension |
| test_instructions.py | MULQ/UMULH (full 64-bit multiply) |
| test_instructions.py | BEQ/BNE/BLT/BLE/BGT/BGE/BLBC/BLBS/BR/BSR |
| test_instructions.py | JMP/JSR/RET (link register save, jump target) |
| test_coverage.py | r31 always reads 0; writes discarded |
| test_coverage.py | HALT = 0x00000000 stops execution |
| test_coverage.py | Unknown opcode raises ValueError |
| test_coverage.py | Unknown PALcode raises ValueError |
| test_coverage.py | Little-endian byte layout (STQ verify) |
| test_coverage.py | LDL sign-extension boundary: 0x7FFFFFFF vs 0x80000000 |
| test_coverage.py | ADDL overflow wraps in 32-bit domain |
| test_coverage.py | CMOV preserves Rc when condition is false |
| test_coverage.py | max_steps guard terminates infinite loop (BR 0) |
| test_programs.py | Sum 1–10 using ADDQ + BNE loop |
| test_programs.py | Factorial 5! using MULQ loop |
| test_programs.py | Fibonacci (8 terms) in memory using LDQ/STQ |
| test_programs.py | Bubble sort 4 quadwords using CMPLT + CMOVLT |
| test_programs.py | Byte copy using LDBU/STB |
| test_programs.py | Subroutine call/return using BSR/RET |
| test_programs.py | 64-bit range exercise: verify UMULH |
