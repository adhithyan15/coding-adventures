# 07h — IBM 704 Simulator (Mainframe ISA)

## Overview

The IBM 704 simulator implements the instruction set of the **IBM 704 Electronic
Data-Processing Machine** (announced May 1954, first delivered December 1955) —
the first mass-produced computer with hardware floating-point arithmetic, the
first machine that **FORTRAN** was developed for, the first machine that **LISP**
ran on, and the source of the names `CAR` and `CDR` (which are literal IBM 704
instruction-field names: *Contents of the Address Register* and *Contents of
the Decrement Register*).

Where prior simulators in this repo (Intel 4004, 8008, ARM1, RISC-V, WASM) all
target microprocessors from 1971 onward, the 704 belongs to the era *before* a
CPU could fit on a chip. A 704 installation filled a room, drew 30 kW, and used
~4,000 vacuum tubes. But the **architecture** — the programmer-visible ISA — is
modest in size and well-documented, and a behavioral simulator can run real
period software.

This is a **behavioral simulator** that executes 704 machine words directly,
producing correct results without modeling vacuum-tube pulse circuits. It
conforms to `SIM00-simulator-protocol` so the same compiler-pipeline tests work
as for any other backend.

## Layer Position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → Lexer → Parser → Compiler → VM
```

This is an alternative Layer 4 sibling of the Intel 4004 (07d), 8008 (07f),
ARM1 (07e), RISC-V (07a), WASM (07c), and GE 225 (07g) simulators.

## Why the IBM 704?

Beyond being a famous machine, the 704 is the *single highest-leverage* ISA for
language resurrection in this repo. A working 704 simulator unlocks:

- **FORTRAN I (1957)** and **FORTRAN II (1958)** — the original FORTRAN
  compilers were written *for* the 704; the language's INTEGER and REAL types
  reflect 704 word semantics directly.
- **LISP 1 (1958)** and **LISP 1.5 (1962)** — McCarthy's group implemented LISP
  on the 704 at MIT. A cons cell *is* a 704 word, with CAR being the address
  field and CDR being the decrement field.
- **IPL-V (1958)**, an early list-processing language used in AI research.
- **COMIT (1957)**, an early symbolic-manipulation language.
- **Programs ported from the IBM 7090/7094** — the 7090 (1959) is essentially a
  transistorized 704 with the same ISA plus minor extensions; the 7094 (1962)
  adds double-precision FP and more index registers. A 704 simulator runs
  the common subset.

Hosting these languages means the repo can resurrect software older than the
Intel 4004 by 17 years, on a machine that *defined* the architecture/
implementation distinction that everything modern inherits.

## Architecture

| Feature | Value |
|---------|-------|
| Word size | 36 bits |
| Number representation | **Sign-magnitude** integers (1 sign bit + 35 magnitude bits) |
| Floating-point | Hardware FP: sign + 8-bit excess-128 exponent + 27-bit fraction |
| Accumulator (AC) | **38 bits**: S, Q, P, then bits 1–35. Q and P detect overflow. |
| Multiplier-Quotient (MQ) | 36 bits — used for multiply, divide, and shifts |
| Index registers | 3 × 15-bit: **IRA**, **IRB**, **IRC** (referenced by tag bits 1, 2, 4) |
| Memory | 4,096 / 8,192 / 32,768 words of magnetic-core (15-bit address) |
| Address space | 15 bits — 32K word maximum |
| I/O | Magnetic tape, card reader, line printer, drum (deferred to v2) |
| Console | Sense lights (4), sense switches (6), overflow trigger, divide-check trigger |
| Clock | 12 µs cycle time (~83 kHz effective, much slower than even the 4004) |

### Word Format

Every 36-bit word can be interpreted in three ways depending on context:

```
Integer interpretation (sign-magnitude):
  ┌─┬───────────────────────────────────────────┐
  │S│            35-bit magnitude               │
  └─┴───────────────────────────────────────────┘
   0  1                                       35

Floating-point interpretation:
  ┌─┬────────┬───────────────────────────────┐
  │S│  exp   │       27-bit fraction         │
  └─┴────────┴───────────────────────────────┘
   0 1      8 9                            35
  S = sign of fraction
  exp = excess-128 exponent (actual = exp - 128)
  fraction = magnitude in fixed-point binary, normalized so high bit is 1

Instruction interpretation (Type B):
  ┌─┬──────────────┬─────┬─────────────────┐
  │S│ 11-bit op    │ tag │  15-bit address │
  └─┴──────────────┴─────┴─────────────────┘
   0 1           11 12 14 15              35

Instruction interpretation (Type A):
  ┌──┬─────────────────┬─────┬─────────────────┐
  │op│   15-bit decr   │ tag │  15-bit address │
  └──┴─────────────────┴─────┴─────────────────┘
   0 2 3              17 18 20 21             35
  op = 3-bit prefix opcode (1, 2, 3, or 5)
```

Note: bit numbering follows IBM's convention where **bit S is bit 0** at the
left and the rightmost bit is bit 35. This is the *opposite* of modern little-
endian bit numbering; we follow IBM's convention throughout the simulator and
spec to match the original Reference Manual.

### Sign-Magnitude — Why It Matters

The 704 stores integers as **sign + magnitude**, not two's complement:

- `+3` is `0 ... 011`
- `-3` is `1 ... 011`
- `+0` and `-0` are *both representable* as distinct words (this is observable
  via the TZE/TNZ instructions which test "is the magnitude zero" — they
  match either +0 or -0, so the two zeros are equal under TZE).

Addition is more involved than two's complement:

- If signs match: add magnitudes, keep the common sign. Overflow if the result
  exceeds 35 bits.
- If signs differ: subtract the smaller magnitude from the larger; the result
  takes the sign of the larger operand.
- The hardware uses the AC's Q and P bits to detect overflow and rounding.

FORTRAN INTEGER variables on a 704 are 36-bit sign-magnitude. A FORTRAN program
that assumes two's-complement bit-twiddling (which would not exist for years)
will not behave the same way it does on a modern machine — but FORTRAN I had no
bit-twiddling features, so this rarely surfaces in practice.

### Accumulator Layout (38 bits, not 36)

```
  ┌─┬─┬─┬───────────────────────────────────────────┐
  │S│Q│P│            35-bit magnitude               │
  └─┴─┴─┴───────────────────────────────────────────┘
   0  1 2 3                                       37
```

- **S** — sign bit
- **Q** — overflow detection: set when an addition carries into bit Q
- **P** — overflow detection: set when an addition carries into bit P. **If P
  is set after an arithmetic operation, the overflow trigger lights up.**
- **Bits 3–37** — 35-bit magnitude

This design lets the AC hold the result of an addition or shift even when the
result would not fit in 35 bits, so software can detect and correct overflow.
Most programs check the overflow trigger via TOV (transfer on overflow) and
either propagate, scale, or signal an error.

### MQ Register (Multiplier-Quotient)

The MQ is 36 bits, sign-magnitude. It participates in:

- **MPY** — multiply: AC × MQ → 70-bit product into AC and MQ (high half in AC,
  low half in MQ).
- **DVP** — divide: AC,MQ ÷ memory → quotient in MQ, remainder in AC.
- **LRS / LLS / ARS / ALS** — long shifts that cross AC↔MQ boundary.

### Index Registers

The 704 has **3 index registers** named IRA, IRB, IRC (later 704 manuals call
them XR1, XR2, XR4 because the *tag* field used to select them is bit-encoded:
tag bit 0 = IRA, tag bit 1 = IRB, tag bit 2 = IRC). The 7094 (1962) extended
this to 7 index registers; this simulator implements the original 3.

When a tag of 1, 2, or 4 appears in an instruction, the **15-bit address
field is decremented by the contents of the selected index register before
the operation**. This is the "indexed addressing" mechanism.

If multiple tag bits are set simultaneously (tag=3, 5, 6, or 7), the *logical
OR* of the corresponding index registers is subtracted (the original 704
manual is explicit that this is well-defined behavior). The simulator
implements this faithfully.

### Memory

- 32,768 36-bit words of core memory (the maximum 704 configuration)
- 15-bit address space, addresses 0–32767
- Memory is word-addressed, *not* byte-addressable — there is no concept of a
  byte on the 704. Characters are stored 6 per word as 6-bit BCD codes when
  needed, but no instruction directly addresses sub-word fields except through
  the accumulator's shift instructions.

The simulator initialises memory to all zeros on reset.

## Instruction Set (v1 Scope)

The full 704 ISA has ~90 instructions. The v1 simulator implements **the
core ~40** that suffice to run FORTRAN-style integer programs and verify
floating-point arithmetic. Deferred to v2: I/O (RDS, WRS, BSR, BSF, REW, etc.),
BCD character manipulation (CVR, CRQ), the full shift family, sense lights and
switches, and the more obscure transfer/test instructions.

### Instruction Format Decoding

Most instructions are **Type B** (op | tag | address). The decoder reads bits 0–11
as opcode and dispatches:

```
opcode (12 bits)  tag (3 bits)  address (15 bits)
```

Type A instructions (TIX, TXI, TNX, TXH, TXL) use a different layout:

```
prefix (3 bits)  decrement (15 bits)  tag (3 bits)  address (15 bits)
```

The decoder distinguishes by inspecting bits 0–2: if they are `001`, `010`,
`011`, or `101`, the instruction is Type A; otherwise Type B.

### Effective Address Computation

For most Type B instructions with a tag T:

```
effective_address = (Y - C(T)) & 0x7FFF
```

where `Y` is the 15-bit address field and `C(T)` is the contents of the index
register (or OR of multiple registers if tag has multiple bits set). When tag
is 0, `effective_address = Y`. The `& 0x7FFF` masks to 15 bits — index-register
underflow wraps cleanly.

**Important exception — index-register family:** For the index-register
instructions (LXA, LXD, SXA, SXD, PAX, PDX, PXA) the tag is interpreted
*only* as the register selector. The address field is used directly with
**no** subtraction by the contents of the tagged register. This is required
behaviour: a "store IRA at address Y" instruction whose destination shifted
with IRA's value would be useless.

For the Type A family (TIX, TXI, TXH, TXL) the address field is also used
directly as a transfer target — these are loop-control instructions whose
destination is a labeled jump target, not a data address to be indexed.

### Core Instructions (v1)

#### Machine Control
| Mnemonic | Opcode (octal) | Description |
|----------|----------------|-------------|
| HTR Y    | 0000           | Halt and Transfer — stop execution; PC = effective address. |
| HPR Y    | +420           | Halt and Proceed — stop; resumable, PC = effective address. |
| NOP      | +0761          | No operation. |

#### Loads and Stores
| Mnemonic | Opcode (octal) | Description |
|----------|----------------|-------------|
| CLA Y    | +0500          | Clear and Add — AC = M[Y] (bits 1–35 + sign). Q and P cleared. |
| CAL Y    | -0500          | Clear and Add Logical — AC = M[Y] treating word as logical (no sign extension). |
| ADD Y    | +0400          | Add — AC = AC + M[Y], sign-magnitude rules. |
| SUB Y    | +0402          | Subtract — AC = AC − M[Y]. |
| ADM Y    | +0401          | Add Magnitude — AC = AC + |M[Y]| (treat operand as positive). |
| STO Y    | +0601          | Store — M[Y] = AC bits S,1–35 (Q and P discarded). |
| STZ Y    | +0600          | Store Zero — M[Y] = 0. |
| STQ Y    | -0600          | Store MQ — M[Y] = MQ. |
| LDQ Y    | +0560          | Load MQ — MQ = M[Y]. |
| XCA      | +0131          | Exchange AC and MQ. |

#### Integer Arithmetic
| Mnemonic | Opcode (octal) | Description |
|----------|----------------|-------------|
| MPY Y    | +0200          | Multiply — AC,MQ = MQ × M[Y]. AC gets high 35 bits, MQ low 35. |
| DVP Y    | +0221          | Divide or Proceed — quotient → MQ, remainder → AC. |
| DVH Y    | +0220          | Divide or Halt — same as DVP but halt on divide check. |

#### Transfer Instructions
| Mnemonic | Opcode (octal) | Description |
|----------|----------------|-------------|
| TRA Y    | +0020          | Transfer — PC = effective address. |
| TZE Y    | +0100          | Transfer on Zero — if AC magnitude == 0, PC = eff addr. |
| TNZ Y    | -0100          | Transfer on Non-Zero — if AC magnitude != 0, PC = eff addr. |
| TPL Y    | +0120          | Transfer on Plus — if AC sign == 0, PC = eff addr. |
| TMI Y    | -0120          | Transfer on Minus — if AC sign == 1, PC = eff addr. |
| TOV Y    | +0140          | Transfer on Overflow — if overflow trigger set, transfer and clear it. |
| TNO Y    | -0140          | Transfer on No Overflow — transfer if NOT overflowed. |
| TQO Y    | +0161          | Transfer on MQ Overflow — clears MQ-overflow flag. |
| TQP Y    | +0162          | Transfer on MQ Plus. |

#### Index Register Instructions
| Mnemonic | Opcode (octal) | Description |
|----------|----------------|-------------|
| LXA Y,T  | +0534          | Load Index from Address — IR(T) = address bits of M[Y]. |
| LXD Y,T  | -0534          | Load Index from Decrement — IR(T) = decrement bits of M[Y]. |
| SXA Y,T  | +0634          | Store Index in Address — M[Y].address = IR(T). |
| SXD Y,T  | -0634          | Store Index in Decrement — M[Y].decrement = IR(T). |
| PAX 0,T  | +0734          | Place Address in Index — IR(T) = AC.address (bits 21–35). |
| PDX 0,T  | -0734          | Place Decrement in Index — IR(T) = AC.decrement (bits 3–17). |
| PXA 0,T  | +0754          | Place Index in Address — AC = IR(T) in address position; sign and other fields cleared. |
| TIX Y,T,D | 2 (Type A)    | Transfer on Index — if IR(T) > D, IR(T) -= D; PC = Y. Else fall through. |
| TXI Y,T,D | 1 (Type A)    | Transfer with Index Incremented — IR(T) += D; PC = Y. (Always transfers.) |
| TXH Y,T,D | 3 (Type A)    | Transfer on Index High — if IR(T) > D, PC = Y. |
| TXL Y,T,D | -3 (Type A)   | Transfer on Index Low or Equal — if IR(T) ≤ D, PC = Y. |

#### Floating-Point (v1: 4 ops)
| Mnemonic | Opcode (octal) | Description |
|----------|----------------|-------------|
| FAD Y    | +0300          | Floating Add — AC,MQ = AC + M[Y] in floating-point. |
| FSB Y    | +0302          | Floating Subtract — AC,MQ = AC − M[Y]. |
| FMP Y    | +0260          | Floating Multiply — AC,MQ = MQ × M[Y]. |
| FDP Y    | +0240          | Floating Divide or Proceed — quotient in MQ, remainder in AC. |

Floating-point format: sign + 8-bit excess-128 exponent + 27-bit fraction.
The fraction is normalized so its high bit is 1 (after operations the result
is renormalized).

### Deferred to v2

- I/O instructions (RDS, WRS, BSR, BSF, REW, RTB, RCD, etc.)
- Sense light / switch instructions (SLT, SLN, SWT, SLF)
- The full shift family (LRS, LLS, ARS, ALS, LGR, LGL, RQL)
- BCD character manipulation (CVR, CRQ, ORA, ORS, ANA, ANS, ERA)
- Magnitude operations beyond ADM (SSP, SSM, CHS, CLS)
- The remaining transfer instructions (TLQ, TQP, TPL variants)
- Round and floating-point reciprocal (RND, FRN, UFA, UFS, UFM, UFDP)
- Programmed interrupts and the trap mechanism

These are all in scope for follow-up PRs but are not required for hosting
FORTRAN-style numeric programs or LISP cons-cell manipulation. They will be
added as the language frontends targeting this simulator demand them.

## SIM00 Conformance

The simulator exposes the standard `Simulator[IBM704State]` interface:

```python
class IBM704Simulator:
    def load(self, program: bytes) -> None: ...
    def step(self) -> StepTrace: ...
    def execute(
        self, program: bytes, max_steps: int = 100_000
    ) -> ExecutionResult[IBM704State]: ...
    def get_state(self) -> IBM704State: ...
    def reset(self) -> None: ...
```

### Program Encoding

The 704 has no native byte order — it operates on whole 36-bit words. To fit
the byte-input `Simulator` protocol, programs are loaded as a sequence of
**packed 5-byte big-endian words**, each holding one 36-bit instruction in the
low 36 bits of the 40-bit big-endian integer (the high 4 bits are zero):

```
byte 0  byte 1  byte 2  byte 3  byte 4
[xxxxXXXX][XXXXXXXX][XXXXXXXX][XXXXXXXX][XXXXXXXX]
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
       36 bits of instruction word, MSB first
```

The high 4 bits of byte 0 are reserved (must be zero). This encoding is purely
a transport detail for `load(program: bytes)`; once loaded, instructions live
in the simulator's native 36-bit memory array.

`load()` writes words starting at memory address 0. A program of N instructions
occupies bytes [0, 5*N) of the input.

### IBM704State Frozen Snapshot

```python
@dataclass(frozen=True)
class IBM704State:
    accumulator_sign: bool        # True if AC sign bit is 1
    accumulator_qp: int           # Q and P overflow bits, 0–3
    accumulator_magnitude: int    # bits 3–37, 0 to 2^35 - 1
    mq: int                       # full 36-bit MQ (sign in bit 35 of int)
    mq_sign: bool                 # MQ sign separately for clarity
    mq_magnitude: int             # 0 to 2^35 - 1
    index_a: int                  # IRA (15-bit)
    index_b: int                  # IRB (15-bit)
    index_c: int                  # IRC (15-bit)
    pc: int                       # 0–32767
    halted: bool
    overflow_trigger: bool
    divide_check_trigger: bool
    memory: tuple[int, ...]       # 32768 words, each 0 to 2^36 - 1
```

All mutable fields are converted to immutable types per the SIM00 immutability
rule. Memory is stored as a tuple of 32768 plain ints — each int is the raw
36-bit word value (sign in bit 35).

### StepTrace

The simulator emits a standard `StepTrace` per instruction with:

- `pc_before` — address where the instruction was fetched
- `pc_after` — address of the next instruction
- `mnemonic` — e.g., `"CLA 0x100"`, `"TIX 50,1,3"`
- `description` — e.g., `"CLA Y=256 → AC=12345"`

## Example Programs

### Sum the integers 1..N (FORTRAN-style loop)

```
        CLA  N         ; AC = N
        STO  CTR       ; CTR = N (counter, in memory)
        STZ  SUM       ; SUM = 0
LOOP:   CLA  SUM       ; AC = SUM
        ADD  CTR       ; AC = SUM + CTR
        STO  SUM       ; SUM = AC
        TIX  LOOP,1,1  ; IRA -= 1; if IRA > 0 goto LOOP
        HTR  EXIT      ; halt
EXIT:   HTR  EXIT
```

(Setting up IRA before the loop with LXA is omitted for brevity; the spec's
`tests/` directory has the complete program.)

### LISP-style cons cell access

A cons cell is a single 36-bit word with `car` in the address field (bits 21–35)
and `cdr` in the decrement field (bits 3–17). To extract `car`:

```
        CLA  CELL      ; AC = whole cons cell word
        PAX  0,1       ; IRA = address field (the car!)
        PXA  0,1       ; AC = IRA, padded with zeros
        STO  RESULT    ; RESULT = car of CELL
```

This is *literally* where the names CAR and CDR come from. The instructions
PAX and PDX extract those exact bit fields.

### Floating-point: y = a*x + b

```
        LDQ  A         ; MQ = a
        FMP  X         ; AC,MQ = a * x
        FAD  B         ; AC,MQ = a*x + b
        STO  Y         ; Y = high half of result
```

## Test Strategy

### Unit tests

- Word-format encode/decode round-trips
- Sign-magnitude addition: same-sign, mixed-sign, with and without overflow
- Each instruction tested in isolation with set state → step → assert state
- Effective-address computation with each tag value (0 through 7)
- Edge cases: AC overflow sets P, AC magnitude wrap with Q, MQ overflow

### State immutability

- `get_state()` returns a frozen dataclass; mutation raises `FrozenInstanceError`
- A snapshot taken before further execution remains unchanged

### End-to-end programs

- Sum 1..N using TIX
- Factorial using MPY and TIX
- Cons-cell CAR/CDR extraction (LISP-style)
- Floating-point polynomial evaluation
- Sign-magnitude special cases: +0 vs -0 transfer behavior

### Protocol conformance

- `IBM704Simulator` satisfies `Simulator[IBM704State]` structurally
- `execute()` returns `ExecutionResult[IBM704State]` with the documented
  semantics for `ok`, `halted`, `error`

### Cross-language consistency (future)

When IBM 704 simulators land in additional languages (Go, Ruby, etc.), the same
test programs must produce identical final state. This mirrors the pattern set
by the Intel 4004 simulator suite.

## Historical Context

### The machine that taught us how to compute

The IBM 704 was announced in May 1954 as IBM's first scientific computer with
hardware floating-point. It cost roughly $2 million in 1954 dollars (about $24
million today) and IBM built 123 of them. For five years it was the highest-
performance computer commercially available; CDC's STRETCH (1961) and the IBM
7090 (1959) eventually replaced it.

### FORTRAN was *for* this machine

John Backus's team at IBM started the FORTRAN project in 1954 explicitly to
make the 704 programmable by scientists who did not want to write assembly.
The first FORTRAN compiler shipped with the 704 in April 1957. FORTRAN's
INTEGER type *is* a 36-bit sign-magnitude word; its REAL type *is* the 704's
floating-point format. The language's eccentricities — 6-character identifiers
(one BCD character per 6-bit field, six fields per word), arithmetic IF (TZE/
TPL/TMI directly), DO loops mapped to TIX — all fall out of 704 hardware.

### LISP was *born* on this machine

John McCarthy's group at MIT implemented the first LISP interpreter on the 704
in 1958. CAR and CDR — the names every Lisp programmer knows — are the
*literal IBM 704 hardware-instruction-field names*: "Contents of the Address
Register" and "Contents of the Decrement Register". A 704 word, with its
15-bit address and 15-bit decrement fields, was exactly the right shape to
hold a pair of pointers — the cons cell.

### Mainframe simulation — where this fits

The Computer History Museum maintains the only operational IBM 704 in the
world (donated by Don Knuth and refurbished). For everyone else, simulation is
the only way to run period software. The SIMH project includes a 7090
simulator capable of booting period IBSYS images. This package is more
focused — its purpose is to be a *target* for the language frontends in this
repo, not a complete operating-system emulator. But the ISA is the same one
the world's first FORTRAN and LISP programs ran on.

## References

- *IBM 704 Electronic Data-Processing Machine: Manual of Operation* (1955).
  [Bitsavers PDF](http://bitsavers.org/pdf/ibm/704/24-6661-2_704_Manual_1955.pdf)
- *Reference Manual: 7090 Data Processing System* (1962). 7090 is binary-
  compatible with the 704 for the core ISA.
- McCarthy, John. *History of LISP* (1979). Sec. 3 explains CAR/CDR origin.
- Backus, John. *The History of FORTRAN I, II, and III* (1978). HOPL I.
- Knuth, Donald. *The Art of Computer Programming, Vol. 1* — uses MIX, but the
  preface acknowledges the 704 as the model machine for the era.
