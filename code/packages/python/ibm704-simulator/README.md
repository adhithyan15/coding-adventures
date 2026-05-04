# `ibm704-simulator`

A behavioral simulator for the **IBM 704 Electronic Data-Processing Machine**
(1954) — the first mass-produced computer with hardware floating-point, the
machine **FORTRAN I** was developed for, and the machine **LISP** was first
implemented on. The names `CAR` and `CDR` come from this machine's instruction-
field names.

This is Layer 4h of the computing stack in this repo, sitting alongside
the Intel 4004 (07d), 8008 (07f), ARM1 (07e), RISC-V (07a), and WASM (07c)
simulators.

## What you get

A Python class that takes 36-bit IBM 704 machine words, executes them, and
returns a frozen snapshot of the machine state. The simulator conforms to the
[`Simulator[StateT]`](../simulator-protocol/) protocol, so the same end-to-end
test harness used for every other ISA in this repo works here too.

```python
from ibm704_simulator import IBM704Simulator, IBM704State

sim = IBM704Simulator()
result = sim.execute(program_bytes)

assert result.ok, f"Program failed: {result.error}"
state: IBM704State = result.final_state

print(state.accumulator_magnitude)   # AC bits 3-37
print(state.mq)                      # full MQ register
print(state.memory[0x100])           # 36-bit word at address 256
```

## Architecture in one screen

| Item | Value |
|------|-------|
| Word size | 36 bits |
| Number representation | sign-magnitude (distinct +0 and −0) |
| Accumulator | 38 bits: sign, Q, P, 35-bit magnitude |
| MQ register | 36 bits |
| Index registers | 3 × 15-bit (IRA, IRB, IRC; tag bits 1, 2, 4) |
| Memory | 32,768 36-bit words (15-bit address) |
| Floating-point | sign + 8-bit excess-128 exponent + 27-bit fraction |
| Cycle time | 12 µs (real hardware; simulator runs as fast as Python) |

## Why simulate this machine?

Because it unlocks the entire pre-1971 era of high-level programming languages.
Targeting an IBM 704 simulator gets you:

- **FORTRAN I (1957)** and **FORTRAN II (1958)** — the original language.
  FORTRAN's `INTEGER` and `REAL` *are* 704 word formats.
- **LISP 1 (1958)** and **LISP 1.5 (1962)** — McCarthy's original Lisp ran on
  the 704 at MIT. A cons cell *is* a 704 word with `car` in the address field
  and `cdr` in the decrement field.
- **IPL-V**, **COMIT** — early symbolic / list-processing languages.
- **IBM 7090 / 7094 software** — the 7090 is binary-compatible with the 704
  for the core ISA, so most 704 programs run unmodified.

## Instruction set (v1)

The v1 simulator implements the 40 core instructions needed to host FORTRAN-
style numeric programs and LISP-style cons-cell manipulation. See
[`07h-ibm704-simulator.md`](../../specs/07h-ibm704-simulator.md) for the full
table; in summary:

- **Loads/stores:** CLA, CAL, STO, STZ, LDQ, STQ, XCA
- **Integer arithmetic:** ADD, SUB, ADM, MPY, DVP, DVH
- **Transfers:** TRA, TZE, TNZ, TPL, TMI, TOV, TNO
- **Index-register ops:** LXA, LXD, SXA, SXD, PAX, PDX, PXA, TIX, TXI, TXH, TXL
- **Floating-point:** FAD, FSB, FMP, FDP
- **Control:** HTR, HPR, NOP

I/O, BCD character manipulation, sense lights/switches, and the full shift
family are deferred to v2 — they are not required for hosting numeric or list-
processing language frontends.

## Sign-magnitude — what to know

The 704 stores integers as **sign + 35-bit magnitude**, not two's complement.
Practical consequences:

- `+0` and `−0` are distinct words, but `TZE` (transfer on zero) treats both
  as zero. Equality of integers is *magnitude equality*, not bit equality.
- Addition checks signs first: same-sign → add magnitudes (overflow into the
  Q/P bits); different-sign → subtract smaller magnitude from larger and take
  the sign of the larger.
- The Accumulator is **38 bits** (S + Q + P + 35 magnitude) so overflow is
  representable, not lost. Programs check the overflow trigger via `TOV`.

If you are used to two's-complement bit-twiddling, FORTRAN integer code on
this machine will still feel familiar — FORTRAN I had no bit-twiddling
operators. But low-level code that assumes `~x = -(x+1)` (true under two's
complement) will not work; on the 704 the bitwise complement of `x` is just
`x` with the sign bit flipped, which is `-x` (not `-(x+1)`).

## Program transport

The 704 has no native byte order — it operates on 36-bit words. To fit the
byte-oriented `Simulator` protocol, programs are loaded as packed big-endian
40-bit groups (5 bytes), with the 36-bit word in the low 36 bits and the high
4 bits of byte 0 reserved (must be zero):

```
bytes:  [b0][b1][b2][b3][b4]
bits:    7 0 7 0 7 0 7 0 7 0
         |____|____|____|____|____|
              big-endian, low 36 bits = the word
```

A helper `pack_word(word: int) -> bytes` and matching `unpack_word(b: bytes)
-> int` are exported for tests and assemblers.

## SIM00 protocol conformance

`IBM704Simulator` is structurally compatible with `Simulator[IBM704State]`
from `simulator-protocol`:

| Method | Purpose |
|--------|---------|
| `load(program: bytes)` | Decode 5-byte groups into memory starting at word 0 |
| `step() -> StepTrace` | Execute one instruction, return PC/mnemonic/description |
| `execute(program, max_steps=100_000) -> ExecutionResult[IBM704State]` | Run to halt or limit |
| `get_state() -> IBM704State` | Return frozen snapshot |
| `reset()` | Clear all state to power-on defaults |

## Status

**Alpha — v1 scope.** Core ISA + floating-point. I/O, BCD, full shift family,
sense lights/switches, and trap mechanism are intentionally deferred. See
the spec's "Deferred to v2" section for the full list.

## License

MIT.
