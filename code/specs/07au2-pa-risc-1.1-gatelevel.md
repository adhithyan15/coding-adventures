# Layer 07au2 — HP PA-RISC 1.1 Gate-Level Simulator

## 1. Status, source, and evidence boundary

This specification defines the educational gate-level companion to the merged
`pa-risc11-simulator` and Spec 07au. Its independently accepted specification
now has a complete local implementation awaiting independent implementation
review and publication.

The normative public architecture source is Hewlett-Packard's
[*PA-RISC 1.1 Architecture and Instruction Set Reference Manual*, third
edition](https://parisc.docs.kernel.org/en/latest/_downloads/f15883d2c095ac4e7606ac263ab8d883/Pa11_acd.pdf),
part number 09740-90039 (February 1994), SHA-256
`9ebb5eb9a178202027f26d415ade387710f0800fa10aa932662227f8b0510949`.
In particular, Chapter 2 defines the 32 general registers, hard-wired GR0,
processor status and paired instruction-address queues; Chapter 4 defines
delayed control flow, queue updates, and direction-sensitive nullification;
Chapter 5 and Appendices C/D define the admitted formats, operation semantics,
major opcodes, and extension fields. Spec 07au remains the normative bounded
functional contract when it deliberately narrows the manual.

The package is an **educational ISA-level gate topology**. It does not claim to
reconstruct any HP silicon netlist, processor implementation, die layout,
pipeline, transistor count, or timing. Its DFF count is derived only from the
explicit bounded architectural and lifecycle state below. The manual defines
architectural behavior, not the internal gates chosen here.

## 2. Exact functional boundary

The gate package must implement exactly the complete Spec 07au surface:

- 32 unsigned 32-bit general-register slots, with GR0 reading as zero,
  discarding writes, and validating as zero on restore;
- 64 KiB of byte-addressed big-endian memory;
- 32-bit `pc` and `next_pc`, representing the front and back IAOQ offsets;
- all eight PSW C/B digit carry/borrow outputs, ordered C/B{0} at the most
  significant nibble through C/B{7} at the least significant nibble;
- the pending nullification bit, bounded-model live-delay-slot bit, halt bit,
  installed-image origin, and installed-image length;
- `BREAK 0,0`; ADD, SUB, AND, ANDCM, OR, XOR; LDO, LDIL, ADDIL; immediate SHD;
  naturally aligned LDB/LDH/LDW and STB/STH/STW in space zero; the register and
  immediate COMBT/COMBF and ADDBT/ADDBF families; and BL, BLR, and BV; and
- exact big-endian transport, stable uppercase mnemonics, complete snapshots,
  typed faults, and transactional load/restore/step/run/execute behavior.

No later PA-RISC instruction, completer, privilege facility, or implementation
extension may alias this boundary. A primary opcode, extension, reserved bit,
space selector, completer, or branch form outside Spec 07au must return the same
`UnknownInstruction`, `UnsupportedInstruction`, or
`UnsupportedDelayedControl` value as the functional oracle, at the same fault
priority, without changing a persistent bit.

The production gate engine may share the functional package's public value and
error types for API compatibility. It must not call its decoder, execution,
step, run, execute, or private helper paths. The functional package is an
oracle only in development tests.

## 3. Exact persistent DFF inventory

Every persistent bit is represented by the stable Q output of a repository
`logic-gates` D flip-flop. Packed stable-Q storage is permitted for scale, but
reset, load, restore, direct lifecycle installation, instruction commit, halt,
and rollback must clock each changed bit through the repository sequential
primitive. A reconstructible index of nonzero memory bytes is host metadata,
not architectural authority.

| Bank | Width/count | DFFs |
|---|---:|---:|
| Big-endian memory | 65,536 x 8 | 524,288 |
| GR0–GR31 physical slots | 32 x 32 | 1,024 |
| IAOQ front and back offsets | 2 x 32 | 64 |
| PSW C/B outputs | 8 x 1 | 8 |
| Pending nullification | 1 | 1 |
| Live delay-slot marker | 1 | 1 |
| Halt latch | 1 | 1 |
| Installed-program origin | 32 | 32 |
| Installed-program length, including 65,536 | 17 | 17 |
| **Total** | | **525,436** |

GR0 has physical Q bits so the complete public state and exact topology remain
one-to-one. Its read mux selects a constant-zero wire, its write-enable line is
always disabled, and reset/restore require its Q outputs to be zero. At six
primitive gates per repository master/slave DFF, persistent storage represents
3,152,616 gate equivalents; transient combinational networks are additional.

The package exports `FLIP_FLOP_COUNT = 525_436`. The inventory test must derive
the total from the actual storage containers and widths. It must fail if a bank
is added, omitted, shadowed by an authoritative host field, or merely replaced
by a literal total.

## 4. Independent strict decode and control

The production package owns an independent decoder. Six opcode bits feed a
gate-built one-hot 6-to-64 network. Gate decoders then classify extension,
register, immediate, condition, T/F, nullify, branch-form, width, load/store,
and reserved-field wires. The admitted primary lines are exactly:

- `0x00` for exact `BREAK 0,0`;
- `0x02` for the admitted ALU extensions and required-zero fields;
- `0x08`, `0x0a`, and `0x0d` for LDIL, ADDIL, and LDO;
- `0x10..=0x12` and `0x18..=0x1a` for long-displacement loads/stores;
- `0x20..=0x23` and `0x28..=0x2b` for compare/add branches;
- `0x34` for admitted immediate SHD; and
- `0x3a` for exact BL, BLR, and BV extension forms.

For primary `0x02`, independent six-bit extension lines admit ANDCM `0x00`,
AND `0x08`, OR `0x09`, XOR `0x0a`, SUB `0x10`, and ADD `0x18`, only with the
Spec 07au reserved fields clear. Primary `0x3a` admits extension 0 BL and exact
extension 2/6 BLR/BV forms with their required-zero fields. All 64 primary
values, every admitted extension, and every adjacent reserved value must have
an explicit fail-closed classification.

One settled decode record is the sole authority for instruction legality,
family selection, operands, trace mnemonic, memory width/direction, branch
kind, and write enables. Host `match`, equality, ranges, Boolean comparisons,
or a second decoder may not independently authorize an architectural action.
Host control flow may dispatch only on the already-settled one-hot lines and
may package a typed result; it may not reinterpret the instruction word.

The decoder must classify a live delayed control transfer before any ordinary
family semantics. A nullified instruction still performs the checked four-byte
fetch and emits `NULLIFIED`, but decode and data-side effects remain disabled.
The gate implementation preserves the functional oracle's exact precedence
among halted, fetch, delay-slot, decode, data, allocation, and run-limit
failures. An installed-image target failure remains deferred until that queue
entry reaches the front and fetch is attempted.

## 5. Gate-derived datapath

All bit vectors use one documented ordering consistently. Conversion helpers
are transport only; they may not decide architectural arithmetic or control.

### 5.1 Register selection and Boolean operations

- Each source register uses a 32-to-1 mux tree for every bit. GR0 selection is
  AND-masked to constant zero.
- Destination selection uses a one-to-32 demultiplexed write-enable network;
  the GR0 enable is gate-disabled.
- AND, ANDCM, OR, and XOR apply repository Boolean primitives to every result
  bit. A one-hot operation mux selects the committed result.
- Every destination DFF receives the selected old/new input through a write
  mux and changes only on the shared commit edge.

### 5.2 Add, subtract, immediates, and C/B state

Repository ripple-carry networks derive 32-bit ADD, SUB, ADDIL, LDO, effective
addresses, add-branch sums, sequential queue increments, links, and branch
targets. Subtraction uses inversion plus carry-in rather than native host
subtraction. ADD and SUB expose the carry out of every four-bit digit. Gate
wiring reverses the little-endian ripple positions into architectural
C/B{0}..C/B{7}; subtraction records one for no borrow and zero for borrow.
Logical operations retain the eight C/B DFFs through muxes. ADDB/ADDIB use
their sum carry/overflow wires for condition selection but, matching the
manual operations admitted by Spec 07au, do not clock new C/B state.

Low-sign immediates are assembled by wires: encoded bit zero becomes the sign
bit and encoded bits N-1:1 become magnitude bits N-2:0 before a gate-derived
sign extension. LDIL/ADDIL `assemble_21`, branch `assemble_12`, and branch
`assemble_17` are fixed bit-permutation networks. Native signed casts, shifts,
or arithmetic must not substitute for those networks.

### 5.3 SHD, comparison, and branch conditions

Immediate SHD forms its documented 64-bit concatenation and passes it through
a fixed five-stage mux/barrel network. Counts 0 and 31 are explicit boundaries.

Equality is a result-zero reduction. For compare/subtract conditions, selectors
0 through 7 are exactly false, zero, `N XOR V`, zero OR `N XOR V`, borrow,
zero OR borrow, signed overflow, and result bit zero. For add conditions they
are exactly false, zero, `N XOR V`, zero OR `N XOR V`, NOT carry (no unsigned
overflow), zero OR NOT carry, signed overflow, and result bit zero. These wires
implement the manual's never, equal, signed less-than/less-or-equal,
unsigned-less/no-unsigned-overflow pair, overflow, and odd relations. A one-hot
condition mux selects the line; the T/F sense is a final XOR/not stage. No host
`Ordering`, signed comparison, wide addition, or Boolean predicate may decide
branch outcome, writeback, C/B, or nullification.

### 5.4 Big-endian memory

Ripple addition computes the effective address. Gate reductions establish
natural alignment and prove the complete 1/2/4-byte span is inside 64 KiB
before any read result or byte write-enable can commit. Byte lanes assemble and
disassemble big-endian halfwords and words through explicit wiring. A width
decoder and load/store line gate every lane enable. Loads zero-extend through
constant wires; stores select the low source bits. Failed access validation
must leave memory, registers, queues, flags, and lifecycle state unchanged.

Host indexing may address a byte after the gate address/range result settles;
it must not replace address arithmetic, alignment, range, width, endian, or
write-enable logic.

## 6. IAOQ, delay, nullification, and halt control

Queue transition is an explicit mux network over old `pc`, old `next_pc`, the
sequential `old_next_pc + 4`, and the branch target:

- ordinary execution clocks `(pc, next_pc)` to
  `(old_next_pc, old_next_pc + 4)`;
- a taken conditional transfer clocks the back queue to
  `old_pc + 8 + displacement`, while an untaken transfer selects sequential;
- BL/BLR write `old_next_pc + 4` through the GPR write network;
- BV forms the base-plus-index address through ripple/shift wiring and gate-
  clears its two bounded-model privilege bits; and
- every control transfer clocks the live-delay marker for the instruction now
  at the front.

For conditional `,n`, a sign wire from the assembled displacement and the
taken wire implement `(forward AND taken) OR (backward AND NOT taken)`.
Unconditional BL/BLR/BV `,n` selects nullification regardless of direction.
A nullified step clocks only the queue/nullify/delay transition required by
Spec 07au. The next normal step clears the live-delay marker unless it attempts
an excluded nested delayed transfer, which fails atomically.

Exact `BREAK 0,0` clocks the halt DFF after the ordinary queue transition.
Every public halt observation, post-halt error, run termination, trace, and
snapshot reads halt Q. No host Boolean mirror may decide behavior.

## 7. Stable-Q commit and checked lifecycle

`pa-risc11-gatelevel` exposes a checked API corresponding to
`PaRisc11Simulator`: reset construction, immutable complete state,
fallible snapshot, validated restore, image load, atomic step, bounded run, and
atomic load-and-execute. Public state/error/trace/result values must compare
directly with the functional oracle.

The lifecycle protocol is:

1. validate caller arguments and the complete entry Q state;
2. allocate every required memory/state snapshot, candidate DFF image,
   retained-trace vector, and final result state before its associated commit;
3. fetch exact bytes and settle decode, datapath, validation, and next-state
   wires without changing Q;
4. construct the complete trace from fetched bytes and settled wires;
5. clock all selected DFF inputs exactly once; and
6. expose the committed state by projecting stable Q.

Every allocation maps to `AllocationFailed`. Deterministic test-only seams,
inert in production, must cover state/memory cloning, load storage, candidate
construction, before/after snapshots, retained-trace storage, and final
execution-result state. Tests force every real allocation boundary before
commit and after at least one successful instruction in both `run` and
`execute`. The complete call-entry Q state must be restored without
allocation.

Load and restore validate memory length, GR0, queue alignment/range, empty-
image invariants, image alignment/range, nullify/delay relation, and represent-
ability of the 17-bit installed length before clocking a bit. A failed step
leaves every DFF unchanged. A failed run or execute restores every DFF to the
call-entry Q image even after partial progress. Trace retention is limited to
64 complete snapshots and preflighted. A post-halt step returns `Halted`
without fetching or clocking.

The implementation may cache projected host values for inspection, but they
are observers only. Test-only Q injection must prove that snapshot, execution,
run termination, faults, and results follow stable Q. If any host observer is
stored, two-way divergence tests must additionally corrupt the observer while
Q remains correct and corrupt Q while the observer remains stale; behavior
must follow Q in both directions and repair the observer after a successful
mutation.

## 8. Differential and exhaustive verification

The functional package is a test-only oracle. Every successful differential
compares exact raw bytes/word, uppercase mnemonic, nullified marker, both queue
values before and after, and the complete post-state: all 32 GPR slots, eight
C/B bits, memory bytes, nullify/delay/halt bits, and image metadata. Every
fault differential compares the exact typed error and verifies bit-for-bit
entry-state preservation in both machines.

Required independent matrices include:

1. all 64 primary opcode lines at image lengths 4 and 8, every admitted ALU and
   branch extension, required-zero/reserved-bit toggles, and neighboring
   unsupported forms;
2. every included family with GR0 as each source/destination position and
   overlapping source/destination registers;
3. exhaustive four-bit adder/subtractor input and carry-in relations at every
   nibble position, directed full-word carry/borrow chains, signed overflow,
   and exact C/B ordering;
4. all 32 SHD counts over zero, one-hot, sign-bit, alternating, and all-ones
   operands;
5. low-sign 5/14-bit and assembled 12/17/21-bit positive, negative, minimum,
   maximum, and every permuted source-bit position;
6. all eight compare and add conditions, T and F senses, register and immediate
   forms, true and false outcomes, forward and backward displacements, `n`
   clear/set, and resulting queue/nullification state;
7. ordinary, taken, untaken, call, computed, return, delay, nullified, nested-
   delayed rejection, terminal fallthrough, and out-of-image target paths;
8. every memory width at aligned, last-valid, misaligned, and first-invalid
   addresses, every byte lane, big-endian load/store round trips, and store
   rollback; and
9. new/reset, arbitrary-origin load, complete restore, malformed restore,
   snapshot, halt, already-halted run, step limit, trace limit, allocation
   seams, second-step rollback, and execute rollback.

Randomized deterministic seeds may supplement these matrices but cannot
replace them. Expected values for primitive and classification tests must be
independent literal/truth-table calculations, not calls into either production
engine.

## 9. Structural anti-bypass contract

Runtime differential testing is necessary but insufficient. A parsed-syntax
validator must lock the complete production transition chain:

- exact external production module declarations and allowed imports;
- opcode/extension decoders and their one-hot consumers;
- register mux/demux, Boolean ALU, ripple add/subtract and C/B wiring;
- immediate permutation/sign extension, SHD stages, condition selection,
  address/range/endian logic, queue/nullification/halt control;
- memory/register/lifecycle DFF read, write, clock, snapshot, commit, and
  rollback helpers; and
- top-level `load`, `restore`, `step`, `run`, and `execute` routing, including
  trace/final-state construction after stable Q.

The validator must use parsed Rust structure plus token-normalized exact-body
or data-flow locks where meaning matters. Text presence, call counts, comments,
dead anchors, or a validator that invokes the same helper it protects are not
evidence.

Hostile tests copy the **entire compilable crate** to a temporary directory,
apply one mutation anchored exactly once, prove the mutated crate passes
`cargo check`, and only then require the structural validator to reject it.
The matrix must include direct native arithmetic/shift/comparison/bitwise
substitution; direct opcode `match`; a second decoder; helper indirection;
shadowed gate imports; duplicated functions; dead gate anchors; inactive
decoys; direct stable-Q or packed-memory assignment; direct host-mirror reads;
removed/constant write enables; post-decode and post-datapath overwrites;
post-commit candidate replacement; trace/final-state construction from a host
candidate; and inline, attributed, aliased, or macro-generated module-routing
confusers. String, comment, macro-discarded, and character-literal lookalikes
must neither satisfy nor confuse the validator.

Production-source absence checks must reject calls to functional-oracle
execution paths, native architectural shortcuts named above, `unsafe`, and
panic/unchecked indexing on caller-controlled state. They supplement, rather
than replace, exact positive data-flow locks.

## 10. Exclusions

The exclusions are exactly Spec 07au's exclusions: remaining PA-RISC 1.1
problem-state families, privileged operations, spaces and control registers,
traps/interruption entry, protection IDs, MMU/TLB, caches, synchronization,
coprocessors/FPU, external buses, multiprocessor ordering, diagnostics, and
nested architectural delayed transfers. This cell also excludes pipeline
stages, hazards, forwarding, cache or bus cycles, clock/reset trees, physical
fan-out, propagation delay, metastability, power, placement, semiconductor
devices, and SPICE behavior.

No documentation may describe this as a complete PA-RISC ISA, a particular
HP processor, a silicon reconstruction, or a transistor-accurate model.

## 11. Documentation and completion gate

Implementation is complete only when all of the following are true:

1. the package is `code/packages/rust/pa-risc11-gatelevel`, is registered in
   the Rust workspace and repository BUILD graph, and has README, changelog,
   BUILD, and source-level teaching documentation;
2. `FLIP_FLOP_COUNT` and a non-tautological inventory test prove exactly
   525,436 authoritative stable-Q bits;
3. the independent decoder, gate datapath/control, stable-Q commit chain,
   allocation seams, full-crate hostile fixtures, and complete differential
   matrices above pass;
4. the functional package remains a development/test oracle and production
   gate source contains no oracle transition call path;
5. package-scoped Rustfmt, strict Clippy with warnings denied, rustdoc with
   warnings denied, release tests, package BUILD, lessons validation, and the
   repository affected-build plan pass; and
6. measured all-target package line coverage exceeds 80 percent, with the
   exact numerator, denominator, percentage, and test counts recorded only
   after measurement in README, changelog, Spec 07au2, and the live backlog.

Independent security/correctness review is mandatory before publication.

The completed local package has 25 release tests and measured all-target line
coverage of 97.65% (1,040/1,065 lines). The recorded evidence includes the exact
inventory, full classification and condition matrices, complete-state
differentials, lifecycle/allocation rollback, stable-Q divergence, and
compiler-checked hostile full-crate mutations required above.
