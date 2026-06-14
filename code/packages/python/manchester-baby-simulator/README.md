# manchester-baby-simulator

Behavioral simulator for the **Manchester Baby (SSEM, 1948)** — the world's
first stored-program computer.

Layer **07l** in the CPU simulator stack.

## What is the Manchester Baby?

The Small-Scale Experimental Machine (SSEM), nicknamed the "Baby", ran the
world's first stored-program on 21 June 1948 at the University of Manchester.
Designed by Freddie Williams, Tom Kilburn, and Geoff Tootill, it proved that
Williams tubes (modified CRT screens) could serve as random-access read-write
memory.

The machine had only **32 × 32-bit words** of store and **7 instructions**.
Its first program found the highest proper divisor of 2¹⁸ = 262 144 by
repeated subtraction — running for 52 minutes and executing 3.5 million
operations.

## Architecture at a Glance

| Feature     | Value                              |
|-------------|------------------------------------|
| Word size   | 32 bits (two's complement)         |
| Store       | 32 words (Williams tube CRT)       |
| Registers   | Accumulator (A), CI (≈ PC, 5-bit)  |
| ISA         | 7 instructions                     |
| I/O         | None                               |
| Halt        | STP instruction                    |

## Instruction Set

| Opcode (F) | Mnemonic | Operation            |
|-----------|----------|----------------------|
| 000       | JMP S    | CI ← Store[S]        |
| 001       | JRP S    | CI ← CI + Store[S]   |
| 010       | LDN S    | A ← −Store[S]        |
| 011       | STO S    | Store[S] ← A         |
| 100/101   | SUB S    | A ← A − Store[S]     |
| 110       | CMP      | if A < 0: CI += 1    |
| 111       | STP      | halt                 |

## Usage

```python
from manchester_baby_simulator import BabySimulator

sim = BabySimulator()

# Encode a word: LDN 0 (F=010, S=0) → (0b010 << 13) | 0 = 0x4000
LDN_0 = 0b010 << 13   # 0x4000 — load negative of Store[0]
STO_1 = (0b011 << 13) | 1   # store A into line 1
STP   = 0b111 << 13         # halt

def word_to_bytes(w: int) -> bytes:
    return w.to_bytes(4, "little")

# Program: negate the value at line 0 and store it at line 1
prog = word_to_bytes(42) + word_to_bytes(0) + word_to_bytes(LDN_0) + word_to_bytes(STO_1) + word_to_bytes(STP)
# Line 0: data = 42
# Line 1: data = 0 (destination)
# Line 2: LDN 0  → A = -42
# Line 3: STO 1  → Store[1] = -42
# Line 4: STP    → halt

result = sim.execute(prog)
print(result.final_state.store[1])   # 0xFFFFFFD6 = -42 in two's complement
```

## Layer Context

This is Layer 07l in the tik-tok alternating sequence between post-4004 ICs
and pre-4004 mainframes:

```
07k  Z80 (1976)              ← post-4004 IC
07l  Manchester Baby (1948)  ← pre-4004 mainframe  ← this package
07m  Intel 8086 (1978)       ← post-4004 IC (next)
07n  EDSAC (1949)            ← pre-4004 mainframe
```

See `code/specs/CPU-SIMULATOR-ROADMAP.md` for the full roadmap.
