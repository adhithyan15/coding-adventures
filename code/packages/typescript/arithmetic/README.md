# Arithmetic

**Layer 2 of the computing stack** — builds number computation from logic gates.

## What is binary arithmetic?

Computers don't understand decimal numbers (0-9). They only understand binary — 0 and 1. To add two numbers, a computer breaks them into their binary digits (bits) and adds them one bit at a time, just like you add decimal numbers digit by digit with carrying.

```
  Decimal:  5 + 3 = 8
  Binary:   101 + 011 = 1000
```

This package builds addition from the ground up using only the logic gates from Layer 1.

## The circuits

### Half Adder — adding two bits

The simplest possible addition circuit. It adds two single bits and produces two outputs: a **sum** and a **carry**.

```
A  B  |  Sum  Carry
------|------------
0  0  |   0     0     (0 + 0 = 0)
0  1  |   1     0     (0 + 1 = 1)
1  0  |   1     0     (1 + 0 = 1)
1  1  |   0     1     (1 + 1 = 10 in binary — sum is 0, carry the 1)
```

**How it's built:** The sum is `XOR(A, B)` and the carry is `AND(A, B)`. Just two logic gates.

### Full Adder — adding two bits plus a carry

Extends the half adder by accepting a **carry-in** from the previous bit position. Built from two half adders and an OR gate.

### Ripple Carry Adder — adding multi-bit numbers

Chains N full adders together. The carry output of each adder feeds into the carry input of the next one.

```
     A3 B3      A2 B2      A1 B1      A0 B0
      |  |       |  |       |  |       |  |
    +----+     +----+     +----+     +----+
    | FA |<----| FA |<----| FA |<----| FA |<-- 0 (initial carry)
    +----+     +----+     +----+     +----+
       S3         S2         S1         S0
```

### ALU (Arithmetic Logic Unit) — the computational brain

The ALU takes two numbers, an operation code (ADD, SUB, AND, OR, XOR, NOT), and produces a result plus status flags (zero, carry, negative, overflow).

## Where it fits

```
Logic Gates -> [Arithmetic] -> CPU -> ARM/RISC-V -> Assembler -> Lexer -> Parser -> Compiler -> VM
```

## Installation

```bash
npm install @coding-adventures/arithmetic
```

## Usage

```typescript
import { halfAdder, fullAdder, rippleCarryAdder, ALU, ALUOp } from "@coding-adventures/arithmetic";
import type { Bit } from "@coding-adventures/logic-gates";

// Half adder: 1 + 1 = 0 with carry 1 (binary 10)
halfAdder(1, 1);  // [0, 1]

// Full adder: 1 + 1 + carry_in=1 = 1 with carry 1 (binary 11)
fullAdder(1, 1, 1);  // [1, 1]

// Ripple carry: 5 + 3 = 8 (bits are LSB first)
const a: Bit[] = [1, 0, 1, 0];  // 5 = 1*1 + 0*2 + 1*4 + 0*8
const b: Bit[] = [1, 1, 0, 0];  // 3 = 1*1 + 1*2 + 0*4 + 0*8
const [result, carry] = rippleCarryAdder(a, b);
// result = [0, 0, 0, 1]  -> 8 = 0*1 + 0*2 + 0*4 + 1*8

// ALU: 1 + 2 = 3
const alu = new ALU(8);
const aluResult = alu.execute(
  ALUOp.ADD,
  [1,0,0,0,0,0,0,0] as Bit[],
  [0,1,0,0,0,0,0,0] as Bit[]
);
// aluResult.value = [1,1,0,0,0,0,0,0]  -> 3
// aluResult.zero = false, aluResult.carry = false
```

## Spec

See [02-arithmetic.md](../../../specs/09-arithmetic.md) for the full specification.
