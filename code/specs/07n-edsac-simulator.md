# Spec 07n — EDSAC (1949) Functional Simulator

## Status

Normative contract for the Rust package `edsac-simulator`.

## Source boundary

This model follows the University of Cambridge Computer Laboratory's 1949
EDSAC order-code documentation and Martin Richards's Cambridge EDSAC
simulator. The primary references are:

- <https://www.cl.cam.ac.uk/relics/orders.html>
- <https://www.cl.cam.ac.uk/relics/orders.notes.html>
- <https://www.cl.cam.ac.uk/events/EDSAC99/simulators/echo/flat.html>
- <https://www.cl.cam.ac.uk/~mr10/Edsac.html>

It is an instruction-level model. Delay-line timing, Initial Orders tape
assembly, telephone-dial extensions, and analog valve behavior are outside
this layer. The later gate-level partner owns the digital timing boundary.

## Architectural state

The simulator owns the complete state needed to stop and resume execution:

- 1024 addressable short locations. Odd locations contain 17 bits. Even
  locations contain 17 data bits plus the otherwise unusable sandwich digit,
  so their raw container is 18 bits.
- a 10-bit sequence control register (`scr`), 17-bit order tank, 71-bit
  accumulator, and 35-bit multiplier and multiplicand registers;
- short/long indication for the current multiplicand;
- paper-tape input, its cursor, teleprinter output, and last output code;
- halt state and installed program origin/length.

The arithmetic registers use the Cambridge simulator's 15-bit tank slices:
five slices for the accumulator and three each for multiplier/multiplicand.
This representation makes the documented serial arithmetic, low-order
product bits, and unusual shifts reproducible without host floating point.

## Program transport and memory

`load` accepts little-endian 32-bit containers. `load_words` accepts the same
raw cells directly. A cell at an odd address must fit 17 bits; a cell at an
even address may fit 18 bits. Transport length, origin, capacity, and cell
width are validated before state changes. A successful load clears the whole
machine, installs the supplied cells, records the installed range, and sets
`scr` to the origin.

A long word is the even/odd pair selected by clearing the low address bit.
The odd cell is its 17-bit senior half; the even cell is its 18-bit junior
half including the sandwich digit. Direct reads and writes are checked.

Instruction fetch must remain inside the installed program range. Addresses
do not wrap. A terminal fall-through therefore becomes a typed fault on the
next step instead of silently returning to location zero.

## Instruction format

One order is 17 bits:

```text
16          12 11 10                         1 0
+--------------+--+---------------------------+-+
| 5-bit opcode |x |       10-bit address      |L|
+--------------+--+---------------------------+-+
```

Bit 11 is unused except as part of the historical shift pulse pattern. `L=0`
selects a short operand and `L=1` a long operand. The implemented character
codes are A, S, H, V, N, T, U, C, R, L, E, G, I, O, F, X, Y, and Z.

- A/S add or subtract the operand.
- H loads the multiplier; V/N add or subtract a product.
- T/U store the accumulator, with T clearing it.
- C collates multiplier AND operand and adds the result.
- R/L shift until the least-significant one pulse in the order is observed.
- E/G branch on nonnegative/negative accumulator sign.
- I reads a five-bit tape row into the low five bits of a short location.
- O emits the high five bits of a short location; F copies the last emitted
  code into the high five bits of a short location.
- X is the no-operation used by the Cambridge 1949 simulator, Y rounds at the
  35-bit boundary, and Z halts.

Orders for which the long discriminator is not meaningful reject `L=1`.
Unknown opcodes and malformed combinations never mutate state.

## Execution and failures

`step` returns a trace containing both sequence-control values, raw order,
mnemonic, operand address/length, and optional I/O code. Z retires normally
and sets halt. Stepping a halted machine is an error.

Every step is atomic. `run` and `execute_words` are transactional: decode,
input, range, halt, and step-limit failures restore the exact pre-call state.
All failures are typed, including malformed transport/cells, invalid state,
unknown or invalid orders, exhausted input, fetch outside the installed
program, halt, and maximum-step exhaustion.

## Acceptance

- Every order family and both applicable operand lengths have direct tests.
- Short/long packing, signed branching, serial shifts, multiplication,
  collation, round, I/O, state restoration, and lifecycle faults are covered.
- Debug and release tests, Clippy with warnings denied, rustdoc with warnings
  denied, the package BUILD recipe, and at least 80% line coverage pass.
