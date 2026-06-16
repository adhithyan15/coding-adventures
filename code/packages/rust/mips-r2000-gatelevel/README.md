# mips-r2000-gatelevel

Gate-level MIPS R2000 (1985) simulator in Rust.  Every arithmetic and logical
data-path operation routes through AND, OR, XOR, NOT gates and a ripple-carry
adder — no native integer arithmetic in the data path.

## Architecture

The MIPS R2000 was the first commercial chip implementing the MIPS I
Instruction Set Architecture (ISA).  Introduced in 1985 by MIPS Computer
Systems Inc. (founded by Stanford professor John Hennessy), it was
manufactured in CMOS at 2-micron feature size and operated at up to 16 MHz.
The R2000 defined "Reduced Instruction Set Computing" in practice: a small,
regular instruction set where every instruction executes in one clock cycle
(in the pipeline).

Unlike the Complex Instruction Set Computers (CISCs) of the era (Intel 8086,
Motorola 68000), the R2000 has only three instruction formats, 32-bit
fixed-width instructions, 32 general-purpose registers, and no condition-code
flags.

### Register file

| Registers | Count | Purpose |
|-----------|-------|---------|
| `$zero` (R0) | 1 | Hardwired 0 — writes are discarded |
| `$at` (R1) | 1 | Assembler temporary |
| `$v0`–`$v1` (R2–R3) | 2 | Return values |
| `$a0`–`$a3` (R4–R7) | 4 | Function arguments |
| `$t0`–`$t7` (R8–R15) | 8 | Temporaries (not preserved across calls) |
| `$s0`–`$s7` (R16–R23) | 8 | Saved temporaries (preserved across calls) |
| `$t8`–`$t9` (R24–R25) | 2 | More temporaries |
| `$k0`–`$k1` (R26–R27) | 2 | OS kernel reserved |
| `$gp` (R28) | 1 | Global pointer |
| `$sp` (R29) | 1 | Stack pointer |
| `$fp` (R30) | 1 | Frame pointer |
| `$ra` (R31) | 1 | Return address (set by JAL/JALR/BGEZAL/BLTZAL) |
| HI | 1 | High 32 bits of MULT/DIV result |
| LO | 1 | Low 32 bits of MULT/DIV result |
| PC | 1 | Program Counter |

### Instruction formats

```text
R-type (op=0):
  ┌────────┬─────┬─────┬─────┬───────┬────────┐
  │ op (6) │rs(5)│rt(5)│rd(5)│shamt(5)│funct(6)│
  └────────┴─────┴─────┴─────┴───────┴────────┘

I-type:
  ┌────────┬─────┬─────┬──────────────────┐
  │ op (6) │rs(5)│rt(5)│    imm16 (16)    │
  └────────┴─────┴─────┴──────────────────┘

J-type:
  ┌────────┬───────────────────────────────┐
  │ op (6) │         target26 (26)         │
  └────────┴───────────────────────────────┘
```

### Gate-level data path

```text
bits.rs
  int_to_bits32/64 → LSB-first bit vectors
  add_32bit        → 33-bit ripple-carry adder → (result, carry_out, overflow)
                     overflow = XOR(carry_into_bit31, carry_out_of_bit31)
  add_64bit        → 64-bit ripple-carry adder → (result, carry_out)
  invert_32bit     → 32 NOT gates in parallel
  shl_32           → barrel-shifter model (bit-list rotation)
  shr_32_logical   → zero-fill right shift
  shr_32_arith     → sign-fill right shift
  compute_zero     → NOR reduction tree

alu.rs
  AluResult32 { result, carry, overflow, zero, negative }
  add32(a, b, cin)  — 32-bit ripple-carry addition with overflow detection
  sub32(a, b)       — A + NOT(B) + 1 (two's complement via NOT gates)
  and32/or32/xor32  — 32 gate instances in parallel
  nor32(a, b)       — NOT(OR(a, b)) per bit — NOR gate for MIPS NOR instruction
  slt32(a, b)       — XOR(N, V) from sub32; signed less-than
  sltu32(a, b)      — NOT(carry) from sub32; unsigned less-than
  sll32/srl32/sra32 — gate-level barrel-shifter wrappers
  multu32(a, b)     — shift-and-add, 32 iterations, 64-bit result
  mult32(a, b)      — signed multiply: compute magnitudes, negate if signs differ
  divu32(a, b)      — non-restoring long division, 32 iterations
  div32(a, b)       — signed division: compute magnitudes, apply sign rules

register_file.rs
  RegisterFile32: gprs[32][32-bit] + hi + lo + pc (all as LSB-first bit arrays)
  read/write_reg (R0 guard), read/write_hi, read/write_lo, read/write_pc
  increment_pc(4) via gate-level add_32bit

decoder.rs
  decode_instruction(word: u32) → DecodedInstruction
  R-type: extracts op=0, rs, rt, rd, shamt, funct from bit slices
  I-type: extracts op, rs, rt, imm16 (sign-extended via bit 15 replication)
  J-type: extracts op=2/3, target26
  gate-level: all extraction from LSB-first bit arrays, no integer masks

cpu.rs
  CpuMipsR2000: rf + mem[64KB] + halted
  execute(program, origin, max_steps) → Result<u32, MipsError>
  Halt: SYSCALL (op=0, funct=0x0C)
  Memory: big-endian, 64 KB flat; word/halfword alignment enforced
  Errors: MipsError::{SignedOverflow, Misalignment, Break, UnknownOpcode}
```

### Two's complement subtraction

```text
SUB A, B = ADD A, NOT(B), carry_in=1

32 NOT gates invert B; the ripple-carry adder adds A + NOT(B) + 1 = A − B.
carry=1 from sub32 means NO borrow (A ≥ B unsigned).
carry=0 means borrow occurred (A < B unsigned).
```

### Overflow detection

```text
overflow = XOR(carry_into_bit31, carry_out_of_bit31)

Implemented via two ripple-carry sub-adders:
  - 31-bit adder over a[0..30], b[0..30] → carry_out = carry_into_bit31
  - 33-bit adder over a[0..32], b[0..32] → sum[32] = carry_out_of_bit31
    (when a[32]=b[32]=0, full_adder(0,0,c31) produces sum=c31, carry=0)
```

### ADD vs ADDU / ADDI vs ADDIU

MIPS distinguishes "trapping" (signed, exception on overflow) from
"unsigned" (wrapping) variants:
- `ADD`, `ADDI`, `SUB` → raise `MipsError::SignedOverflow` on overflow
- `ADDU`, `ADDIU`, `SUBU` → silently wrap (ignore overflow)

### Branch targets (no delay slots)

```text
branch_target = PC_after_fetch + sext(imm16) * 4
jump_target   = (PC_after_fetch & 0xF000_0000) | (target26 << 2)
```

No delay slots are modeled.  PC is already incremented past the instruction
when branch targets are computed.

### Multiplication and division

`MULT`/`MULTU` use a 32-iteration shift-and-add loop.  Each partial product
is a 64-bit left-shifted copy of the multiplicand, accumulated via
`add_64bit` (gate-level 64-stage ripple-carry adder).

`DIV`/`DIVU` use 32-iteration non-restoring long division.  For each bit
position `i` from 31 to 0, the divisor is left-shifted by `i`; if the
remainder ≥ shifted divisor (tested via `sub32` carry), the shifted divisor
is subtracted and the quotient bit is set.

## Usage

```rust
use coding_adventures_mips_r2000_gatelevel::cpu::CpuMipsR2000;

let mut cpu = CpuMipsR2000::new();

// Encode big-endian MIPS instructions:
// ADDIU $t0, $zero, 5   (op=0x09, rs=0, rt=8, imm=5) → 0x24080005
// ADDIU $t1, $zero, 3   (op=0x09, rs=0, rt=9, imm=3) → 0x24090003
// ADDU  $t2, $t0, $t1   (rs=8, rt=9, rd=10, funct=0x21) → 0x01094021
// SYSCALL               (op=0, funct=0x0C) → 0x0000000C
let prog: &[u8] = &[
    0x24, 0x08, 0x00, 0x05,  // ADDIU $t0, $zero, 5
    0x24, 0x09, 0x00, 0x03,  // ADDIU $t1, $zero, 3
    0x01, 0x09, 0x40, 0x21,  // ADDU $t2, $t0, $t1
    0x00, 0x00, 0x00, 0x0C,  // SYSCALL (halt)
];
cpu.execute(prog, 0, 100).unwrap();
assert_eq!(cpu.rf.read_reg(10), 8); // $t2 = 5 + 3 = 8
assert!(cpu.halted);
```

## Covered instructions

| Class | Instructions | Notes |
|-------|-------------|-------|
| Arithmetic | ADD, ADDU, ADDI, ADDIU, SUB, SUBU | Signed variants trap on overflow |
| Logical | AND, OR, XOR, NOR, ANDI, ORI, XORI | R-type and immediate forms |
| Load upper | LUI | Shifts imm16 to upper 16 bits |
| Compare | SLT, SLTU, SLTI, SLTIU | Signed and unsigned set-less-than |
| Shifts | SLL, SRL, SRA, SLLV, SRLV, SRAV | Fixed and variable shift amounts |
| Multiply | MULT, MULTU, MFHI, MFLO, MTHI, MTLO | 64-bit result in HI/LO |
| Divide | DIV, DIVU | Quotient in LO, remainder in HI |
| Branches | BEQ, BNE, BLEZ, BGTZ, BLTZ, BGEZ | Conditional jumps |
| Branch+link | BGEZAL, BLTZAL | Also sets $ra |
| Jumps | J, JAL | 26-bit target (in 64 KB space) |
| Jump register | JR, JALR | R-type jumps to register |
| Loads | LB, LBU, LH, LHU, LW, LWL, LWR | Byte/half/word with sign/zero ext |
| Stores | SB, SH, SW, SWL, SWR | Byte/half/word aligned and unaligned |
| Misc | NOP, BREAK | BREAK raises MipsError::Break |
| Halt | SYSCALL | Halts simulator; sets `halted = true` |

## Limitations

- **No delay slots**: real MIPS R2000 has branch delay slots.  This simulator
  ignores the instruction in the delay slot (the PC is already past it when
  the branch target is computed).
- **No MMU / TLB**: flat 64 KB address space.
- **No coprocessors**: CP0 (system control), CP1 (FPU) not implemented.
- **Division by zero**: returns `(0xFFFF_FFFF, a)` matching hardware
  undefined behavior — no exception.
- **Unaligned word accesses** (LW/SW to non-4-byte-aligned address) raise
  `MipsError::Misalignment`.  LWL/LWR/SWL/SWR handle unaligned accesses
  deliberately.

## How it fits in the stack

This crate is part of the **coding-adventures** gate-level CPU simulator
series (spec layer 07q2).  Sibling crates implement the Intel 4004 (06i2),
Intel 8008 (07a2), Intel 8080 (07j2), MOS 6502 (07i2), Zilog Z80 (07h2),
Intel 8086 (07m2), Motorola 68000 (07n2), and Intel 8051 (07p2) at the
same level of gate fidelity.
