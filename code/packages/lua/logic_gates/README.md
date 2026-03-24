# logic-gates

The fundamental building blocks of all digital circuits. Every computation a
computer performs ultimately reduces to combinations of these gates.

## Where this fits in the stack

```
[YOU ARE HERE] -> Arithmetic -> CPU -> ARM -> Assembler -> Lexer -> Parser -> Compiler -> VM
```

**Input from:** Nothing — this is the foundation.
**Output to:** Arithmetic package (half adders, full adders, ALU).

## What's included

### Combinational Gates (gates.lua)

| Function | Description | Truth Table |
|----------|-------------|-------------|
| `AND(a, b)` | 1 only when BOTH inputs are 1 | 0,0->0 0,1->0 1,0->0 1,1->1 |
| `OR(a, b)` | 1 when AT LEAST ONE input is 1 | 0,0->0 0,1->1 1,0->1 1,1->1 |
| `NOT(a)` | Inverts the input | 0->1 1->0 |
| `XOR(a, b)` | 1 when inputs DIFFER | 0,0->0 0,1->1 1,0->1 1,1->0 |
| `NAND(a, b)` | NOT(AND) — the universal gate | 0,0->1 0,1->1 1,0->1 1,1->0 |
| `NOR(a, b)` | NOT(OR) | 0,0->1 0,1->0 1,0->0 1,1->0 |
| `XNOR(a, b)` | 1 when inputs are SAME | 0,0->1 0,1->0 1,0->0 1,1->1 |

### NAND-Derived Gates (proving functional completeness)

| Function | Description |
|----------|-------------|
| `NAND_NOT(a)` | NOT built from NAND only (1 gate) |
| `NAND_AND(a, b)` | AND built from NAND only (2 gates) |
| `NAND_OR(a, b)` | OR built from NAND only (3 gates) |
| `NAND_XOR(a, b)` | XOR built from NAND only (4 gates) |

### Multi-Input Gates

| Function | Description |
|----------|-------------|
| `ANDn(...)` | 1 only when ALL inputs are 1 |
| `ORn(...)` | 1 when ANY input is 1 |

### Sequential Logic (sequential.lua)

| Function | Description |
|----------|-------------|
| `SRLatch(set, reset, q, q_bar)` | Set-Reset latch (simplest memory) |
| `DLatch(data, enable, q, q_bar)` | Data latch (transparent when enabled) |
| `DFlipFlop(data, clock, state)` | Edge-triggered flip-flop |
| `Register(data, clock, state)` | N-bit parallel storage |
| `ShiftRegister(serial_in, clock, state, direction)` | Serial shift register |
| `Counter(clock, reset, state)` | Binary counter with ripple carry |

## Usage

```lua
local lg = require("coding_adventures.logic_gates")

-- Combinational gates
print(lg.AND(1, 1))   -- 1
print(lg.XOR(1, 0))   -- 1
print(lg.NAND(1, 1))  -- 0

-- NAND-derived (same results, built from NAND only)
print(lg.NAND_AND(1, 1))  -- 1

-- Multi-input
print(lg.ANDn(1, 1, 1, 1))  -- 1

-- Counter
local state = lg.new_counter_state(4)
for i = 1, 5 do
    local outputs
    outputs, state = lg.Counter(1, 0, state)
end
-- state.bits is now {1, 0, 1, 0} (binary 5, LSB first)
```

## Development

```bash
cd tests && busted . --verbose --pattern=test_
```
