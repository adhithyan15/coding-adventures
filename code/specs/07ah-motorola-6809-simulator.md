# Layer 07ah — Motorola MC6809 Functional Simulator

## Status and primary sources

This layer defines the repository's checked instruction-level model of the
Motorola MC6809 introduced in 1978. It is RCPU-065 in the Rust CPU simulator
wave and is the behavioral reference for the separate RCPU-066 educational
gate-level partner.

The normative architecture sources are Motorola's 1981
*MC6809–MC6809E 8-Bit Microprocessor Programming Manual* (M6809PM/AD) and the
Motorola MC6809 data sheet. The opcode map, indexed-postbyte table, programming
model, flag equations, stack order, vectors, and big-endian byte order follow
those documents. Undocumented opcodes, cycle-exact bus timing, external
interrupt pins, DMA, peripherals, and later Hitachi 6309 extensions are outside
this CPU-only layer.

## Complete architectural state

`Motorola6809State` is an owned snapshot containing:

- accumulators A and B, exposed together as 16-bit accumulator D;
- index registers X and Y;
- user and hardware stack pointers U and S;
- program counter PC and direct-page register DP;
- all eight condition-code bits E, F, H, I, N, Z, V, and C;
- the complete 65,536-byte address space;
- deterministic wait/halt state; and
- installed-program origin and length.

All 16-bit memory values and instruction operands are most-significant byte
first. Native 16-bit addresses wrap at the bus boundary for architectural data
and stack accesses. Public memory access and state restoration are checked.

## Transport and lifecycle

Programs are raw MC6809 bytes. `load(program, origin)` rejects empty or
non-contiguous images before mutation, clears memory and registers, installs
the exact image, and sets PC to `origin`. Instruction fetch, prefixes,
postbytes, and operands must remain inside that image; data accesses may use
the whole address space.

`step` is atomic. `run(max_steps)` and `execute` are transactional as a whole.
Every successful step returns before/after PC, exact raw bytes, dynamic width,
an uppercase conventional mnemonic, and complete state after the transition.
Because every trace owns a complete 64 KiB snapshot, an execution retains at
most 1,024 traces; larger limits are rejected before allocation or mutation.

`SYNC` and `CWAI` enter deterministic wait state after their architectural
flag/stack effects. With no asynchronous device model in this layer, a later
step returns `Halted` without mutation. `SWI`, `SWI2`, and `SWI3` remain real
software interrupts and load vectors `$FFFA`, `$FFF4`, and `$FFF2`.

## Addressing and decoding

The simulator implements inherent, immediate, direct, extended, short/long
relative, and indexed modes. Direct addresses concatenate DP with the operand
byte. The indexed postbyte implements all documented forms over X/Y/U/S:

- signed 5-bit, 8-bit, and 16-bit constant offsets;
- A, B, and D accumulator offsets;
- no-offset, predecrement, and postincrement forms;
- 8-bit and 16-bit PC-relative forms;
- every documented indirect counterpart; and
- extended indirect `[nn]`.

Reserved postbytes fail atomically with `InvalidIndexedPostbyte`.

The complete documented opcode surface is required: every legal base-page,
page-2 (`$10` prefix), and page-3 (`$11` prefix) encoding in Motorola's
machine-code cross-reference. This includes:

- 8-bit arithmetic, comparison, logic, load/store, test, shifts and rotates;
- 16-bit D/X/Y/U/S arithmetic, comparisons, load/store, and multiply;
- LEA, EXG, TFR, ABX, SEX, and DAA;
- all short and long conditional branches, BSR/LBSR, JSR/JMP, and RTS;
- PSHS/PULS/PSHU/PULU with exact mask order;
- ORCC, ANDCC, NOP, SYNC, CWAI, RTI, SWI, SWI2, and SWI3; and
- direct, indexed, and extended read-modify-write memory forms.

Undefined and unused encodings return `UnknownOpcode` with the full prefixed
opcode and original PC.

## Flags, stacks, and interrupts

Flag effects follow the programming manual. In particular, subtraction sets C
for a borrow, 16-bit operations update N/Z/V/C without changing H, stores set
N/Z and clear V, LEAX/LEAY update Z, LEAS/LEAU do not, MUL sets Z from D and C
from product bit 7, and DAA uses the pre-adjust H/C state.

Both stacks grow downward. Push operations predecrement and pull operations
read then postincrement. Register-mask bit order follows the manual, with bit 6
selecting U on S-stack operations and S on U-stack operations. Full software
interrupt frames push PC, U, Y, X, DP, B, A, and CC and set E; RTI restores a
full frame only when the pulled E bit is set.

## Errors and acceptance

Typed errors distinguish empty/oversized images, fetch outside the installed
image, truncated instructions, unknown prefixed opcodes, reserved indexed
postbytes, invalid transfer register codes or widths, invalid restored state,
halted execution, excessive retention limits, and exhausted step budgets.
Every error has stable nonempty display text and every failing transition is
atomic.

Completion requires:

1. table-driven classification over all 256 base bytes and all documented
   page-2/page-3 encodings;
2. witnesses for every instruction family and addressing form, including every
   legal indexed-postbyte class and both outcomes of each conditional branch;
3. exact stack, transfer, software-interrupt, trace, and complete-state tests;
4. lifecycle and malformed-byte fuzz tests proving panic freedom and rollback;
5. line coverage above 80%; and
6. formatting, strict Clippy, rustdoc, release tests, BUILD integration,
   lessons validation, affected-package checks, and security review before
   publication.
