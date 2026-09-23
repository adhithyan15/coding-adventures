# Spec 07aa — DEC PDP-8/E Functional Simulator

**Architecture:** DEC PDP-8/E base processor

**Year:** 1970 (PDP-8 family ISA introduced in 1965)

**Rust package:** `pdp8-simulator`

## Sources and scope

The instruction formats and processor behavior follow Digital Equipment
Corporation's *PDP-8/E, PDP-8/M & PDP-8/F Small Computer Handbook* (1972).
The hardware boundary follows DEC's *PDP-8/E, PDP-8/F & PDP-8/M Processor
Maintenance Manual, Volume 1* (September 1973), particularly its processor,
memory-transfer, and I/O-pulse descriptions.

This first model is one base 4K-word memory field. It implements the base
processor, front-panel switch register, interrupt facility, CPU-control IOTs,
and external IOT bus events. Memory extension, peripherals, DMA/data break,
and the optional KE8-E Extended Arithmetic Element are separate hardware cells.
Group 3 instructions therefore fail with `UnsupportedEae` instead of receiving
invented behavior.

Primary references:

- https://www.grc.com/pdp-8/docs/PDP-8_Small_Computer_Handbook_1972.pdf
- https://bitsavers.org/pdf/dec/pdp8/pdp8e/DEC-8E-HMM1A-D-D_PDP-8e_Maintenance_Manual_Volume_1_Processor_Sep73.pdf

## Architectural state

| State | Width / count |
|---|---:|
| Core memory | 4,096 × 12-bit words |
| AC | 12 bits |
| Link | 1 bit |
| PC | 12 bits |
| MQ | 12 bits (reserved for the future EAE cell) |
| Switch register | 12 bits |
| Interrupt enable/request/delay | 1 bit each |
| Halt | 1 bit |
| Installed origin/length | lifecycle metadata |

All words and addresses are validated as 12-bit values. Program transport is
a sequence of little-endian two-byte containers; upper bits must be zero.

## Instruction surface

All six memory-reference operations are implemented: `AND`, `TAD`, `ISZ`,
`DCA`, `JMS`, and `JMP`. Effective addresses support zero page, the current
128-word page, direct and indirect access, and pre-increment through locations
0010–0017.

Group 1 supports `CLA`, `CLL`, `CMA`, `CML`, `IAC`, `RAR`, `RAL`, `RTR`, `RTL`,
and PDP-8/E `BSW` in documented micro-order. Conflicting simultaneous left and
right rotates fail atomically. Group 2 supports `SMA`, `SZA`, `SNL`, reverse
skip sense/`SKP`, `CLA`, `OSR`, and `HLT`.

CPU IOTs implement `SKON`, `ION`, `IOF`, `SRQ`, and `CAF`. `GTF`, `RTF`, and
`SGT` require the memory-extension/user-mode/greater-than flags not present in
this base cell and are typed unsupported operations. Nonzero device-select IOTs
retire and expose their device and pulse wires as an `IotEvent`; a future SoC
bus connects those events to peripheral state.

## Lifecycle and failure contract

Reset, strict load, state restoration, single-step, bounded run, and execute
are deterministic. State snapshots are deep and owned. Invalid transport,
addresses, words, OPR combinations, optional hardware instructions, and steps
after halt return typed errors. A failed step restores the entire pre-fetch
state. Bounded execution never allocates from an untrusted step limit.
