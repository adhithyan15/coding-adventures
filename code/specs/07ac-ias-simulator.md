# Spec 07ac — IAS Machine (1951) Functional Simulator

## Status

Normative contract for the Rust package `ias-simulator`.

## Sources and scope

The architecture follows the Institute for Advanced Study Electronic Computer
Project reports: Burks, Goldstine, and von Neumann's 1947 second edition of
*Preliminary Discussion of the Logical Design of an Electronic Computing
Instrument*, and the project's September 1951 *Sixth Interim Progress Report*.
The former defines the stored-program organization, 40-bit words, complement
arithmetic, 4,096-word address space, arithmetic registers, paired 20-bit
orders, and control transfers.  The latter is the implementation reference for
the arithmetic organ, Williams memory, and main control.

Primary references:

- https://www.ias.edu/sites/default/files/library/Prelim_Disc_Logical_Design.pdf
- https://www.ias.edu/sites/default/files/library/pdfs/ecp/sixthinterimprog06inst.pdf
- https://www.ias.edu/library/ecp

The 21-operation order table and opcode assignment are cross-checked against
the source-linked IASSim reference maintained by Colby College.  Opcode zero is
the stop order anticipated by section 6.8.5 of the IAS report; it is the only
extension to the numbered 1–21 table.

- https://www.cs.colby.edu/djskrien/IASSim/

This cell models the processor and electronic memory at instruction
boundaries.  Williams-tube timing, vacuum-tube networks, and physical control
pulses belong to the paired gate-level cell.  The original order set has no
program-visible input/output operations, so programs and data are installed
through the checked lifecycle API.

## Representation and transport

A memory word is 40 bits.  It is either a two's-complement fixed-point/integer
value or two 20-bit instructions.  Each instruction is an 8-bit opcode followed
by a 12-bit memory address.  The high instruction occupies word bits 39–20 and
executes before the low instruction in bits 19–0.

Binary transport uses exactly five big-endian bytes per word.  Misaligned
transport, values wider than 40 bits, origins outside memory, and loads whose
complete range does not fit fail before allocation or mutation.  Structured
helpers encode instructions and pair them into words without accepting values
outside the architectural widths.

## Architectural state

| State | Width / count |
|---|---:|
| Electronic memory | 4,096 × 40-bit words |
| Accumulator (`AC`) | 40 bits |
| Arithmetic / multiplier-quotient register (`AR`) | 40 bits |
| Control counter | 12 bits, plus the one-past sentinel while a right order is buffered |
| Buffered right order | absent or 20 bits |
| Current control register | 20 bits |
| Function table register | 8 bits |
| Memory address register | 12 bits |
| Selectron / memory buffer register | 40 bits |
| Halt | 1 bit |
| Installed origin and length | lifecycle metadata |

Fetching a left order reads one complete memory word, buffers its right order,
and advances the control counter to the next word.  The buffered order is then
executed without refetching memory.  This preserves the machine's observable
self-modifying-code boundary: a left order may alter the stored right order,
but the already-buffered right order remains unchanged.  A jump to a right
order fetches and buffers that target before the following step.

Snapshots own all memory and transient control state.  Restore validates every
word, register width, buffer relationship, loaded range, and control position
before replacing live state.

## Order set

Opcodes `1` through `8` load, negate, take absolute value, add, and subtract
memory operands.  Opcodes `9` and `10` move memory to `AR` and `AR` to `AC`.
Opcode `11` multiplies signed 40-bit operands into the 80-bit `AC:AR` pair.
Opcode `12` divides signed `AC` by a memory operand, placing the quotient in
`AR` and remainder in `AC`; zero divisors and an unrepresentable quotient are
typed faults.

Opcodes `13` and `14` jump to the left or right target order.  Opcodes `15` and
`16` do the same only when `AC` is non-negative; zero is positive.  Opcode `17`
stores `AC`.  Opcodes `18` and `19` replace only the 12-bit address field of the
left or right order in a target word with the low 12 bits of `AC`.  Opcodes
`20` and `21` shift `AC` left with zero fill or right with sign extension.
Opcode `0` halts.  Every other opcode is rejected.

Addition, subtraction, negation, and left shift wrap modulo `2^40`, matching
the report's complement accumulator.  Absolute-value operations therefore
leave the unique minimum negative value unchanged rather than inventing a
41st result bit.

## Lifecycle and failure contract

Construction, reset, strict load, direct memory access, checked state restore,
single-step, bounded run, and execute are deterministic.  Reset clears memory
and all registers.  Fetches and control transfers must remain within the
installed image; they never wrap the 12-bit address space.  A step after halt,
unknown opcode, invalid state, divide fault, or range failure returns a typed
error and restores the exact pre-step state, including memory and buffered
control.  A bounded run is transactional across the whole run and returns the
original state on fault or budget exhaustion.  Step limits are checked before
trace allocation.

Each successful trace records the control position and half, raw instruction,
opcode, address, mnemonic, pre/post `AC` and `AR`, and the complete post-step
state.  Run results include the trace sequence and final complete state.

## Completion evidence

- Every order family, both control halves, taken and untaken branches,
  self-modifying buffered behavior, arithmetic edge cases, direct boundaries,
  strict transport, restore validation, and typed atomic fault are tested.
- Debug and release tests, Clippy with warnings denied, rustdoc with warnings
  denied, the package BUILD recipe, and at least 80% line coverage pass.
