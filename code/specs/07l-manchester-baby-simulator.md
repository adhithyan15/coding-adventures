# Spec 07l — Manchester Baby (SSEM) Behavioral Simulator

**Layer**: 07l  
**Architecture**: Manchester Small-Scale Experimental Machine (SSEM)  
**Year**: 1948  
**Package**: `coding-adventures-manchester-baby-simulator`  
**Depends on**: `coding-adventures-simulator-protocol`

---

## Historical Context

The Manchester Baby (SSEM) ran the world's **first stored-program** on
21 June 1948. Designed by Freddie Williams, Tom Kilburn, and Geoff Tootill at
the University of Manchester, it proved that Williams tubes (modified CRT
screens) could serve as random-access read-write memory — a concept that
transformed computing.

The machine was intentionally minimal: a proof-of-concept built to validate
the memory technology, not to be a general-purpose computer. Its tiny
instruction set and 32-word store make it an ideal teaching example for the
fetch-decode-execute cycle, two's-complement arithmetic, and the concept of
self-modifying code.

The famous first program (T.K. 1948) found the **highest proper divisor of
2¹⁸ = 262 144** by testing all potential factors via repeated subtraction.
It ran for 52 minutes and made 3.5 million operations.

---

## Architecture Overview

| Feature         | Value                              |
|-----------------|------------------------------------|
| Word size       | 32 bits (two's complement)         |
| Main store      | 32 words (Williams tube)           |
| Registers       | Accumulator (A), Control Instruction (CI) |
| Instruction set | 7 instructions                     |
| I/O             | None (no I/O instructions)         |
| Halt            | STP instruction                    |
| Addressing      | Absolute, word-granular (0–31)     |

---

## Memory Model

The SSEM has a single flat store of **32 × 32-bit words**, addressed by a
5-bit line number (0–31). Both program and data share this store
(von Neumann architecture). Each word is a 32-bit two's-complement integer.

There is no separate I/O space. This simulator provides no I/O ports.

---

## Registers

| Name | Width | Description |
|------|-------|-------------|
| A    | 32 bit | Accumulator. Only register accessible to the programmer. Initialized to 0. |
| CI   | 5 bit  | Control Instruction counter (≈ program counter). Points to the currently executing line. Initialized to 0x1F (31) so first increment yields 0. |

There is no flags register — the only conditional is the sign of A, tested
by the CMP instruction.

---

## Instruction Encoding

Each instruction is a 32-bit word:

```
Bit:  31  30  ...  18  17  16  15  14  13  12  ...  5   4   3   2   1   0
                                F2  F1  F0                   S4  S3  S2  S1  S0
                       └── function (3 bits) ──┘    └────── operand line (5 bits) ──┘
```

- **S (bits 0–4)**: 5-bit line/address field — the operand (which store line to use)
- **F (bits 13–15)**: 3-bit function code — which instruction to execute
- All other bits are ignored (conventionally zero)

Extraction in Python:
```python
s = word & 0x1F           # bits 0–4
f = (word >> 13) & 0x7    # bits 13–15
```

---

## Instruction Set

| F   | Mnemonic | Full Name | Operation |
|-----|----------|-----------|-----------|
| 000 | JMP S    | Jump      | CI ← Store[S] |
| 001 | JRP S    | Relative Jump | CI ← CI + Store[S] |
| 010 | LDN S    | Load Negative | A ← −Store[S] |
| 011 | STO S    | Store     | Store[S] ← A |
| 100 | SUB S    | Subtract  | A ← A − Store[S] |
| 101 | SUB S    | (alternate) | A ← A − Store[S] (same as 100) |
| 110 | CMP      | Compare / Skip | if A < 0: CI ← CI + 1 |
| 111 | STP      | Stop      | halted ← True |

### Notes

**Two's complement throughout**: All arithmetic uses 32-bit two's complement.
Overflow wraps silently (modulo 2³²).

**CMP semantics**: The sign bit (bit 31) determines "negative". If A ≥ 0
(bit 31 = 0), CMP is a no-op. If A < 0 (bit 31 = 1), CI is incremented by
an extra 1, effectively skipping the next instruction.

**No ADD**: The SSEM has no ADD instruction. To add two values, negate one
with LDN and subtract the negated value with SUB: `A ← A − (−X) = A + X`.

**JMP vs JRP**: JMP is an *absolute* jump — it loads CI from memory. JRP is
a *relative* jump — it *adds* the memory value to CI (displacement).

---

## Execution Cycle

The real SSEM hardware used this cycle (important for timing):

```
1. CI ← CI + 1               (increment BEFORE fetch)
2. PI ← Store[CI]             (fetch present instruction)
3. Decode PI: S = PI[0:4], F = PI[13:15]
4. Execute instruction
```

CI starts at **31** (= −1 mod 32). The first increment brings it to 0, so
the first instruction executed is always **line 0**.

This "pre-increment then fetch" model has an important consequence for
**JMP**: setting `CI ← Store[S]` means the *next* fetch address will be
`Store[S] + 1` (after the next increment). To jump to line N, you must
store N − 1 at address S.

For **JRP**: at execute time CI has already been incremented. The operation
`CI ← CI + Store[S]` adds the displacement to the *already-incremented* CI,
so the next fetch is from `CI_current + Store[S] + 1`.

For **CMP skip**: the extra CI++ is applied after the normal increment, so
the skip advances by 2 total from the current CI.

---

## Signed Arithmetic

All store words and the accumulator are 32-bit two's complement:

- Values 0x00000000–0x7FFFFFFF are non-negative (0 to 2³¹−1)
- Values 0x80000000–0xFFFFFFFF are negative (−2³¹ to −1)
- Overflow wraps modulo 2³²

For **LDN S**: `A ← (−Store[S]) & 0xFFFFFFFF`  
For **SUB S**: `A ← (A − Store[S]) & 0xFFFFFFFF`

---

## Load Convention

Since the SSEM uses 32-bit words (not bytes), the `load()` method interprets
program bytes in **4-byte little-endian** chunks — each chunk becomes one
store word:

```python
word_N = program[4*N] | (program[4*N+1] << 8) | (program[4*N+2] << 16) | (program[4*N+3] << 24)
```

The `origin` parameter is in **word** units (0–31), not bytes.
A complete 32-word image is 128 bytes.

---

## SIM00 Protocol

The simulator implements the `Simulator[BabyState]` protocol from
`simulator_protocol`:

| Method | Behaviour |
|--------|-----------|
| `reset()` | CI ← 31; A ← 0; store ← all zeros; halted ← False |
| `load(program, origin=0)` | Decode bytes as 32-bit LE words; write to store[origin:] |
| `step()` | Execute one full cycle (increment CI, fetch, execute); return StepTrace |
| `execute(program, max_steps=10_000)` | reset + load + loop until STP or max_steps |
| `get_state()` | Return frozen BabyState snapshot |

No `set_input_port` / `get_output_port` — the SSEM has no I/O.

---

## State Snapshot

`BabyState` is a frozen dataclass:

```python
@dataclass(frozen=True)
class BabyState:
    store:       tuple[int, ...]  # 32 words (32-bit unsigned)
    accumulator: int              # 32-bit unsigned
    ci:          int              # 0–31 (word address of last executed instr)
    halted:      bool
```

Helper properties:
- `acc_signed`: accumulator as Python signed int (−2³¹ … 2³¹−1)
- `present_instruction`: store[ci]

---

## Test Coverage Targets

| Test file | Coverage areas |
|-----------|----------------|
| `test_protocol.py` | Construction, reset, load (byte→word), step, execute, get_state |
| `test_instructions.py` | Each of the 7 opcodes; edge cases (overflow, skip/no-skip) |
| `test_programs.py` | Multi-instruction programs: negate, sum, loop, count-down |

Target: ≥ 95% line coverage.

---

## Relation to Spec CPU-SIMULATOR-ROADMAP

This is Layer 07l in the alternating tik-tok sequence:

```
07k  Z80 (1976)          ← post-4004 IC
07l  Manchester Baby (1948) ← pre-4004 mainframe  ← this spec
07m  Intel 8086 (1978)   ← post-4004 IC (next)
07n  EDSAC (1949)        ← pre-4004 mainframe (after 07m)
```
