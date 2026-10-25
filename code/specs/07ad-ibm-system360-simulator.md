# Layer 07ad — IBM System/360 Functional Simulator

## Status

This specification defines the repository's checked, instruction-level IBM
System/360 CPU model. It is the behavioral reference for a later gate-level
partner. Timing, channel programs, devices, storage protection, interruption
delivery, privileged control, and model-specific microarchitecture are outside
this layer.

The architectural source is IBM's *System/360 Principles of Operation*, form
GA22-6821-8 (November 1970). In particular, the simulator follows its
big-endian byte ordering, 24-bit addresses, instruction formats, effective
address rules, fixed-point condition codes, and hexadecimal floating-point
representation.

## Architectural state

`System360State` is a complete, owned snapshot containing:

- sixteen 32-bit general registers;
- four 64-bit floating-point registers, architecturally numbered 0, 2, 4, 6;
- a 24-bit instruction address and two-bit condition code;
- every byte of installed big-endian storage;
- halt state; and
- the installed program origin and byte length.

The simulator installs 1 MiB of storage. Every generated address remains a
24-bit System/360 address, but an address outside installed storage is a typed
fault. Register zero participates normally as an operand. In an address field,
an index or base number of zero contributes no address component.

## Program transport and lifecycle

Programs are raw System/360 instruction bytes. `load(program, origin)` rejects
an empty image, an odd origin, or a range outside installed storage before it
changes state. A successful load resets registers, condition code, storage,
and halt state, copies the complete image, records its exact installed range,
and selects `origin` as the instruction address.

Instruction fetch must begin on a halfword boundary and within the installed
program. The first opcode byte determines a two-, four-, or six-byte format;
the complete instruction must remain within the installed program. Fetching
uninstalled bytes is not permitted merely because storage exists there.

`step`, `run`, `execute`, direct state restoration, and memory/register helpers
are checked operations. A failed checked operation restores the exact prior
architectural state. `run` and `execute` are transactional as a whole and use
a caller-provided bounded step count.

System/360 has no unprivileged HALT order. For repository programs, `SVC 0`
(`0A00`) is the deterministic halt sentinel. Other SVC numbers are typed
unsupported-supervisor-call faults.

## Instruction formats

The first opcode byte selects one of the historical formats:

- **RR** (2 bytes): opcode, R1/M1, R2;
- **RX** (4 bytes): opcode, R1/M1, X2, B2, D2;
- **RS** (4 bytes): opcode, R1, R3, B2, D2;
- **SI** (4 bytes): opcode, I2, B1, D1; and
- **SS** (6 bytes): opcode, length byte, B1/D1, B2/D2.

RX effective addresses are `(X2 == 0 ? 0 : GPR[X2]) + (B2 == 0 ? 0 :
GPR[B2]) + D2`, truncated to 24 bits. RS, SI, and SS base/displacement
addresses omit the index term. Halfword, fullword, and doubleword operands
must satisfy their architectural alignment.

## Required instruction surface

The functional core implements the non-privileged standard families below.
RR and RX variants share arithmetic semantics.

### Fixed point and data movement

- `LR`, `L`, `LH`, `LTR`, `LCR`, `LPR`, `LNR`, `LM`;
- `ST`, `STH`, `STM`, `STC`, `IC`, `LA`;
- `AR/A`, `AH`, `ALR/AL`, `SR/S`, `SH`, `SLR/SL`;
- `CR/C`, `CH`, `CLR/CL`;
- `MR/M`, `MH`, `DR/D`; and
- `SLA`, `SRA`, `SLDA`, `SRDA`, `SLL`, `SRL`, `SLDL`, `SRDL`.

Signed add, subtract, sign-control, and algebraic-left-shift overflow returns a
typed atomic fault; the repository's checked lifecycle therefore restores the
pre-instruction result register and condition code rather than exposing the
interrupted hardware's transient result.
Divide-by-zero, an odd register-pair selector, or an unrepresentable quotient
is a typed atomic fault.

### Logical, character, and storage-to-storage

- `NR/N`, `OR/O`, `XR/X`;
- `MVI`, `MVC`, `CLI`, `CLC`;
- `NI/NC`, `OI/OC`, `XI/XC`; and
- `TM`.

SS fields are processed from left to right, one byte at a time, including
overlapping `MVC`. The SS length byte means `length - 1`. Logical results set
condition code 0 for zero and 1 for nonzero. Comparisons set 0 equal, 1 low,
and 2 high using signed or unsigned ordering as defined by the instruction.

### Branching

- `BCR/BC`, `BALR/BAL`, `BCTR/BCT`; and
- `BXH`, `BXLE`.

The branch mask bits 8, 4, 2, 1 correspond to condition codes 0, 1, 2, 3.
Branch-and-link stores the updated 24-bit instruction address in the target
general register. Branch-on-count decrements before testing. Index branches
perform the documented add-and-compare sequence.

### Hexadecimal floating point

Four 64-bit registers preserve exact raw long-format values. Short operations
use their high 32 bits and preserve the low half. The required instructions
are long/short load and store, load-and-test/sign control, normalized add and
subtract, compare, halve, multiply, and divide in RR and RX forms.

The representation is sign plus a seven-bit excess-64 hexadecimal exponent
and a 24- or 56-bit fraction. Arithmetic uses integer fractions and radix-16
normalization, never host binary floating point. Divide by a zero fraction and
an exponent outside the representable characteristic are typed atomic faults.

The optional decimal feature, extended precision, and unnormalized floating
add/subtract are reserved for a later extension rather than approximated.

## Traces and errors

Every successful step returns the address before and after execution, exact
raw instruction bytes, instruction width, conventional uppercase mnemonic,
and a concise description. Halting returns an `SVC 0` trace and marks the
machine halted; a later step returns `Halted` without mutation.

Errors distinguish empty/oversized/misaligned programs, fetch outside the
installed image, truncated instructions, unknown opcodes, unsupported SVC,
misaligned or out-of-range storage, invalid registers/state, specification
faults, integer overflow/divide, floating exponent/divide, halt, and exhausted
step limits. Every error has stable nonempty display text.

## Acceptance criteria

- The specification precedes tests and implementation.
- Tests cover every instruction family, every format, both branch outcomes,
  overlapping storage movement, integer and hexadecimal-floating boundaries,
  transport, complete-state restore, bounded execution, and atomic faults.
- Line coverage exceeds 95% where practical and never falls below 80%.
- Formatting, strict Clippy, rustdoc, package BUILD, release tests, lessons
  validation, and the repository workspace build are attempted before review.
