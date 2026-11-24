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

## Input validation and errors

A bit is valid only if it is the Lua **integer** 0 or 1 (`math.type(v) == "integer"`).
The floats `1.0`/`0.0`, strings, booleans, `nil`, tables etc. are rejected, because
`1.0 == 1` is true in Lua 5.3+ and would otherwise slip through.

Invalid input raises a structured error table, not a string:

```lua
local ok, err = pcall(lg.AND, 1.0, 1)
-- err.code  == "INVALID_BIT"
-- err.param == "a"
-- lg.is_invalid_bit(err) == true;  tostring(err) is a readable message
```

Extra arguments raise `INVALID_ARITY` (`lg.is_invalid_arity(err)`).

## Hardening (0.6.0)

- Table arguments must be plain tables (no metatable) with keys exactly
  `1..n`; holes, extra keys and oversize lists raise structured errors
  (`INVALID_SHAPE` / `INVALID_WIDTH`) instead of being truncated.
- `ANDn` / `ORn` accept 2..`MAX_INPUTS` (65536) inputs.
- Errors are read-only objects recognised by identity: use
  `is_invalid_bit`, `is_invalid_arity`, `is_invalid_width`,
  `is_invalid_shape`, `is_internal` or `error_code(err)`.
- All standard-library functions and sibling gates are captured at load time;
  the exported tables are read-only proxies. `rawset` and the debug library
  still defeat a proxy -- that is the host's sandbox boundary.
- Always identify errors with `error_code(err)` or the `is_*` predicates;
  `rawset` can shadow `err.code` on the read-only proxy, but the predicates
  and `tostring` never read it.
- The error-minting kit is private (built once by `gates.lua`, passed to the
  sequential factory); no public path returns it.
- Each CMOS `evaluate_digital` is captured at load and called through a
  captured `pcall`; failures or non-0/1 results raise structured `INTERNAL`.
  The `transistors` dependency itself is INSIDE the trust boundary (its
  internal sub-gate lookups use its own mutable class tables), so a valid call
  under tampering is either correct or `INTERNAL`.
