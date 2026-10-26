# Layer 07aw — SPARC V9 Functional Simulator

## 1. Status and primary sources

This layer defines a checked instruction-level model of a deliberately bounded
SPARC Version 9 integer processor. Its normative source is SPARC
International's *The SPARC Architecture Manual, Version 9*, document
SAV09R1459912, together with SPARC International's published V9 errata.
Register widths, instruction encodings, operand ordering, condition codes,
register-window transitions, delayed control transfer, and memory alignment
come from those sources rather than from one UltraSPARC implementation.

SPARC V9 defines privileged state, traps, multiple address spaces, floating
point, and a memory model in addition to the integer ISA. This package is a
coherent problem-state integer oracle for the boundary below. It is not a claim
to reproduce an UltraSPARC/SPARC64 pipeline, cache hierarchy, MMU, server, or
complete V9 system.

## 2. Bounded architectural state

The simulator owns eight normal 64-bit global registers and four windows of
sixteen 64-bit physical registers. The current window exposes 32 logical
registers as eight globals, eight outs, eight locals, and eight ins; `%g0` is
hard-wired to zero. The complete state includes all 72 physical registers,
`PC`, `nPC`, `Y`, the integer condition-code fields `icc` and `xcc`, and the V9
window-management fields `CWP`, `CANSAVE`, `CANRESTORE`, `CLEANWIN`,
`OTHERWIN`, and `WSTATE` required by admitted `SAVE`, `RESTORE`, and `RETURN`
operations.

The state also owns deterministic halt, installed-program origin and length,
and every byte of flat 64 KiB teaching memory. Normal globals are the only
global set. Privileged alternate/interrupt/MMU globals and privileged PSTATE,
TL, TSTATE, TPC, TNPC, TT, PIL, TBA, VER, and ASI state are outside this first
cell. Address masking is fixed off and transport is fixed to big-endian primary
address space semantics.

Instructions are fixed-width 32-bit words. Architectural addresses and integer
registers are 64 bits, but any actual fetch or data transfer must fit owned
memory without wrapping or truncation.

## 3. Included instruction boundary

The first cell includes the following V9 integer families in their documented
problem-state forms:

- `SETHI` and canonical `NOP`;
- 64-bit `ADD`, `ADDC`, `SUB`, `SUBC`, `AND`, `ANDN`, `OR`, `ORN`, `XOR`, and
  `XNOR`, plus their `cc` forms that update both required integer condition-code
  views;
- `SLL`, `SRL`, and `SRA` in 32-bit and `X` 64-bit forms;
- `MULX`, `SDIVX`, `UDIVX`, `POPC`, and the architecturally retained V8
  multiply/divide forms where they have an unambiguous problem-state result;
- `MOVcc` and `MOVr` for every integer condition admitted by their encoding;
- `BPcc`, `BPr`, the compatible `Bicc` family, `CALL`, `JMPL`, `RETURN`,
  `SAVE`, and `RESTORE`, including annul and delayed-transfer behavior;
- signed and unsigned byte, halfword, and word loads; signed/unsigned extended
  word loads; doubleword loads; byte, halfword, word, and doubleword stores;
- `LDD`/`STD` compatibility forms where their even-register-pair constraints
  are met; and
- `MEMBAR` as a validated architectural no-op in this single-core sequential
  model, plus `TA 0` as the repository's deterministic halt sentinel.

Reserved op/op2/op3 values, privileged forms, unsupported ASIs, VIS opcodes,
and implementation-dependent instructions fail as typed unsupported or unknown
instructions. They never alias a nearby included operation.

## 4. Arithmetic and condition codes

Register operations are 64-bit unless the encoding selects the architectural
32-bit result. Immediate operands use the manual's signed `simm13` or other
family-specific assembly rules. Writes to `%g0` are discarded, reads return
zero, and restored physical global zero must be zero.

The `icc` N/Z/V/C fields describe the low 32-bit result. The `xcc` fields
describe the full 64-bit result. A `cc` instruction updates both sets from the
same operation at their respective widths; non-`cc` instructions preserve both.
Carry, borrow, and signed overflow follow the architecture's definitions, not
host-language overflow. Condition evaluation is centralized and exhaustively
tested for all branch and conditional-move senses.

`MULX` returns the low 64 bits of the signed-independent two's-complement
product. `SDIVX` and `UDIVX` fault atomically on zero divisors and on the
architecture's quotient overflow boundary. `POPC` counts all 64 operand bits.
Any admitted legacy operation that uses Y names that dependency explicitly in
its trace and complete state.

## 5. Memory and atomicity

Byte access is unaligned; halfword, word, extended-word, and doubleword access
requires two-, four-, four-, and eight-byte alignment respectively. Signed
loads extend from their source width to 64 bits; unsigned loads clear upper
bits. Stores use the low source bits and big-endian byte order. `LDD` and `STD`
validate even register pairing and the complete memory range before mutation.

Effective addresses are full 64-bit wrapping register/immediate sums followed
by checked conversion to owned memory. Alternate-space, little-endian ASI,
non-faulting load, prefetch, block transfer, cache-control, and MMU forms are
outside this cell. `MEMBAR` validates its reserved fields and emits an exact
trace but needs no host fence because the model is single-threaded and
sequentially executed.

Every instruction preflights all source registers, window transitions, target
alignment, and memory ranges before committing. A typed fault preserves every
register, counter, condition bit, memory byte, PC/nPC value, halt bit, and image
field.

## 6. Register windows and delayed control flow

Logical outs of one window overlap physical ins of the adjacent window.
`SAVE`, `RESTORE`, and `RETURN` follow the V9 CWP and window-counter algorithms,
including the destination register's old/new-window selection. A transition
that would require a spill, fill, or clean-window trap returns a distinct typed
window fault without changing state; this package does not synthesize a trap
handler. Restored state must satisfy the manual's counter conservation and
range invariants for the configured four-window file.

Ordinary execution advances `(PC,nPC)` to `(nPC,nPC+4)`. A taken transfer
executes the instruction at old `nPC` as its delay slot before the target is
fetched. Annul follows each branch family's V9 rule: the not-taken conditional
slot and the taken branch-always slot are suppressed only where the encoded
annul contract says so. An annulled step still validates/fetches the installed
word, emits an exact `ANNULLED` trace, advances PC/nPC, and has no other effect.

`CALL` and `JMPL` write the required link address before the delay slot, subject
to `%g0`. `RETURN` combines its target computation with the architected window
restore. A control transfer, halt sentinel, or unsupported instruction in a
live delay slot returns `IllegalDelaySlot` atomically rather than inventing
implementation-specific chaining.

Target alignment and installed-image membership are checked when the target
becomes the next fetch. A fault in a delay-slot instruction rolls back the
slot and pending transfer. Successful traces identify ordinary, delayed, and
annulled execution explicitly.

## 7. Transport, traces, and lifecycle

`load(program, origin)` accepts a nonempty, word-multiple, word-aligned,
contiguous big-endian image that fits memory. A successful load replaces the
entire state. Fetch must be word aligned and wholly inside that exact image;
terminal fetches never wrap. `restore` validates physical registers, `%g0`,
condition bits, window counters, PC/nPC, pending delay/annul metadata, halt,
image metadata, and memory size before committing.

Every successful `step` returns PC and nPC before and after the transition,
the exact four bytes fetched before any self-modifying write, decoded word,
stable uppercase mnemonic, delay/annul metadata, and a complete post-state.
`step` is atomic. `run(max_steps)` and `execute(program, origin, max_steps)` are
transactional as a whole and preflight a maximum of 64 retained complete-state
traces.

`TA 0` emits a successful trace and enters deterministic halt. A later step
returns `Halted` without mutation. Other trap instructions return typed
unsupported-trap errors; no privileged trap state is fabricated.

Typed errors distinguish invalid programs and restored states, misaligned and
out-of-range instruction/data accesses, fetch outside the installed image,
unknown and unsupported instructions, divide faults, window faults, illegal
delay slots, halted execution, excessive trace retention, allocation failure,
and exhausted budgets. Every error has stable nonempty display text.

## 8. Explicit exclusions and follow-up backlog

This cell excludes floating-point registers and operations, VIS, privileged
instructions/state, trap entry and return, interrupt delivery, alternate
global sets, alternate address spaces, ASI-controlled little endian, MMU/TLB,
address masking, compare-and-swap and other multiprocessor atomics, prefetch,
non-faulting loads, cache control, the relaxed-memory implementation model,
external buses, timers and peripherals, pipeline timing, and cycle accuracy.

Follow-up cells must separately add FPU/VIS state, privileged/trap execution,
atomics plus a multi-core memory model, alternate address spaces/endian modes,
and a source-backed V9 gate implementation. The README and changelog must call
this a bounded V9 problem-state integer core, not a complete SPARC server or
complete V9 implementation.

## 9. Completion gate

The cell is complete only when it has:

1. table-driven classification tests for every admitted opcode family and its
   reserved, privileged, ASI, VIS, and implementation-dependent neighbors;
2. witnesses for 32-/64-bit flags, all condition senses, every transfer width,
   sign extension, alignment, divide/product/popcount boundaries, and `%g0`;
3. exhaustive configured-window SAVE/RESTORE/RETURN transitions and typed
   spill/fill/clean-window rollback;
4. taken/not-taken, annulled, delayed, CALL/JMPL/RETURN, target-fault, slot-fault,
   and self-modifying exact-byte traces;
5. complete-state load, restore, halt, run, execute, trace-bound, allocation,
   malformed-image, fetch, and rollback tests;
6. release line coverage above 80 percent; and
7. scoped Rustfmt, strict Clippy, rustdoc, release tests, BUILD integration,
   lessons validation, latest-main ancestry, and independent exact-head review
   before publication.
