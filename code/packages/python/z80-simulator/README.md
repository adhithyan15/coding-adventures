# z80-simulator — Layer 07k

Behavioral simulator for the **Zilog Z80** (1976) microprocessor.

The Z80 is a superset of the Intel 8080 — every valid 8080 program runs
unmodified. It adds an alternate register bank (A', F', BC', DE', HL'),
two index registers (IX, IY), a richer instruction set with bit manipulation
(`BIT`, `SET`, `RES`), block memory/search/I-O instructions (`LDIR`, `CPIR`,
`INIR`, …), and three configurable interrupt modes.

The Z80 powered the TRS-80, Sinclair ZX Spectrum, and the majority of CP/M
business machines of the late 1970s and early 1980s. Microsoft BASIC (the
direct successor to Altair BASIC, originally written for the 8080) ran
unmodified on all of these platforms.

## Quick start

```python
from z80_simulator import Z80Simulator

sim = Z80Simulator()
result = sim.execute(bytes([
    0x3E, 0x0A,   # LD A, 10
    0xC6, 0x05,   # ADD A, 5
    0x76,         # HALT
]))
assert result.final_state.a == 15
```

## Architecture highlights

- **Registers**: A, B, C, D, E, H, L + alternate bank A', F', B', C', D', E', H', L'
- **Index registers**: IX, IY with signed 8-bit displacement addressing
- **Flags**: S, Z, H, P/V, N, C  (H = half-carry, N = subtract, P/V = parity/overflow)
- **Prefix system**: `CB` (bit ops), `ED` (extended), `DD`/`FD` (IX/IY variants)
- **Block ops**: LDIR, LDDR, LDI, LDD, CPIR, CPDR, INIR, OTIR, …
- **I/O**: 256-port separate address space via `IN`/`OUT`
- **Interrupts**: IM 0/1/2 + NMI; `interrupt(data)` and `nmi()` methods

## Relationship to other simulators

| Layer | Processor | Year |
|-------|-----------|------|
| 07i   | Intel 8080 | 1974 — Z80's direct ancestor |
| **07k** | **Zilog Z80** | **1976 — this package** |
| 07j   | MOS 6502  | 1975 — contemporary rival |

## SIM00 protocol

Implements `Simulator[Z80State]`:
`reset()`, `load()`, `step()`, `execute()`, `get_state()`,
`set_input_port()`, `get_output_port()`, `interrupt()`, `nmi()`.
