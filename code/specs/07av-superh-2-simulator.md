# Layer 07av — Renesas SuperH-2 Functional Simulator

## 1. Status and primary source

This layer defines a checked instruction-level model of a deliberately bounded
SuperH-2 (SH-2) integer processor. Its normative source is Renesas Technology's
*SH-1/SH-2/SH-DSP Software Manual*, revision 5.00, document
REJ09B0171-0500O (June 30, 2004). Register meanings, 16-bit instruction
encodings, operand ordering, sign extension, delayed branches, status-bit
effects, and multiply/divide behavior come from that manual rather than from a
console emulator or a later SH-2A derivative.

The package is an architectural teaching oracle for the coherent SH-2 subset
below. It is not a claim to reproduce a particular Renesas microcontroller,
Sega system, pipeline, cache, bus, interrupt controller, or cycle timing.

## 2. Bounded architecture

The simulator owns sixteen 32-bit general registers `R0` through `R15`, `PC`,
procedure register `PR`, global-base register `GBR`, vector-base register
`VBR`, multiply-accumulate registers `MACH` and `MACL`, and the architecturally
visible `T`, `S`, `Q`, `M`, and interrupt-mask fields of `SR`. The complete
state also owns a pending delayed-transfer record, deterministic sleep/halt
state, installed-program origin and length, and every byte of a flat 64 KiB
teaching memory.

Instructions are fixed-width 16-bit words. This first configuration pins both
instruction and data transport to the manual's big-endian byte ordering while
preserving 32-bit wrapping address arithmetic in registers. Every actual
fetch or data transfer must fit the owned memory without wrapping. A later
configuration cell may add little-endian transport; this cell must never infer
endianness from host layout.

The included instruction families are:

- register and immediate moves, `MOVT`, `MOVA`, `SWAP.B`, `SWAP.W`, and
  `XTRCT`;
- byte, word, and long loads/stores for register-indirect, pre-decrement,
  post-increment, R0-indexed, displacement, PC-relative, and GBR-relative
  forms admitted by the manual;
- `ADD`, `ADDC`, `ADDV`, `SUB`, `SUBC`, `SUBV`, `NEG`, `NEGC`, `DT`,
  sign/zero extension, and immediate add;
- `AND`, `OR`, `XOR`, `NOT`, and `TST`, including the documented immediate
  and GBR-byte forms;
- `CMP/EQ`, `CMP/HS`, `CMP/GE`, `CMP/HI`, `CMP/GT`, `CMP/PZ`, `CMP/PL`,
  `CMP/STR`, and immediate compare;
- logical/arithmetic shifts and rotates, with and without transfer through T;
- `MULU.W`, `MULS.W`, `DMULU.L`, `DMULS.L`, `MAC.W`, `MAC.L`, `DIV0U`,
  `DIV0S`, and `DIV1` with exact `MACH`, `MACL`, `M`, `Q`, `S`, and `T`
  behavior;
- `CLRT`, `SETT`, `CLRMAC`, and transfers to/from `MACH`, `MACL`, `PR`, and
  `GBR`; and
- `BT`, `BF`, `BT/S`, `BF/S`, `BRA`, `BSR`, `BRAF`, `BSRF`, `JMP`, `JSR`,
  `RTS`, `NOP`, and `SLEEP`.

Encodings outside those families, reserved neighbors, and SH-2A/SH-DSP
extensions must fail as typed unsupported or unknown instructions rather than
aliasing an included operation.

## 3. Integer, memory, and multiply/divide rules

All general-register arithmetic wraps at 32 bits. Immediate byte operands are
sign-extended where the manual specifies signed arithmetic and zero-extended
for logical immediates. `ADDC`, `SUBC`, `NEGC`, overflow instructions,
compares, shifts, rotates, `DT`, and test instructions update T exactly as
specified; instructions without a documented SR result preserve every SR
field.

Byte loads sign-extend, word loads sign-extend, and long loads preserve all
bits. Stores use the low 8, 16, or 32 bits. Word and long accesses must be
naturally aligned. Post-increment does not increment when source and
destination are the same register; pre-decrement computes the address before
the store. All addressing forms compute a full 32-bit architectural address
before checked conversion to owned memory.

PC-relative word addresses use the instruction's aligned PC basis; PC-relative
long addresses use the required four-byte-aligned basis. `MOVA` uses the same
four-byte-aligned basis and writes R0. GBR-relative byte, word, and long
displacements are scaled by one, two, and four respectively. Indexed forms use
R0 plus the named base register.

Unsigned and signed word multiplication write MACL. Double-word multiplication
writes the exact 64-bit product across MACH:MACL. Multiply-accumulate uses
checked source reads before mutation, updates address registers only after all
reads succeed, and applies the manual's S-controlled saturation boundary.
`DIV0U`, `DIV0S`, and each `DIV1` step expose the documented M/Q/T state so a
software division sequence is reproducible instruction by instruction.

## 4. Delayed control flow

Ordinary execution advances PC by two. `BRA`, `BSR`, `BRAF`, `BSRF`, `JMP`,
`JSR`, and `RTS` execute exactly one instruction at the sequential address as
a delay slot before committing their target. `BT/S` and `BF/S` use a delay slot
only when taken; plain `BT` and `BF` have no delay slot. `BSR`, `BSRF`, and
`JSR` write the address following the delay slot to PR.

Branch displacements are sign-extended and scaled by two from the manual's
architectural PC basis. Register targets must be half-word aligned. A target
is validated when it becomes the next fetch; this preserves the architectural
delay-slot effect while keeping faults atomic.

A control-transfer instruction, `SLEEP`, or any instruction the manual marks
illegal in a live delay slot returns `IllegalDelaySlot` without mutation. A
fault in a delay-slot instruction rolls back both that instruction and the
pending transfer. Successful traces identify whether the instruction was a
delay slot and expose the pending target in the complete post-state.

`SLEEP` emits a successful trace and enters deterministic halt because external
interrupt wake-up is outside this cell. A later `step` returns `Halted` without
mutation. Reset or a successful load clears halt.

## 5. Transport, traces, and transactional lifecycle

`load(program, origin)` accepts a nonempty, even-length, half-word-aligned,
contiguous big-endian image that fits memory. A successful load replaces the
entire state. Fetch must be half-word aligned and wholly inside that exact
image; terminal fetches do not wrap. `restore` validates memory size, SR field
bounds, image metadata, PC alignment, halt state, and pending-transfer
invariants before committing.

Every successful `step` returns the PC before and after the transition, exact
two fetched bytes and decoded word, stable conventional uppercase mnemonic,
delay-slot metadata, and a complete post-step snapshot. `step` is atomic.
`run(max_steps)` and `execute(program, origin, max_steps)` are transactional as
a whole: any typed fault or exhausted budget restores every register, SR bit,
memory byte, pending transfer, halt bit, and image field. Trace retention is
preflighted and bounded to 64 complete-state traces.

Typed errors distinguish invalid programs and restored states, misaligned and
out-of-range instruction or data accesses, fetch outside the installed image,
unknown and unsupported instructions, illegal delay slots, halted execution,
excessive trace retention, allocation failure, and exhausted step budgets.
Every error has stable nonempty display text.

## 6. Explicit exclusions and follow-up backlog

This cell excludes privileged SR/VBR loads, `RTE`, traps and exception entry,
interrupt arbitration, reset vectors, on-chip peripheral registers, MMU or
protection behavior, caches, external buses, DMA, timers, serial devices,
watchdogs, debug mode, implementation-specific power states, pipeline timing,
and cycle accuracy. `TAS.B` and its interlocked bus semantics are also deferred
until a memory-ordering cell can state them honestly.

FPU, SH-DSP, SH-2A, SH-3, and SH-4 instructions and state are separate
architectures. Little-endian transport, external interrupt wake from `SLEEP`,
privileged exception return, and a complete SoC integration remain explicit
follow-up work. The README and changelog must call this a bounded SH-2 integer
core, not a complete chip or complete SuperH-family simulator.

## 7. Completion gate

The cell is complete only when it has:

1. independent table-driven classification tests covering every admitted
   encoding family and its reserved/unsupported neighbors;
2. behavioral witnesses for every addressing mode, transfer width, alignment
   boundary, T/S/Q/M transition, multiply/divide result, compare, and branch
   sense;
3. forward/backward and register-target delay-slot witnesses, PR linkage,
   illegal-slot rejection, delayed target faults, sleep/halt, and full rollback;
4. complete-state load, restore, step, run, execute, trace-retention, malformed
   image, fetch, allocation, and budget tests;
5. release line coverage above 80 percent; and
6. scoped Rustfmt, strict Clippy, rustdoc, release tests, BUILD integration,
   lessons validation, latest-main ancestry, and independent exact-head review
   before publication.
