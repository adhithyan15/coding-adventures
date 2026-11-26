# Logic Gates (TypeScript)

**Layer 1 of the computing stack** — the foundation of all digital logic.

## What is this?

This package implements the seven fundamental logic gates (NOT, AND, OR, XOR, NAND, NOR, XNOR), proves that all of them can be built from NAND alone (functional completeness), and provides sequential logic components (latches, flip-flops, registers, counters).

All code uses Knuth-style literate programming — every function includes detailed explanations, truth tables, circuit diagrams, and real-world analogies inline with the implementation.

## Gates

- **NOT** — inverts a single bit
- **AND** — outputs 1 only when both inputs are 1
- **OR** — outputs 1 when at least one input is 1
- **XOR** — outputs 1 when inputs differ
- **NAND** — NOT AND (functionally complete)
- **NOR** — NOT OR (functionally complete)
- **XNOR** — equivalence gate (outputs 1 when inputs match)

## NAND-derived gates

Proves functional completeness by building every gate from NAND alone:

- `nandNot(a)` — NOT from NAND
- `nandAnd(a, b)` — AND from 2 NANDs
- `nandOr(a, b)` — OR from 3 NANDs
- `nandXor(a, b)` — XOR from 4 NANDs
- `nandNor(a, b)` — NOR from NANDs
- `nandXnor(a, b)` — XNOR from NANDs

## Multi-input gates

- `andN(...inputs)` — N-input AND
- `orN(...inputs)` — N-input OR

## Selectors

- `mux(a, b, sel)` — 2-to-1 multiplexer
- `dmux(input, sel)` — 1-to-2 demultiplexer

## Sequential logic

- `srLatch` — SR latch (2 cross-coupled NOR gates)
- `dLatch` — D latch with enable control
- `dFlipFlop` — master-slave edge-triggered flip-flop
- `register` — N-bit parallel register
- `shiftRegister` — serial-to-parallel shift register
- `counter` — binary counter with reset

## Input validation

Every gate (NOT, AND, OR, XOR, NAND, NOR, XNOR and the NAND-derived gates)
validates its own arguments with `validateBit`:

- not a primitive number (boolean, string, bigint, null, undefined, object,
  symbol, function) -> `TypeError`
- a number that is not an integer (NaN, Infinity, 0.5, 1.5) -> `TypeError`
- an integer other than 0 or 1 -> `RangeError`
- `-0` is rejected (`RangeError`); it is not the bit 0
- `1.0` is accepted: JavaScript has no integer type and `1.0 === 1`
- arity is checked: too few or too many arguments -> `TypeError`

Version 0.2.0 is breaking for `1.5`: it is now a `TypeError` (it used to be a
`RangeError` from AND/OR/XOR).

## Where it fits

```
[Logic Gates] -> Arithmetic -> CPU -> ARM/RISC-V -> Assembler -> Lexer -> Parser -> Compiler -> VM
```

This package is used by the **arithmetic** package to build half adders, full adders, and the ALU.

## Installation

```bash
npm install @coding-adventures/logic-gates
```

## Usage

```typescript
import { AND, OR, NOT, XOR, NAND, andN, type Bit } from "@coding-adventures/logic-gates";

AND(1, 1);    // 1 — both inputs are 1
AND(1, 0);    // 0 — one input is 0
OR(0, 1);     // 1 — at least one input is 1
NOT(1);       // 0 — inverted
XOR(1, 0);    // 1 — inputs are different
XOR(1, 1);    // 0 — inputs are the same
andN(1, 1, 1, 0);  // 0 — one input is 0
```

## Spec

See [01-logic-gates.md](../../../specs/10-logic-gates.md) for the full specification.

## Resource bounds and state validation (0.3.0)

`decoder` and `demuxN` accept at most `MAX_SELECT_BITS = 16` select bits;
`register` and `shiftRegister` widths are integers in `1..MAX_WIDTH` (65536) and
`counter` in `1..MAX_COUNTER_WIDTH` (64). Violations throw `RangeError` (or
`TypeError` for non-integers) before any allocation. Caller-supplied sequential
state is fully validated on every call and never returned unvalidated.
