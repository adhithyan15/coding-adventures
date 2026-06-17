# motorola68k-gatelevel

Gate-level Motorola 68000 (1979) simulator in Rust. Every arithmetic and
logic operation routes through AND, OR, XOR, NOT gates and a 32-stage
ripple-carry adder — no integer primitives in the data path.

## Architecture

The Motorola 68000 was a landmark 16/32-bit processor introduced in 1979.
Implemented in NMOS at 3.5-micron, it contained ~68,000 transistors and
offered a 24-bit flat linear address space (16 MB) with a clean,
orthogonal instruction set that influenced processor design for decades.
It powered the Apple Lisa, Macintosh, Amiga, Atari ST, and Sun-1
workstations, among many others.

### Registers

| Group           | Registers | Width  | Notes |
|-----------------|-----------|--------|-------|
| Data            | D0–D7     | 32-bit | Byte/word partial writes preserve upper bits |
| Address         | A0–A7     | 32-bit | A7 = supervisor stack pointer; word writes sign-extend |
| Program Counter | PC        | 24-bit | 16 MB flat address space |
| Status Register | SR        | 16-bit | System byte (bits 15-8) + CCR (bits 4-0) |

### Condition Code Register (CCR)

| Bit | Flag | Meaning |
|-----|------|---------|
| 4   | X    | Extend — same as C for ADD/SUB; input to ADDX/SUBX/NEGX |
| 3   | N    | Negative — set when MSB of result is 1 |
| 2   | Z    | Zero — set when all result bits are 0 |
| 1   | V    | Overflow — signed overflow detected |
| 0   | C    | Carry/borrow |

Unlike the Intel 8086, the 68000 has **no AF (auxiliary carry) flag** and
**no PF (parity) flag**. The X flag is separate from C and is only
modified by arithmetic operations, not logic operations.

### Gate-level data path

```
bits.rs
  int_to_bits8/16/32 → LSB-first bit vectors
  add_8/16/32bit_full → ripple-carry adder (N full_adder stages)
  compute_v_from_carries → XOR(carries[N-2], carries[N-1])
  compute_n8/16/32 → MSB extraction
  compute_z / compute_z8/16/32 → NOR tree (OR-fold + NOT)
  not_8/16/32bit → per-bit NOT gate pass

alu.rs
  AluResult68K { result, flag_c, flag_v, flag_z, flag_n, flag_x }
  add8/16/32, sub8/16/32, neg8/16/32, negx8/16/32
  and8/16/32, or8/16/32, xor8/16/32, not8/16/32_flags, cmp8/16/32
  shift_op: ASL/ASR/LSL/LSR/ROXL/ROXR/ROL/ROR

registers.rs
  RegisterFile68K: D0–D7, A0–A7, PC, SR
  read/write_dn with size (byte/word/long, upper bits preserved)
  write_an: word size sign-extends to 32 bits
  set_ccr / set_nzvc_x / set_nz_clear_vc / negx_z
  test_cc: all 16 condition codes (T, F, HI, LS, CC/HI, CS/LO, …)

cpu.rs
  Cpu68K: 16 MB flat memory (heap-allocated Vec<u8>, big-endian byte order)
  EA resolution: all 14 addressing modes
  ~100 opcodes across instruction lines 0–9, B–E
```

### Subtraction model

Subtraction follows the two's-complement identity `A − B = A + NOT(B) + 1`.
Every NOT is performed gate-by-gate through `not_8bit`/`not_16bit`/
`not_32bit`. The carry flag for SUB is **inverted carry-out** (borrow
convention): C=1 means a borrow occurred.

SUBX uses `A + NOT(B) + NOT(X)` to inject the extend flag as borrow-in.

### NEG special rules

- **C flag**: `OR-reduction(result bits)` — C=1 when result ≠ 0.
- **V flag**: overflow only when negating the most-negative value
  (0x80, 0x8000, 0x80000000 — the only value whose negation overflows).
- **NEGX Z flag**: Z is only *cleared*, never set — AND(old_Z, result_Z).

### ADDX/SUBX Z-flag rule

The Z flag accumulates across a multi-precision chain: each ADDX/SUBX
ANDs the incoming Z with the current result's Z. This lets a sequence of
ADDX/SUBX instructions produce Z=1 only if **every** partial result was
zero.

## Usage

```rust
use coding_adventures_motorola68k_gatelevel::cpu::Cpu68K;

let mut cpu = Cpu68K::new();
// MOVEQ #5, D0; MOVEQ #3, D1; ADD.L D1, D0; TRAP #15
let steps = cpu.execute(&[
    0x70, 0x05,  // MOVEQ #5, D0
    0x72, 0x03,  // MOVEQ #3, D1
    0xD0, 0x81,  // ADD.L D1, D0
    0x4E, 0x4F,  // TRAP #15 (halt without disturbing SR)
], 1000);
assert_eq!(cpu.rf.d[0], 8);
assert_eq!(cpu.rf.flag_c(), 0);
assert!(cpu.halted);
```

## Covered instructions

| Line | Instructions |
|------|-------------|
| 0    | ADDI, ANDI, CMPI, EORI, ORI, SUBI, BTST/BSET/BCLR/BCHG (imm) |
| 1–3  | MOVE.B / MOVE.W / MOVE.L / MOVEA |
| 4    | CLR, EXT, ILLEGAL, LINK, NEG, NEGX, NOT, PEA, RESET, STOP, SWAP, TRAP, UNLK, TST |
| 5    | ADDQ, SUBQ, DBcc, Scc |
| 6    | BRA, BSR, Bcc (all 14 conditions) |
| 7    | MOVEQ |
| 8    | OR, DIVU/DIVS (host arithmetic), SBCD |
| 9    | SUB, SUBA, SUBX |
| B    | CMP, CMPA, CMPM, EOR |
| C    | AND, MULU/MULS (host arithmetic), ABCD, EXG |
| D    | ADD, ADDA, ADDX |
| E    | ASL/ASR, LSL/LSR, ROL/ROR, ROXL/ROXR (register and memory forms) |

## Limitations

- **MUL/DIV**: use host arithmetic — a gate-level ×16 booth multiplier
  is out of scope for this simulator.
- **Supervisor/user mode**: the simulator initialises SR=0x2700
  (supervisor mode, interrupts masked) but does not enforce privilege.
- **Interrupts and exceptions**: not simulated; TRAP #15 is used as a
  soft halt.
- **Word alignment**: not enforced (real 68000 raises address error on
  misaligned word/long accesses).

## How it fits in the stack

This crate is part of the **coding-adventures** gate-level CPU simulator
series (spec layer 07n2). Sibling crates implement the Z80 (07h2),
MOS 6502 (07i2), Intel 8080 (07j2), Intel 8086 (07m2), PowerPC 601
(07u2), and ARMv8-A AArch64 (07v2) at the same level of gate fidelity.
