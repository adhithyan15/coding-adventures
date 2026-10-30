# Layer 07bg — MIPS R4000 / MIPS III Functional Simulator

**Status:** `RCPU-089` specification-only predecessor. Implementation is
forbidden until this exact contract receives an independent review. The
`RCPU-090` educational gate-level partner remains blocked until the functional
simulator merges.

## 1. Authority, evidence, and bounded profile

The normative source is Joe Heinrich, *MIPS R4000 Microprocessor User's
Manual*, Second Edition, MIPS Technologies / Silicon Graphics publication
007-2489-001, 1 April 1994. The public SGI technical-publications copy is
<https://techpubs.jurassic.nl/library/manuals/2000/007-2489-001/pdf/007-2489-001.pdf>.
It is a 754-page, 1,591,328-byte PDF with SHA-256
`47cff21fc018e28f814e87244084287a742196391bc7a5e416d7f7b39ed81876`.
The controlling passages are Chapter 1 pages 13–33, Chapter 2 pages 35–42,
Chapter 11 page 289, Chapter 12 pages 316–320 and 327–328, and Appendix A pages
A-1 through A-178. Instruction encodings and pseudocode in Appendix A control
over assembler convention or a later MIPS revision.

One non-normative differential source is pinned now rather than selected after
implementation: the official QEMU project tag `v10.1.2`, peeled commit
`ccaea6b2656ec6eab966585f7b16438208f98de7`, Git tree
`61cfba8633bcdd92ad95a8a8abdeecc0a1130973`. A developer-only generator will
build `qemu-mips64` from that exact clean tree and run source-locked static N64
test images with `-cpu R4000`. QEMU can corroborate common instruction
transitions but never overrides the manual, this bounded profile, or a
classified disagreement.

This layer models one fixed **MIPS III, 64-bit, big-endian, problem-state
integer** profile. It is an instruction and ordered-memory-transaction oracle,
not an R4000 pipeline, cache, TLB, CP0, FPU, operating system, board, or silicon
timing claim. The owned responder is a deterministic flat 64 KiB teaching
memory. Architectural registers and effective addresses remain 64 bits; an
access outside the responder faults instead of wrapping or truncating.

The included profile has hardware-interlocked loads, one architectural branch
delay slot, branch-likely nullification, 64-bit integer extensions, and one
local LL/SC reservation. It does not emulate an R3000 load delay. Configurable
little-endian operation, reverse-endian user mode, 32-bit operating mode,
virtual translation, caches, coherency, interrupts, and exception-vector
entry are separate work.

## 2. Complete authoritative state

`MipsR4000State` is an owned, frozen value containing exactly:

- all thirty-two 64-bit GPR projections, with GPR 0 canonically zero;
- 64-bit `HI` and `LO`;
- current `pc` and `next_pc`, both 64-bit;
- `in_delay_slot: bool` for the instruction at `pc`;
- one reservation: `ll_valid`, aligned `ll_address`, and `ll_width` (`4` or
  `8` when valid, zero otherwise);
- all 65,536 memory bytes;
- installed image `origin: Option<u64>` and `length: u64`; and
- the deterministic simulator `halted` bit.

`pc` names the next instruction to execute and `next_pc` names its successor.
Outside delayed control flow the canonical relation is `next_pc == pc + 4`
using 64-bit wrapping arithmetic. After an ordinary taken branch or jump,
`pc` names the delay slot, `next_pc` names the transfer target, and
`in_delay_slot` is true. This pipeline-visible state is included in snapshot,
restore, traces, equality, and rollback; it is not hidden host metadata.

When `ll_valid` is false, restore requires `ll_address == 0` and
`ll_width == 0`. When it is true, `ll_width` must be 4 or 8, the address must
have matching alignment, and the complete reserved span must be within owned
memory. The reservation is a deterministic single-core abstraction of the
manual's LLbit/LLAddr behavior, not a cache-coherency claim.

`new` and `reset` return the canonical empty state: zero registers, HI/LO,
memory and reservation; `pc = 0`, `next_pc = 4`; no image; not halted. A
successful load first resets the same state, installs the exact caller bytes,
sets `pc = origin`, `next_pc = origin + 4`, and records the image. Empty input
restores the canonical empty state. Program origins must be word-aligned and
the complete image must fit owned memory; no successful load leaves stale
bytes or a stale reservation.

## 3. Transport and ordered transactions

Instructions are fixed 32-bit big-endian words. Halfwords, words, and
doublewords use big-endian byte order. Public checked helpers read and write
8-, 16-, 32-, and 64-bit values. Natural halfword, word, and doubleword access
requires 2-, 4-, and 8-byte alignment. Every helper and instruction preflights
the complete span before reading or changing state.

Each successful step records, in order, one four-byte instruction-read
transaction and any data transaction. A transaction contains purpose,
direction, full 64-bit address, width, big-endian value, exact byte-enable
mask, and success. Failed instruction/data transport is also retained in the
returned typed error without mutating architectural state. Trace construction
must capture original fetched bytes before a self-modifying store.

`LWL/LWR/SWL/SWR` operate on the containing aligned four-byte word and
`LDL/LDR/SDL/SDR` on the containing aligned eight-byte doubleword. Their
big-endian merge tables come directly from Appendix A. The complete containing
span is preflighted atomically, the old register or memory value is read before
the destination is written, and byte-enable masks identify exactly the merged
lanes. No host slice, endian, or boundary behavior may decide the result
implicitly.

## 4. Included instruction surface

The complete bounded decoder admits these problem-state integer families and
no neighboring encoding by accident:

- base loads/stores: `LB`, `LBU`, `LH`, `LHU`, `LW`, `LWL`, `LWR`, `SB`,
  `SH`, `SW`, `SWL`, `SWR`;
- MIPS III loads/stores: `LD`, `LDL`, `LDR`, `LWU`, `SD`, `SDL`, `SDR`,
  `LL`, `LLD`, `SC`, `SCD`, and `SYNC`;
- immediate ALU: `ADDI`, `ADDIU`, `SLTI`, `SLTIU`, `ANDI`, `ORI`, `XORI`,
  `LUI`, `DADDI`, and `DADDIU`;
- register ALU: `ADD`, `ADDU`, `SUB`, `SUBU`, `SLT`, `SLTU`, `AND`, `OR`,
  `XOR`, `NOR`, `DADD`, `DADDU`, `DSUB`, and `DSUBU`;
- HI/LO: `MULT`, `MULTU`, `DIV`, `DIVU`, `DMULT`, `DMULTU`, `DDIV`,
  `DDIVU`, `MFHI`, `MTHI`, `MFLO`, and `MTLO`;
- shifts: `SLL`, `SRL`, `SRA`, `SLLV`, `SRLV`, `SRAV`, `DSLL`, `DSRL`,
  `DSRA`, `DSLLV`, `DSRLV`, `DSRAV`, `DSLL32`, `DSRL32`, and `DSRA32`;
- control: `J`, `JAL`, `JR`, `JALR`, `BEQ`, `BNE`, `BLEZ`, `BGTZ`,
  `BLTZ`, `BGEZ`, `BLTZAL`, `BGEZAL`, and all eight corresponding
  branch-likely forms;
- traps: `TGE`, `TGEU`, `TLT`, `TLTU`, `TEQ`, `TNE`, `TGEI`, `TGEIU`,
  `TLTI`, `TLTIU`, `TEQI`, `TNEI`, `SYSCALL`, and `BREAK`; and
- the all-zero canonical `NOP` alias of `SLL r0,r0,0`.

The decoder validates all fixed fields. In particular, immediate `LUI` has
`rs == 0`; fixed shifts have `rs == 0`; variable shifts have `sa == 0`;
`JR`, `JALR`, HI/LO moves, multiply/divide, arithmetic/logical operations,
and `SYNC` obey their Appendix A zero fields; `BLEZ/BGTZ` and likely variants
have `rt == 0`. Trap code fields and `BREAK/SYSCALL` code fields remain
software payload rather than invented reserved-zero fields. Exhaustive
classification covers all 64 primary opcodes, all 64 `SPECIAL` functions,
all 32 `REGIMM` selectors, and every one-bit fixed-field mutation.

All COP0–COP3 moves, branches, operations and loads/stores; `CACHE`; `ERET`;
TLB instructions; FPU; watch/debug operations; implementation-specific
encodings; and every reserved hole are typed unsupported or unknown. They do
not alias an included instruction. The later gate partner may use public RTL
only as source-backed evidence and remains blocked on this functional merge.

## 5. Integer semantics

Every GPR read of register zero returns zero and every write is discarded.
All source operands are captured before any GPR, HI, LO, reservation, memory,
or control state is changed.

Word arithmetic (`ADD[U]`, `SUB[U]`, `ADDI[U]`) operates on low 32 bits and
sign-extends the 32-bit result to 64 bits. Signed forms fault on 32-bit signed
overflow; unsigned-named forms wrap without trapping. Doubleword `DADD[U]`,
`DSUB[U]`, and `DADDI[U]` use all 64 bits and only signed forms fault on
64-bit overflow. Logical and set-less-than operations use all 64 bits in this
64-bit profile; immediate comparisons sign-extend and logical immediates
zero-extend their 16-bit field. `LUI` sign-extends the 32-bit value
`immediate << 16`.

Appendix A requires every register operand consumed by the word arithmetic,
word multiply, word divide, and right-shift (`SRL[V]`/`SRA[V]`) families to be
a valid sign-extended 32-bit value in 64-bit mode. A value whose upper 32 bits
do not replicate bit 31 therefore returns `UndefinedOperation` before
overflow, divide, or destination effects. The shifted `rt` operand of
`SLL/SLLV` is the documented exception: those instructions truncate the low
word and sign-extend the result even when `rt` was not a valid sign-extended
word. A variable shift count is always selected from the documented low five
bits of `rs` and does not impose this validity condition on `rs`.

Word shifts consume low 32 bits, use five count bits, and sign-extend their
32-bit result. Doubleword variable shifts use six count bits. `DSLL/DSRL/DSRA`
select counts 0–31 and their `32` variants select 32–63.

`MULT/MULTU` multiply low 32-bit operands and place sign-extended 32-bit
product halves in HI and LO as Appendix A specifies. `DMULT/DMULTU` place the
upper/lower halves of the complete 128-bit product in HI/LO. Word division
uses low 32-bit operands and writes sign-extended 32-bit remainder/quotient;
doubleword division uses all 64 bits. Signed division truncates toward zero
and remainder has the dividend's sign. A zero divisor, or minimum signed value
divided by minus one, is architecturally undefined in this bounded oracle and
returns `UndefinedOperation` atomically rather than borrowing host behavior.

Signed trap relations use complete 64-bit operands; unsigned traps use their
bit patterns. Immediate traps sign-extend the 16-bit operand, including the
unsigned comparisons required by Appendix A. A true trap returns typed
`Trap` atomically because CP0 exception entry is excluded; a false trap is a
successful ordinary step.

## 6. Loads, stores, and LL/SC

Effective addresses are the wrapping 64-bit sum of `GPR[base]` and signed
16-bit offset, followed by checked owned-memory conversion. `LB/LH/LW/LL`
sign-extend; `LBU/LHU/LWU` zero-extend; `LD/LLD` transfer all 64 bits. Stores
use the low source width. Alignment is checked before owned range, and the
entire transfer is preflighted before a register, memory byte, reservation, or
trace is committed.

`LL` and `LLD` load first, then set a width-specific aligned reservation. A
matching `SC` or `SCD` captures the old `rt` store value, preflights alignment
and range even when the reservation is invalid, conditionally stores, writes
one or zero to `rt`, and clears the reservation. No valid reservation means a
defined failed conditional store. A valid reservation with different address
or width is Appendix A's undefined case and returns `UndefinedOperation`
atomically. As this profile's deterministic conservative replacement for
cache/coherency LLbit invalidation, every successful ordinary store and the
public `invalidate_reservation` input clears the reservation; ordinary loads
preserve it. This policy is part of the teaching oracle and is not attributed
to an exact R4000 cache event. `SYNC` requires its exact zero encoding and is a
traced no-op in this single-threaded sequential responder.

## 7. Delayed control flow and terminal convention

Ordinary execution advances `(pc,next_pc)` to `(next_pc,next_pc+4)`. A normal
taken branch or jump advances to the delay-slot address while installing its
target in `next_pc`; the slot executes on the following step. A normal
not-taken branch still executes its delay slot, with the sequential successor
already in `next_pc`. A not-taken branch-likely nullifies the slot: the skipped
word is neither fetched nor decoded and cannot fault, and the branch trace
records the annulled address. Taken likely branches use an ordinary live slot.

Branch displacement is sign-extended, shifted left two, and added to the delay
slot address (`branch pc + 4`), modulo 2^64. `J/JAL` combine the target field
with bits 63:28 of the delay-slot address. Register jumps capture the target
before a link write. Link addresses use wrapping `pc + 8`. `JAL` writes the
link to r31 and `JALR` writes it to `rd`. All
four conditional link branches write r31 **unconditionally**, even when their
condition is false; a false likely-link form both writes the link and nullifies
its slot. Link forms follow the manual's restartability constraints: `JALR
rd == rs` and a conditional link whose link register is also its source return
`UndefinedOperation` atomically.

A jump or branch encoding in a live delay slot returns
`UnsupportedDelaySlot` without changing the already-pending transfer. Fetch,
truncation, and decode required to identify that instruction precede this
classification. Jump-register target alignment is not eagerly rejected: the
control instruction and legal slot commit, and the subsequent fetch reports
`MisalignedFetch`, matching Appendix A's stated timing. Other out-of-image or
out-of-responder targets likewise commit and fail only on the next fetch.

MIPS III has no architectural halt instruction. This repository reserves
`BREAK` code `0x48414` as the deterministic `HALT` sentinel. That exact form
performs ordinary PC/next-PC completion, then sets `halted`, and emits mnemonic
`HALT`. Thus a HALT in a legal live slot first completes the pending transfer.
Every other `BREAK` and every `SYSCALL` returns its distinct typed
architectural event without CP0 entry. Post-halt step/run fails without
mutation.

## 8. Fault priority and transactional lifecycle

For one step, classification order is:

1. `Halted`;
2. instruction-address alignment;
3. complete owned fetch span;
4. installed-image membership and truncation;
5. strict decode and fixed/reserved fields;
6. live delay-slot legality;
7. source and undefined-operation constraints;
8. data alignment, then complete data span;
9. arithmetic/trap/system-call/breakpoint event; and
10. candidate trace/final-state allocation and atomic commit.

Unknown/reserved decode therefore precedes operand-dependent behavior, while
transport needed to obtain an instruction precedes decode. A false ordinary
branch does not validate its target; a not-taken likely branch does not fetch
its annulled slot. SC/SCD alignment/range faults take precedence over a failed
reservation as the manual requires.

Typed host errors include `AllocationFailed`, `ProgramTooLarge`,
`MisalignedProgram`, `ProgramOutOfRange`, `Halted`, `MisalignedFetch`,
`FetchOutsideProgram`, `TruncatedInstruction`, `UnknownInstruction`,
`ReservedEncoding`, `UnsupportedInstruction`, `UnsupportedDelaySlot`,
`UndefinedOperation`, `MisalignedData`, `DataOutOfRange`,
`ArithmeticOverflow`, `Trap`, `SystemCall`, and `Breakpoint`. Errors carry the
faulting address/raw word where applicable and preserve complete entry state.

`snapshot`, `restore`, `load`, direct memory helpers, `step`, `run`, and
`execute` are checked. Restore validates every invariant before replacement.
`run(max_steps,max_traces)` is transactional: success returns exact retained
traces and final state; source error, step limit, trace limit, or allocation
failure restores entry state. `execute` is the same transaction around load
plus run. No API panics, silently truncates, wraps owned memory, or retains a
partial candidate.

All production allocations are fallible with deterministic test-only seams at
state/memory clone, load copy, snapshot/restore staging, fetched-byte and
mnemonic retention, trace construction/retention, result/final-state creation,
and execute wrapping. Both run and execute have a seam after at least one
successful instruction, with exact entry-state rollback. Tests must prove each
named seam reaches a real production allocation or narrow this contract before
implementation.

## 9. Trace and independent validation contract

`MipsR4000Trace` contains instruction PC, original four raw bytes, uppercase
mnemonic, captured operands, destinations/effects, ordered transactions,
delay-slot/annul/link/reservation metadata, and complete states before and
after. Failed steps return the same available raw/transaction context without
a fabricated successful state.

Completion requires independent, reproducible evidence rather than the
existing R2000 simulator (which deliberately lacks architectural delay slots
and is not an oracle for this profile):

1. a literal manual-derived classification table covering every primary,
   `SPECIAL`, and `REGIMM` selector and every fixed-field neighbor;
2. checked-in transition vectors derived independently from Appendix A for
   every admitted family, source/destination overlap, r0, register extrema,
   immediate extrema, valid and invalid sign-extended word operands,
   arithmetic/shift/multiply/divide/trap boundary, and each typed fault;
3. exhaustive or mathematically complete 32- and 64-bit carry, borrow,
   overflow, signed/unsigned relation, shift-count, and HI/LO edge evidence;
4. every aligned width/residue/endpoint, all 4- and 8-byte merge offsets in
   both complementary pair orders, self-modification, and exact byte enables;
5. all ordinary/likely branch senses taken and untaken, link overlap, jump
   region construction, live-slot execution, annul, pending-target rollback,
   and next-fetch target faults;
6. LL/SC and LLD/SCD success, no-reservation failure, width/address mismatch,
   base/rt overlap, store-source capture, invalidate, and fault precedence;
7. new/reset/load/snapshot/restore/helpers/halt/step/run/execute, malformed
   complete state, every allocation seam, and post-progress rollback; and
8. a corpus from the pinned official QEMU tree for common problem-state
   instructions. Its generator records the exact clean source tree, build
   configuration, compiler/QEMU versions, N64 image sources and hashes, and
   classifies every disagreement against the primary manual before completion.

The implementation must not call or embed the reference emulator. Checked-in
corpora are developer-reproducible offline once their pinned public source or
binary is supplied; ordinary package builds and tests remain network-free.

## 10. Completion gate and exclusions

The functional cell is complete only when the reviewed spec-only commit
precedes implementation; package/workspace/BUILD/README/changelog wiring is
present; all matrices above and full-state corpus consumers pass; release
tests, package-scoped formatting, strict Clippy/rustdoc, affected BUILD, and
lessons validation are green; measured package-source line coverage exceeds
80%; documentation records observed counts only; current `origin/main` is
merged; the tree is clean; and a different reviewer grants an exact-head PASS.

Explicit exclusions are CP0 registers and instructions, TLB/MMU and address
spaces, exception-vector entry/return, interrupts/NMI/reset vectors, caches and
cache operations, FPU/CP1, CP2/CP3, reverse/little-endian operation, 32-bit
mode, weak-memory and multiprocessor timing, debug/watch, performance counters,
pipeline/cycle timing, R4400 errata/features, board/SoC peripherals, and any
transistor or proprietary physical-layout claim. Each is a later source-backed
cell; none may be silently approximated here.
