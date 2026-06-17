# sparc-v8-gatelevel

Gate-level Rust implementation of the SPARC V8 processor (1987).

Every data-path operation — arithmetic, logic, shift, multiply, divide — is
implemented using individual logic gates (`and_gate`, `or_gate`, `xor_gate`,
`not_gate`) and the ripple-carry adder from the `arithmetic` crate.  No native
Rust integer operators (`+`, `-`, `&`, `|`, `^`, `!`) are used in the ALU or
CPU data paths.

## Architecture

```
sparc-v8-gatelevel
├── bits.rs         Bit-vector helpers (u32↔Vec<u8>, shifts, sign-extend)
├── alu.rs          ALU: arithmetic, logic, shifts, mul, div, MULScc, SETHI
├── register_file.rs 56 physical registers, CWP, SAVE/RESTORE, PSR
├── decoder.rs      4 SPARC instruction formats (F1/F2/F3r/F3i)
└── cpu.rs          Instruction fetch/decode/execute loop + 30+ unit tests
```

## SPARC V8 features implemented

- **Formats**: CALL (F1), SETHI/Bicc/NOP (F2), ALU/Load/Store (F3)
- **ALU**: ADD, ADDX, SUB, SUBX, AND, ANDN, OR, ORN, XOR, XNOR (all with/without `cc`)
- **Shifts**: SLL, SRL, SRA
- **Multiply**: UMUL, SMUL, UMULcc, SMULcc (full 64-bit product → Y:rd)
- **Divide**: UDIV, SDIV (64÷32 with saturation)
- **MULScc**: iterative signed multiply step
- **Branches**: BA/BN/BE/BNE/BL/BLE/BG/BGE/BGU/BLEU/BCC/BCS/BPOS/BNEG/BVC (16 conditions)
- **JMPL**: computed jump, saves return address
- **CALL**: PC-relative 30-bit displacement
- **SAVE/RESTORE**: register window rotation with overflow detection
- **Load**: LD, LDUB, LDUH, LDSB, LDSH
- **Store**: ST, STB, STH
- **WR/RD %y**: Y-register access
- **ta 0** (`0x91D0_2000`): halt convention

## Register windows

SPARC V8 uses a sliding register window mechanism.  With `NWINDOWS=3` there
are 56 physical registers (8 globals + 3 × 16 windowed).  Each procedure
sees 32 logical registers; `SAVE` rotates the window pointer (`CWP`) so the
caller's out-registers become the callee's in-registers — enabling zero-copy
argument passing.

```
  Physical layout:
  [0..7]   %g0–%g7  globals (all windows)
  [8..23]  window 0 outs/locals  (CWP=0: %o0–%l7)
  [24..39] window 1 outs/locals  (CWP=1: %o0–%l7)
  [40..55] window 2 outs/locals  (CWP=2: %o0–%l7)

  Logical-to-physical (CWP=0):
  %o0 (l8)  → phys 8
  %l0 (l16) → phys 16
  %i0 (l24) → phys 24  (= window 1's %o0)
```

## Usage

```rust
use coding_adventures_sparc_v8_gatelevel::SparcCpu;

let mut cpu = SparcCpu::new();
let program = assemble_your_program();
cpu.execute(&program, 0x0000, 100_000).expect("run failed");
println!("result in %o0: {}", cpu.rf.read(8));
```

## Halting

Programs must end with `ta 0` = `0x91D0_2000` (big-endian bytes `91 D0 20 00`).

## Memory

64 KiB flat, big-endian.  All addresses are masked to 16 bits.

## Testing

```
cargo test --package coding-adventures-sparc-v8-gatelevel
```
