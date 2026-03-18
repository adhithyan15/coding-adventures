# logic-gates

A Go package implementing fundamental digital logic gates and sequential circuits, from basic Boolean operations up through counters. This is the foundational layer of the computing stack — every CPU, GPU, and digital device is built from these primitives.

## Where this fits in the stack

```
Layer 0: Logic Gates      <-- you are here
Layer 1: Arithmetic (adders, multipliers)
Layer 2: ALU (arithmetic logic unit)
Layer 3: CPU / GPU control
```

Logic gates are the first abstraction above transistors. From just a single gate type (NAND or NOR), you can build every digital circuit that has ever existed.

## What's included

### Combinational Logic (`gates.go`)

Seven fundamental gates with full truth tables:

| Gate | Function | Description |
|------|----------|-------------|
| `AND(a, b)` | `a & b` | Output 1 only when both inputs are 1 |
| `OR(a, b)` | `a \| b` | Output 1 when at least one input is 1 |
| `NOT(a)` | `^a` | Invert the input |
| `XOR(a, b)` | `a ^ b` | Output 1 when inputs differ |
| `NAND(a, b)` | `^(a & b)` | Universal gate (NOT of AND) |
| `NOR(a, b)` | `^(a \| b)` | Universal gate (NOT of OR) |
| `XNOR(a, b)` | `^(a ^ b)` | Equivalence gate (NOT of XOR) |

NAND-derived gates proving functional completeness:
- `NAND_NOT(a)` -- NOT built from NAND only
- `NAND_AND(a, b)` -- AND built from NAND only (2 gates)
- `NAND_OR(a, b)` -- OR built from NAND only (3 gates)
- `NAND_XOR(a, b)` -- XOR built from NAND only (4 gates)

Multi-input gates:
- `ANDn(inputs...)` -- N-input AND (all must be 1)
- `ORn(inputs...)` -- N-input OR (any must be 1)

### Sequential Logic (`sequential.go`)

Six sequential components that build on each other:

| Component | Function | Description |
|-----------|----------|-------------|
| `SRLatch` | Set-Reset memory | Two cross-coupled NOR gates; simplest memory |
| `DLatch` | Data latch | SR latch with data/enable interface |
| `DFlipFlop` | Edge-triggered memory | Master-slave pair; captures on clock edge |
| `Register` | N-bit storage | N parallel flip-flops sharing a clock |
| `ShiftRegister` | Serial I/O | Chain of flip-flops; shifts bits left or right |
| `Counter` | Self-incrementing register | Counts up on each clock pulse, wraps at max |

## Usage

```go
import logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"

// Basic gates
logicgates.AND(1, 1)   // 1
logicgates.OR(0, 1)    // 1
logicgates.NOT(1)      // 0
logicgates.XOR(1, 0)   // 1

// Multi-input
logicgates.ANDn(1, 1, 1, 1)  // 1
logicgates.ORn(0, 0, 0, 1)   // 1

// D Flip-Flop (store a bit)
state := &logicgates.FlipFlopState{0, 1, 0, 1}
_, _, state = logicgates.DFlipFlop(1, 1, state)  // clock HIGH: master captures
q, _, state := logicgates.DFlipFlop(1, 0, state) // clock LOW: slave outputs
// q == 1

// 4-bit counter
cs := &logicgates.CounterState{Bits: []int{0,0,0,0}, Width: 4}
bits, cs := logicgates.Counter(1, 0, cs) // bits == [1,0,0,0] (count = 1)
bits, cs = logicgates.Counter(1, 0, cs)  // bits == [0,1,0,0] (count = 2)
```

## Input validation

All inputs must be 0 or 1. Functions panic on invalid inputs, matching the hardware behavior where out-of-range voltages cause undefined behavior.

## Testing

```bash
go test ./... -v -cover
```

All tests verify behavior against hardware truth tables. Coverage is 100%.

## Literate programming

All source files use Knuth-style literate programming with extensive comments explaining:
- How each gate works at the transistor level
- ASCII circuit diagrams
- Truth tables for every gate
- Real-world applications (CPUs, GPUs, memory)
- Why each sequential component matters in digital design
