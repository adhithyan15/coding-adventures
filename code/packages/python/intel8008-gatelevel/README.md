# Intel 8008 Gate-Level Simulator

A gate-level simulation of the Intel 8008 microprocessor. Every arithmetic
operation routes through real logic gate functions — AND, OR, XOR, NOT —
chained into half-adders, full-adders, an 8-bit ripple-carry adder, and an ALU.

## What makes this "gate-level"?

In a behavioral simulator, `A + B` is computed with Python's `+` operator.
In this gate-level simulator, the same addition flows through:

```
int_to_bits → full_adder × 8 → ripple_carry_adder → bits_to_int
   ↑              ↑                    ↑
XOR/AND gates  XOR/AND/OR gates    chains 8 adders
```

The 8-bit ALU requires 8 full adders (vs 4 for the Intel 4004), each built
from 5 logic gates, for a total of 40 gates just for addition. Parity is
computed via a 7-gate XOR reduction tree. The whole CPU uses approximately
1,100 logic gates, modeling the same computation the real 3,500-transistor
chip performed.

## Layer position

```
[Logic Gates] → [Arithmetic] → [CPU] → [YOU ARE HERE] → Assembler → ...
     ↑              ↑                         ↑
  AND/OR/NOT     Adders/ALU             8008 wiring
```

This package composes:
- `logic-gates`: AND, OR, XOR, NOT, XOR_N (for parity), register
- `arithmetic`: ALU(bit_width=8), ripple_carry_adder
- `intel8008-simulator`: Intel8008Flags, Intel8008Trace (shared data types)

## Usage

```python
from intel8008_gatelevel import Intel8008GateLevel

cpu = Intel8008GateLevel()

# Same program API as the behavioral simulator
program = bytes([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76])
traces = cpu.run(program)

print(f"A = {cpu.a}")           # 3
print(f"Gates: {cpu.gate_count()}")  # {'alu': 84, 'registers': 480, ...}
```

## Cross-validation

The gate-level simulator produces identical results to the behavioral simulator
for any program. Use the cross-validation test to verify:

```python
from intel8008_simulator import Intel8008Simulator
from intel8008_gatelevel import Intel8008GateLevel

program = bytes([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76])
bsim = Intel8008Simulator()
gsim = Intel8008GateLevel()
b_traces = bsim.run(program)
g_traces = gsim.run(program)
for bt, gt in zip(b_traces, g_traces):
    assert bt.a_after == gt.a_after
    assert bt.flags_after == gt.flags_after
```
