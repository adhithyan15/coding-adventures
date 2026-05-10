# mos6502-simulator

Behavioral simulator for the MOS Technology 6502 (NMOS) microprocessor — Layer 07j.

The 6502 (1975) powered the Apple II, Commodore 64, BBC Micro, Atari 2600,
and NES/Famicom. At $25 (vs $179 for the Intel 8080), it democratised
personal computing.

## Features

- All 151 official opcodes, all 13 addressing modes
- Correct flag behaviour (N, V, B, D, I, Z, C)
- BCD decimal-mode ADC/SBC (NMOS behaviour)
- The infamous indirect JMP page-wrap bug replicated
- Memory-mapped I/O (0xFF00–0xFFEF)
- SIM00 `Simulator[MOS6502State]` protocol

## Usage

```python
from mos6502_simulator import MOS6502Simulator

sim = MOS6502Simulator()

# Sum 1..10 = 55
result = sim.execute(bytes([
    0xA9, 0x00,           # LDA #0
    0xA2, 0x0A,           # LDX #10
    0x8E, 0x00, 0x02,     # STX $0200
    0x18,                 # CLC
    0x6D, 0x00, 0x02,     # ADC $0200
    0xCA,                 # DEX
    0xD0, 0xF7,           # BNE loop (-9)
    0x00,                 # BRK
]))
print(result.final_state.a)   # 55
```

## Layer Position

```
logic-gates → arithmetic → simulator-protocol → [YOU ARE HERE]
```

See spec: `code/specs/07j-mos6502-simulator.md`
