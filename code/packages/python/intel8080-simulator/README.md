# intel8080-simulator

**Layer 4i — Intel 8080 Behavioral Simulator**

A complete Python simulation of the Intel 8080A microprocessor — the CPU inside the
Altair 8800 (1975), the machine that launched the personal computer revolution.

## What It Does

Executes Intel 8080 machine code byte-by-byte. Every instruction in the 8080's
244-opcode ISA is implemented: MOV, MVI, LXI, arithmetic (ADD/ADC/SUB/SBB/DAA),
logical (ANA/ORA/XRA), shifts/rotates, all conditional jumps/calls/returns, PUSH/POP,
IN/OUT, and HLT.

```python
from intel8080_simulator import Intel8080Simulator

sim = Intel8080Simulator()

# MVI A, 0x42; HLT
result = sim.execute(bytes([0x3E, 0x42, 0x76]))
assert result.final_state.a == 0x42
assert result.halted is True
```

## Where It Fits

```
Logic Gates → Arithmetic → CPU → [HERE] → Assembler → Lexer → Parser → Compiler → VM
```

This is Layer 4i alongside: RISC-V (07a), ARM (07b), WASM (07c),
Intel 4004 (07d), ARM1 (07e), Intel 8008 (07f), GE-225 (07g), IBM 704 (07h).

## Architecture

| Feature | Value |
|---------|-------|
| Data width | 8 bits |
| Address space | 64 KiB (16-bit) |
| Registers | A, B, C, D, E, H, L + SP + PC |
| Flags | S, Z, AC, P, CY |
| I/O | 256 input + 256 output ports |
| Stack | RAM-based, grows downward |

## SIM00 Protocol

`Intel8080Simulator` implements `Simulator[Intel8080State]` from
`coding-adventures-simulator-protocol`:

```python
sim.load(program: bytes)              # Load program into memory
sim.step() -> StepTrace               # Execute one instruction
sim.execute(program) -> ExecutionResult  # Run until HLT
sim.get_state() -> Intel8080State     # Frozen snapshot
sim.reset()                           # Clear all state
```

## I/O Ports

```python
sim = Intel8080Simulator()
sim.set_input_port(1, 0xFF)   # Port 1 returns 0xFF on IN 1
result = sim.execute(program)
print(result.final_state.output_ports[2])  # Value written by OUT 2
```

## Historical Context

The 8080 (1974) powered the Altair 8800 (1975). Gary Kildall's CP/M OS targeted it.
Microsoft was founded to write BASIC for it. The 8080's ISA directly influenced the
Z80 and Intel 8086, making it the grandfather of every x86 program ever written.

See `code/specs/07i-intel8080-simulator.md` for the full specification.
