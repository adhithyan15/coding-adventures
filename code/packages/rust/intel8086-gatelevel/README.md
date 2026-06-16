# coding-adventures-intel8086-gatelevel

Gate-level simulator for the Intel 8086 (1978) written in Rust.

Every arithmetic and logical operation on data routes through real logic gate
functions from the `logic-gates` and `arithmetic` workspace crates — no host
integer arithmetic on the data path (except MUL/DIV, documented below).

## Architecture

```
bits.rs      — integer ↔ LSB-first bit-vector conversion
               8-bit and 16-bit adder wrappers around full_adder chains
               nibble_borrow() — dedicated 4-bit subtractor for the AF flag
alu.rs       — AluResult8086: all arithmetic/logic through gate primitives
               add/sub/and/or/xor/inc/dec/neg/not (8-bit and 16-bit)
               shl/shr/sar/rol/ror/rcl/rcr shifts and rotates
               daa/das/aaa/aas/aam/aad BCD operations
               mul/div (host arithmetic — gate-level ×16 multiplier out of scope)
registers.rs — RegisterFile8086: 14 registers, 9 flag flip-flops
               read16/write16/read8/write8 with ModRM encoding
               pack_flags/unpack_flags for PUSHF/POPF/LAHF/SAHF
               physical_address() — 20-bit segment:offset via add_20bit()
cpu.rs       — Cpu8086: full fetch-decode-execute loop
               ~120 opcodes including ModRM, REP prefix, string ops, I/O ports
```

## Intel 8086 overview

Announced June 1978. First x86 processor.

| Property          | Value                         |
|-------------------|-------------------------------|
| Data bus          | 16-bit                        |
| Address bus       | 20-bit (1 MB physical)        |
| Registers         | AX BX CX DX SI DI SP BP      |
| Segment registers | CS DS SS ES                   |
| Transistors       | ~29,000 (NMOS, 3-micron)      |
| Clock speeds      | 5–10 MHz                      |

### Memory model

Physical address = (segment_reg × 16 + offset) & 0xFFFFF.

The "× 16" is a 4-bit left shift — hardware pin routing, not arithmetic. The
16-bit offset is added through a 20-stage ripple-carry chain (`add_20bit()`).

### FLAGS layout

```
bit 0: CF   bit 1: 1(always)  bit 2: PF   bit 4: AF
bit 6: ZF   bit 7: SF         bit 8: TF   bit 9: IF
bit 10: DF  bit 11: OF
```

### Subtraction and AF flag

SUB/SBB/CMP use two's-complement addition: `A - B = A + NOT(B) + 1`.
The carry out of bit 15 is inverted to produce CF (1 = borrow).

The AF flag cannot be derived from the main adder's carry chain in subtraction
mode. A dedicated `nibble_borrow()` function runs a 4-bit two's-complement
subtractor on the low nibbles to compute AF correctly.

### MUL/DIV exception

Gate-level implementation of a 16×16 multiplier (~1000 gates) is out of scope
for this educational simulator. `mul8`, `mul16`, `imul8`, `imul16`, `div8`,
`div16`, `idiv8`, `idiv16` use host Rust arithmetic.

## Usage

```rust
use coding_adventures_intel8086_gatelevel::cpu::Cpu8086;

let mut cpu = Cpu8086::new();
// MOV AX, 10; MOV BX, 5; ADD AX, BX; HLT
let steps = cpu.execute(&[
    0xB8, 10, 0,   // MOV AX, 10
    0xBB,  5, 0,   // MOV BX, 5
    0x03, 0xC3,    // ADD AX, BX
    0xF4,          // HLT
], 1000);
assert_eq!(cpu.rf.ax, 15);
assert!(cpu.halted);
```

### I/O ports

```rust
cpu.input_ports[0x60] = 0x41; // set port 0x60 to 'A'
// IN AL, 0x60 → AL = 'A'
// OUT 0x61, AL → output_ports[0x61] = 'A'
```

### Direct memory access

```rust
cpu.mem[0x1234] = 0xFF;    // write to physical address 0x1234
let b = cpu.mem[0xABCD];   // read from physical address 0xABCD
```

## Gate cost estimates

| Component                | Approximate gate count |
|--------------------------|----------------------|
| 16-bit ripple adder      | ~80 (16 × 5)         |
| 16-bit NOT (for SUB)     | 16                   |
| 16-bit AND/OR/XOR        | 48 (16 × 3)          |
| Zero NOR tree (16-bit)   | ~20                  |
| Parity XOR tree (8-bit)  | 8                    |
| Overflow XOR gate        | 1                    |
| Shifter / rotator        | ~64                  |
| Nibble borrow (AF)       | ~10                  |
| **ALU total estimate**   | **~247**             |
| Register file (14 × 16) | ~896 flip-flops       |
| FLAGS (9 bits)           | ~9 flip-flops        |

## Relationship to other simulators

This package is part of the `coding-adventures` gate-level CPU series:

| Spec  | CPU                    | Year | Language |
|-------|------------------------|------|----------|
| 07a2  | Manchester Baby        | 1948 | Rust     |
| 07b2  | IBM 701                | 1952 | Rust     |
| 07c2  | IBM 704                | 1954 | Rust     |
| 07d2  | TX-0                   | 1956 | Rust     |
| 07e2  | PDP-1                  | 1959 | Rust     |
| 07f2  | IBM 7090               | 1959 | Rust     |
| 07g2  | CDC 6600               | 1964 | Rust     |
| 07h2  | PDP-8                  | 1965 | Rust     |
| 07i2  | S/360                  | 1966 | Rust     |
| 07j2  | Intel 4004             | 1971 | Rust     |
| 07k2  | Intel 8080             | 1974 | Rust     |
| 07l2  | MOS 6502               | 1975 | Rust     |
| 07m2  | **Intel 8086**         | 1978 | **Rust** |
