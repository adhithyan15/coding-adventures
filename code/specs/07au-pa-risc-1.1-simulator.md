# Layer 07au — HP PA-RISC 1.1 Functional Simulator

## 1. Status and primary source

This layer defines a checked instruction-level model of a deliberately bounded
PA-RISC 1.1 problem-state integer processor. Its normative source is
Hewlett-Packard's *PA-RISC 1.1 Architecture and Instruction Set Reference
Manual*, third edition, part number 09740-90039 (February 1994). Encoding,
operand ordering, immediate assembly, branch displacement, address-queue, and
nullification rules come from that manual rather than a later derivative ISA or
an emulator implementation.

PA-RISC implementations contain substantially more machinery than this cell.
This package is an architectural oracle for the coherent user-mode subset below,
not a claim to reproduce an HP processor's pipeline, timing, or complete system.

## 2. Bounded architecture

The simulator has 32 unsigned 32-bit general registers, with GR0 hard-wired to
zero, and a flat 64 KiB byte-addressed teaching memory. Instructions are fixed-width
32-bit words and both instruction and data transport are big-endian. The
architecturally visible instruction address queues are represented by `pc`
(IAOQ front) and `next_pc` (IAOQ back). The complete state also contains the
eight PSW carry/borrow outputs (C/B{0} through C/B{7}), the pending
nullification bit, an explicit bounded-model delay-slot marker,
deterministic halt state, installed-program origin and length, and every memory
byte.

The included PA-RISC 1.1 problem-state families are:

- integer ADD and SUB plus AND, ANDCM, OR, and XOR register operations, in the
  non-trapping, non-nullifying completer forms admitted by this boundary;
- LDO, LDIL, and ADDIL immediate/address construction;
- SHD with immediate shift count for 32-bit double-word funnel shifts;
- naturally aligned LDB, LDH, LDW, STB, STH, and STW in space zero;
- COMBT/COMBF and COMIBT/COMIBF compare-and-branch forms;
- ADDBT/ADDBF and ADDIBT/ADDIBF add-and-branch forms;
- BL, BLR, and BV calls, computed branches, and conventional returns; and
- BREAK 0,0 as the repository's deterministic halt sentinel.

The included instruction names denote only the exact formats and reserved-bit
constraints documented here. Other completers or reserved neighbors never
alias an included operation.

## 3. Register, immediate, and data rules

Reading GR0 always returns zero and writes to GR0 are discarded. Restored state
must therefore contain zero in the GR0 slot. Arithmetic wraps at 32 bits unless
an included branch condition explicitly consumes the full carry or overflow
relationship. ADD and SUB update the eight PSW carry/borrow bits from the
corresponding 4-bit ALU digit outputs exactly as the architecture manual
defines: ADD records each carry out, while SUB records zero for a borrow and one
for no borrow. The array is ordered C/B{0} (most-significant digit) through
C/B{7} (least-significant digit). Included logical operations leave it
unchanged. Completers that consume carry/borrow as input remain outside this
boundary.

PA-RISC's low-sign immediates are not ordinary two's-complement bit fields. For
an N-bit low-sign field, bit 0 supplies the sign and bits N-1:1 supply the
magnitude bits N-2:0; the assembled value is then sign-extended. LDIL and ADDIL
use the manual's `assemble_21` permutation before shifting left eleven. Branch
displacements use the manual's `assemble_12` or `assemble_17` permutation,
sign-extension, and final left shift by two. Tests must independently encode
positive, negative, minimum, maximum, and permuted-bit boundary values.

Byte loads zero-extend, half-word loads zero-extend, and word loads preserve all
bits. Stores use the low 8, 16, or 32 bits. Half-word and word accesses must be
naturally aligned and every access must fit without wrapping in owned memory.
The two-bit space field must be zero. Data may be accessed anywhere in memory;
instruction fetch is additionally confined to the installed image.

SHD shifts the concatenation of the rightmost 31 bits of the first operand and
the full second operand right by the decoded count, selecting the low 32-bit
result. Count zero and count 31 are explicit acceptance boundaries.

## 4. Address queues, delayed control flow, and nullification

On reset or load, `pc = origin` and `next_pc = origin + 4`. An ordinary step
advances the queues to `(old_next_pc, old_next_pc + 4)`. A taken branch leaves
the old `next_pc` instruction in the front queue as its delay-slot instruction
and replaces the new back queue with `old_pc + displacement + 8` (or the
computed target). The target must be word aligned and inside the installed
image when it is eventually fetched.

The explicit `n` bit on BL, BLR, and BV nullifies the delay-slot instruction.
For conditional branches, the manual's static prediction/nullification rule is
normative: when `n` is set, a forward branch nullifies its delay slot when
taken, while a backward branch nullifies its delay slot when not taken. A
nullified step still fetches the exact installed word, emits an exact
`NULLIFIED` trace, advances both address queues, and clears pending
nullification, but it causes no other architectural effect.

BL and BLR write the address following the delay slot (`old_next_pc + 4`) to
their link register, subject to the GR0 rule. BV supplies the conventional
register-indirect return. Architecturally, BV forms IAOQ_Next address bits
0..29 from the base-plus-index sum and assigns the two privilege-level bits
separately. This flat problem-state model has no privilege levels, so it
normalizes those low two bits to zero rather than treating them as address
misalignment.

PA-RISC defines the queue behavior of a taken control transfer in another
transfer's delay slot. This bounded first cell deliberately excludes that
documented behavior and rejects any control-transfer instruction encountered
in another transfer's live delay slot, including the linear queue state after
a not-taken transfer. Supporting nested delayed transfers remains explicit
follow-up work; the exclusion is a scope boundary, not a claim that the
architectural sequence is undefined or nonportable.

Compare branches evaluate the manual's never, equal, signed less-than,
signed-less-or-equal, unsigned less-than, unsigned-less-or-equal, signed
overflow, and odd-result conditions. Add branches write their wrapping sum
whether or not the branch is taken, then apply the analogous zero, signed,
no-unsigned-carry, overflow, and odd conditions. The T forms branch when the
condition is true; the F forms invert it.

## 5. Transport, traces, and transactional lifecycle

`load(program, origin)` accepts a nonempty, word-multiple, word-aligned,
contiguous big-endian image that fits memory. A successful load replaces the
entire state. Fetch must be word aligned and wholly within that exact image; no
terminal address wraps. `restore` validates all register, queue, nullification,
halt, image, and memory invariants before committing.

Every successful `step` returns the front PC and next PC before and after the
transition, exact four fetched bytes and decoded word, a stable conventional
uppercase mnemonic, whether the instruction was nullified, and a complete
post-step snapshot. `step` is atomic. `run(max_steps)` and
`execute(program, origin, max_steps)` are transactional as a whole: any typed
fault or exhausted step budget restores every register, queue, memory byte,
halt bit, and image field. Trace retention is bounded to 64 complete-state
traces (a 4 MiB snapshot budget) and is preflighted before mutation.

`BREAK 0,0` produces a successful `BREAK` trace and enters halt. A later step
returns `Halted` without mutation. Other BREAK encodings remain a typed
unsupported boundary.

Typed errors distinguish invalid programs and restored states, misaligned and
out-of-range instruction or data accesses, fetch outside the installed image,
unknown and unsupported instructions, unsupported delayed control transfer,
halted execution, excessive trace retention, allocation failure, and exhausted
step budgets. Each error has stable nonempty display text.

## 6. Explicit exclusions and follow-up backlog

This cell excludes privileged instructions, space-register and control-register
access, traps and exception entry, interruption state, protection identifiers,
the MMU/TLB, virtual aliases, caches, coprocessors, FPU instructions and state,
quad-word and semaphore accesses, cache-control and probe operations, external
buses, multiprocessor ordering, implementation-specific diagnostics, pipeline
timing, and cycle accuracy.

It also deliberately leaves these documented problem-state families for later
bounded extensions: extract/deposit/variable-shift operations, unit operations,
diagnose-like implementation extensions, indexed and short-displacement memory
forms, load/store modification completers, decimal correction, multiply and
divide families, MOVB/MOVIB branches, and the remaining arithmetic completers
and condition/nullification forms. Their encodings fail as typed unsupported or
unknown instructions; no README or changelog may call this a complete PA-RISC
1.1 ISA simulator.

## 7. Completion gate

The cell is complete only when it has:

1. independent table-driven encoding/classification tests for every admitted
   family, every primary opcode, required extension value, and reserved
   neighbor;
2. behavioral witnesses for GR0, every memory width and alignment boundary,
   low-sign and assembled immediates, all eight branch conditions and both T/F
   senses, forward/backward delay-slot nullification, calls and returns;
3. complete-state trace, load, restore, halt, fetch, allocation, rollback,
   malformed-word, delayed-control, and bounded-run tests;
4. release line coverage above 80 percent (completion baseline: 92.80%,
   580/625 lines); and
5. scoped Rustfmt, strict Clippy, rustdoc, release tests, workspace build, BUILD
   integration, lessons validation, and independent review before publication.
