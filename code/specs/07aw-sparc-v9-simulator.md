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

Y is stored in the package's 64-bit state shape, but only its low 32 bits are
architectural. Its upper 32 bits always read as zero, snapshots canonicalize
them to zero, and restore rejects a nonzero reserved upper half.

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
- `SLL`, `SRL`, and `SRA` in non-`X` and `X` forms;
- `MULX`, `SDIVX`, `UDIVX`, `POPC`, and the retained `UMUL`, `SMUL`, `UDIV`,
  and `SDIV` V8-compatible forms, including their `cc` variants; `MULScc` is
  deliberately excluded from this first cell;
- `MOVcc` and `MOVr` for every integer condition admitted by their encoding;
- `BPcc`, `BPr`, the compatible `Bicc` family, `CALL`, `JMPL`, `RETURN`,
  `SAVE`, and `RESTORE`, including annul and delayed-transfer behavior;
- `LDSB`, `LDUB`, `LDSH`, `LDUH`, `LDSW`, `LDUW`, and the single-register
  64-bit `LDX`, together with `STB`, `STH`, `STW`, and `STX`;
- compatibility `LDD`/`STD` register-pair forms where their even-register-pair
  constraints are met; and
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

Non-`X` shift counts use five bits and `X` counts use six. Non-`X` `SLL`
nevertheless shifts the complete 64-bit source, while non-`X` `SRL` clears the
upper 32 bits and non-`X` `SRA` sign-extends bit 31. The `X` forms operate on
all 64 bits.

The `icc` N/Z/V/C fields describe the low 32-bit result. The `xcc` fields
describe the full 64-bit result. A `cc` instruction updates both sets from the
same operation at their respective widths; non-`cc` instructions preserve both.
Carry, borrow, and signed overflow follow the architecture's definitions, not
host-language overflow. Condition evaluation is centralized and exhaustively
tested for all branch and conditional-move senses.

`ADDC` and `SUBC` always consume `icc.C`, including their `cc` forms; they do
not select `xcc.C` for 64-bit operands. `SUBC` subtracts `icc.C`, and the
subtraction C bit records borrow as specified by V9.

`MULX` returns the low 64 bits of the signed-independent two's-complement
product. `SDIVX` and `UDIVX` fault atomically only on a zero divisor: the
`SDIVX` minimum-signed-value divided by minus one returns that minimum value,
and every `UDIVX` quotient fits 64 bits. `POPC` counts all 64 operand bits.

`UMUL` and `SMUL` multiply the low 32-bit operands, write the complete 64-bit
product to `rd`, and write product bits 63:32 to Y bits 31:0. Their `cc` forms
set `icc.N` from product bit 31, `icc.Z` when product bits 31:0 are zero, and
clear `icc.V/C`; they set `xcc.N` from product bit 63, `xcc.Z` when the whole
product is zero, and clear `xcc.V/C`.

`UDIV` and `SDIV` divide the 64-bit dividend formed from Y bits 31:0 and the low
32 bits of `rs1`; signed division truncates toward zero. They saturate an
out-of-range quotient to the architectural unsigned or signed 32-bit limit.
`UDIV` zero-extends and `SDIV` sign-extends the selected 32-bit result into
`rd`. Their `cc` forms set `icc.N/Z` from that 32-bit selected result, set
`icc.V` exactly on saturation, and clear `icc.C`; they set `xcc.N/Z` from the
extended 64-bit value in `rd` and clear `xcc.V/C`. Non-`cc` forms preserve both
condition-code sets. Because Y is architecturally undefined after a successful
legacy divide, this deterministic oracle preserves its pre-instruction low 32
bits and documents that choice in traces.

## 5. Memory and atomicity

Byte access is unaligned; halfword, word, and single-register extended-word
access requires two-, four-, and eight-byte alignment respectively. Signed
loads extend from their source width to 64 bits; unsigned loads clear upper
bits. `LDX`/`STX` transfer one 64-bit register. Stores use the low source bits
and big-endian byte order.

Compatibility `LDD` loads two consecutive 32-bit big-endian words and
zero-extends them into an even/odd register pair. If `rd` is `%g0`, the first
write is discarded but the odd partner still receives the second word. `STD`
stores the low 32 bits of each register in the pair; an `rd` of `%g0` stores a
zero first word. Both require an even `rd`, eight-byte alignment, and atomic
validation of the whole eight-byte range.

Effective addresses are full 64-bit wrapping register/immediate sums followed
by checked conversion to owned memory. Alternate-space, little-endian ASI,
non-faulting load, prefetch, block transfer, cache-control, and MMU forms are
outside this cell. `MEMBAR` validates its reserved fields and emits an exact
trace but needs no host fence because the model is single-threaded and
sequentially executed.

Every instruction preflights all source registers, window transitions, and
memory ranges before committing. `JMPL` and `RETURN` additionally validate
that their computed target's low two bits are zero before link, window, PC, or
nPC mutation. A typed fault preserves every register, counter, condition bit,
memory byte, PC/nPC value, halt bit, and image field.

## 6. Register windows and delayed control flow

Logical outs of window `CWP` overlap the physical ins of window `CWP + 1`
modulo four. `SAVE` increments CWP. It first returns a spill-window fault when
`CANSAVE == 0`; only otherwise does it return a clean-window fault when
`CLEANWIN - CANRESTORE == 0`. On success it decrements CANSAVE and increments
CANRESTORE. `RESTORE` and `RETURN` decrement CWP; they require
`CANRESTORE != 0`, then increment CANSAVE and decrement CANRESTORE. SAVE and
RESTORE read source operands in the old window and write their destination in
the new window. RETURN computes its target from old-window operands, restores
the window, and has no integer destination.

A transition that would require a spill, fill, or clean-window trap returns a
distinct typed window fault without changing state; this package does not
synthesize a trap handler. Restored state requires each counter to be in
`0..N-1`, `CANSAVE + CANRESTORE + OTHERWIN == N - 2`, and
`CLEANWIN >= CANRESTORE` for `N = 4`. This single-context model applies the
published SAVE predicate. It makes no claim to implement the OS/WSTATE
workaround or cross-context sanitization discussed by erratum 64.

Ordinary execution advances `(PC,nPC)` to `(nPC,nPC+4)`. A taken delayed
transfer installs its effective address in nPC so the instruction at old nPC
executes first. Annul follows V9 Table 13 and its errata. Taken `BA,a` is
non-delayed and advances directly to `(EA,EA+4)`. `BN,a`, and a not-taken
conditional branch with `a`, skip the slot. A skipped instruction is neither
fetched nor decoded and cannot fault; the branch trace carries explicit
`ANNULLED` metadata and the skipped address.

`CALL` and `JMPL` write the required link address before the delay slot, subject
to `%g0`. `RETURN` combines its target computation with the architected window
restore. A control-transfer instruction in a live delay slot uses the normal
V9 PC/nPC chaining rules: it executes with the incoming target already in nPC
and may replace the following nPC with its own target. Excluded privileged
`DONE`/`RETRY` special cases remain unsupported. A repository halt sentinel in
a live slot halts only after that successful slot step; unknown and unsupported
slot instructions return their ordinary typed errors.

`JMPL` and `RETURN` alignment is checked by the control-transfer instruction
itself. Branch and CALL targets are encoding-aligned. Installed-image and owned-
memory membership for any scheduled target is checked only when that target
becomes the next fetch. A failed slot `step` preserves the exact pre-slot state,
including PC, nPC, and the already-installed pending transfer. A transactional
`run` or `execute` failure instead rolls back that whole invocation. Successful
traces identify ordinary, delayed, chained, and annulled execution explicitly.

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

The exact canonical `TA 0` encoding is intercepted as this repository's
deterministic halt sentinel; this is not an architectural SPARC halt. It emits
a successful trace and enters halt. A later step returns `Halted` without
mutation. Every other `Tcc` encoding returns a typed unsupported-trap error;
no privileged trap state is fabricated.

Typed errors distinguish invalid programs and restored states, misaligned and
out-of-range instruction/data accesses, fetch outside the installed image,
unknown and unsupported instructions, divide-by-zero faults, window faults,
halted execution, excessive trace retention, allocation failure,
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
2. witnesses for 32-/64-bit flags, carry/borrow input, shift asymmetry, all
   condition senses, every transfer width and LDD/STD pair rule, sign extension,
   alignment, divide/product/popcount boundaries, Y behavior, and `%g0`;
3. exhaustive configured-window SAVE/RESTORE/RETURN transitions and typed
   spill/fill/clean-window rollback;
4. taken/not-taken, each annul table row, delayed and chained transfers,
   CALL/JMPL/RETURN, target-fault, slot-fault, and self-modifying exact-byte
   traces;
5. complete-state load, restore, halt, run, execute, trace-bound, allocation,
   malformed-image, fetch, and rollback tests;
6. release line coverage above 80 percent; and
7. scoped Rustfmt, strict Clippy, rustdoc, release tests, BUILD integration,
   lessons validation, latest-main ancestry, and independent exact-head review
   before publication.
