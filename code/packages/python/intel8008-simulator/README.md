# Intel 8008 Simulator

A complete behavioral simulator for the Intel 8008 — the world's first
commercial 8-bit microprocessor, released by Intel in April 1972.

## What is the Intel 8008?

The Intel 8008 was designed by Ted Hoff, Stanley Mazor, and Hal Feeney at
Intel, originally for Computer Terminal Corporation (CTC) who wanted a CPU
for their Datapoint 2200 terminal. CTC ultimately rejected the chip for being
too slow, which allowed Intel to sell it commercially. This decision changed
computing history: the 8008 inspired the 8080, which inspired the Z80 and the
x86 architecture — making this humble terminal chip the ancestor of the
processors running the world's computers today.

With ~3,500 transistors, the 8008 was a significant leap from Intel's own 4004
(2,300 transistors, 4-bit): eight times the data width, seven general-purpose
registers instead of one accumulator, a 14-bit address space (16 KiB) instead
of 12-bit ROM + 5-bit RAM, and a deeper hardware stack. It ran at 500–800 kHz
on a two-phase clock, achieving roughly 200–500K instructions per second.

## What this package does

This package implements a **behavioral simulator** — it executes Intel 8008
machine code byte-by-byte, producing correct register and flag results without
modeling internal gate-level hardware. It uses a custom fetch-decode-execute
loop (not the GenericVM used by the 4004 simulator) because the 8008's
variable-length instructions (1, 2, or 3 bytes) and unique push-down stack
architecture require bespoke handling.

The complete 48-instruction set is implemented, including all register and
immediate ALU operations, conditional and unconditional jumps/calls/returns,
restart (RST) instructions, rotate operations, and I/O port access.

## Usage

```python
from intel8008_simulator import Intel8008Simulator

sim = Intel8008Simulator()

# x = 1 + 2: MVI B,1; MVI A,2; ADD B; HLT
program = bytes([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76])
traces = sim.run(program)

print(f"A = {sim.a}")          # 3
print(f"Carry = {sim.flags.carry}")   # False
print(f"Zero  = {sim.flags.zero}")    # False
print(f"Parity= {sim.flags.parity}")  # True (0b00000011 has 2 ones, even)

# Inspect each step
for t in traces:
    print(f"  0x{t.address:04X}: {t.mnemonic}  A: {t.a_before} → {t.a_after}")
```

## Architecture

| Feature | Value |
|---------|-------|
| Data width | 8 bits |
| Registers | A, B, C, D, E, H, L (plus M pseudo-register) |
| Flags | CY, Z, S, P |
| PC | 14 bits (0x0000–0x3FFF) |
| Stack | 8-level push-down (entry 0 = current PC) |
| Memory | 16,384 bytes |
| I/O | 8 input ports, 24 output ports |

## Layer position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → Lexer → Parser
```

This is Layer 4f of the computing stack — an alternative Layer 4 alongside
RISC-V (07a), ARM/ARMv7 (07b), WASM (07c), Intel 4004 (07d), and ARM1 (07e).

For a gate-level simulator that routes every operation through AND/OR/XOR gates,
see the `intel8008-gatelevel` package.
