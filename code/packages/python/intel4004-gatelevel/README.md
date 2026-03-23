# Intel 4004 Gate-Level Simulator

**Layer 4d2 of the computing stack** — simulates the Intel 4004 where every operation routes through real logic gates.

## What this package does

Unlike the behavioral simulator (which directly computes results), this package routes every computation through the actual gate-level primitives:

- **ALU:** Uses `ALU(bit_width=4)` from the arithmetic package, which internally chains XOR → AND → OR → full_adder → ripple_carry_adder
- **Registers:** 16 × 4-bit registers, each built from D flip-flops (logic_gates.register)
- **Program Counter:** 12-bit register with half-adder incrementer chain
- **Stack:** 3 × 12-bit registers for hardware call stack
- **Decoder:** Combinational AND/OR/NOT gate network for instruction decoding

## Usage

```python
from intel4004_gatelevel import Intel4004GateLevel

cpu = Intel4004GateLevel()
traces = cpu.run(bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]))
assert cpu.registers[1] == 3  # 1 + 2 = 3
print(f"Gate count: {cpu.gate_count()}")
```

## Spec

See [07d2-intel4004-gatelevel.md](../../../specs/07d2-intel4004-gatelevel.md).
