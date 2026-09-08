# Rust CPU Simulator Backlog

**Status:** active

**Scope:** functional and gate-level Rust simulators for every CPU target in the
07-series, followed by a complete cross-language port wave
**Last reprioritized:** 2026-08-28

## Definition of the matrix

The architecture inventory is the CPU portion of the 07-series roadmap. It
contains every named physical CPU/ISA target from 07a through 07z and excludes
07c (WebAssembly), which is a virtual instruction set rather than a CPU. The
07b educational ARMv7 lane and the later 07x ARMv7-A/Thumb-2 lane remain
separate deliverables because they have different scope and public contracts.

Each target needs two Rust packages:

1. A **functional simulator** that performs instruction-level state changes
   with host-language operations.
2. A **gate-level simulator** whose data-path arithmetic and logic flow through
   the repository's Rust logic-gate and arithmetic primitives. Host integers
   may still be used for control flow, addresses, and trace bookkeeping. This
   is an ISA-level educational gate model, not a transistor-accurate die model.

An existing package is not automatically complete. Every existing Rust package
has a conformance/completeness audit in the backlog. An audit must compare the
package with its architecture spec, document the supported ISA surface, add
missing tests, and either close the row or add concrete follow-up items.

## Completion contract

A matrix cell is complete only when all of the following are true:

- The architecture and supported instruction surface are specified.
- Reset, load, single-step, bounded execution, and immutable state inspection
  are deterministic and tested.
- Invalid programs and out-of-range memory operations fail closed without
  panics or unbounded execution.
- Instruction families and architectural edge cases have tests, with at least
  80% coverage and a target of 95%.
- The package has a working `BUILD` recipe, is included in the Rust workspace,
  passes formatting and Clippy with warnings denied, and is documented in its
  README and changelog.
- A gate-level package has differential tests against its functional partner.

## Prioritization policy

The queue is recalculated after every merged pull request and whenever new work
is discovered. The ordering rules are:

1. Security, data-corruption, broken-main, and CI-blocking work comes first.
2. Otherwise proceed chronologically by CPU year, oldest first.
3. Within an architecture, finish or audit the functional simulator before the
   gate-level simulator so the latter has a behavioral oracle.
4. A discovered prerequisite is recorded immediately and placed before the
   item it blocks; non-blocking follow-up work is ranked by severity and age.
5. The cross-language wave begins only after every Rust matrix cell is complete.

## Rust coverage matrix and ordered queue

`Missing` means no matching Rust package exists. `Audit` means a package exists
but has not yet passed the completion contract above. Backlog IDs are ordered
according to the current prioritization run.

| IDs (functional / gate) | Year | Target | Functional Rust | Gate-level Rust |
|---|---:|---|---|---|
| RCPU-001 / RCPU-002 | 1948 | Manchester Baby (SSEM) | Complete: `manchester-baby-simulator` | Complete: `manchester-baby-gatelevel` |
| RCPU-003 / RCPU-004 | 1954 | IBM 704 | Complete: `ibm704-simulator` | Complete: `ibm704-gatelevel` |
| RCPU-005 / RCPU-006 | 1961 | GE-225 | Complete: `ge225-simulator` | Complete: `ge225-gatelevel` |
| RCPU-007 / RCPU-008 | 1964 | CDC 6600 | Complete: `cdc6600-simulator` | Complete: `cdc6600-gatelevel` |
| RCPU-009 / RCPU-010 | 1970 | DEC PDP-11 | Complete: `pdp11-simulator` | Complete: `pdp11-gatelevel` |
| RCPU-011 / RCPU-012 | 1971 | Intel 4004 | Complete: `intel4004-simulator` | Complete: `intel4004-gatelevel` |
| RCPU-013 / RCPU-014 | 1972 | Intel 8008 | Complete: `intel8008-simulator` | Complete: `intel8008-gatelevel` |
| RCPU-015 / RCPU-016 | 1974 | Intel 8080 | Complete: `intel8080-simulator` | Complete: `intel8080-gatelevel` |
| RCPU-017 / RCPU-018 | 1975 | MOS 6502 | Complete: `mos6502-simulator` | Complete: `mos6502-gatelevel` |
| RCPU-019 / RCPU-020 | 1976 | Zilog Z80 | Complete: `z80-simulator` | Complete: `z80-gatelevel` |
| RCPU-021 / RCPU-022 | 1978 | Intel 8086 | Complete: `intel8086-simulator` | Complete: `intel8086-gatelevel` |
| RCPU-023 / RCPU-024 | 1979 | Motorola 68000 | Complete: `m68k-simulator` | Complete: `motorola68k-gatelevel` |
| RCPU-025 / RCPU-026 | 1980 | Intel 8051 | Complete: `intel8051-simulator` | Complete: `intel8051-gatelevel` |
| RCPU-027 / RCPU-028 | 1985 | ARM1 / ARMv1 | Complete: `arm1-simulator` | Complete: `arm1-gatelevel` |
| RCPU-029 / RCPU-030 | 1985 | MIPS R2000 | Complete: `mips-r2000-simulator` | Complete: `mips-r2000-gatelevel` |
| RCPU-031 / RCPU-032 | 1987 | SPARC V8 | Complete: `sparc-v8-simulator` | Complete: `sparc-v8-gatelevel` |
| RCPU-033 / RCPU-034 | 1992 | DEC Alpha AXP 21064 | Complete: `alpha-axp-simulator` | Complete: `alpha-axp-gatelevel` |
| RCPU-035 / RCPU-036 | 1992 | PowerPC 601 | Complete: `powerpc601-simulator` | Complete: `powerpc601-gatelevel` |
| RCPU-037 / RCPU-038 | 2003 | x86-64 (AMD64) | Complete: `x86-simulator` | Complete: `x86-64-gatelevel` |
| RCPU-039 / RCPU-040 | 2004 | ARMv7 educational baseline (07b) | Complete: `arm-simulator` | Missing |
| RCPU-041 / RCPU-042 | 2004 | ARMv7-A / Thumb-2 (07x) | Missing | Missing |
| RCPU-043 / RCPU-044 | 2010 | RISC-V RV32I (07a) | Audit: `riscv-simulator` | Missing |
| RCPU-045 / RCPU-046 | 2010 | RISC-V RV64I + M | Missing | Missing |
| RCPU-047 / RCPU-048 | 2011 | AArch64 (ARMv8-A) | Missing | Missing |
| RCPU-049 / RCPU-050 | 2020 | Apple M1 (AArch64 + NEON) | Missing | Missing |

Current selection: **RCPU-040**, the ARMv7 educational gate-level simulator.
RCPU-039 completed the documented Spec 07b functional surface and advances to
its dependency partner while completed earlier cells publish one at a time.
RCPU-005 is complete after its AAU/final-audit slice added separate
40-bit AX/BX/QX/IX state, all three calculation modes, exact general/arithmetic/
data-transfer and plug-7 status words, deterministic integer floating-point,
transient/hold alerts, modification, and fail-closed preflight. The functional
oracle has 91 simulator tests and 88.81% core line coverage (2,152/2,423), above
the 80% completion floor.

RCPU-006 is split into gate-auditable slices. P006A establishes flip-flop
memory/registers, gate decode, the 20-bit and one-sign-plus-38-data-bit central
binary datapaths, modification, control flow, shifts, compares, and lifecycle
differentials. P006B1 adds central decimal/clock state. P006B2 adds direct I/O,
and P006B3 adds controller-selector and API signals/state. P006C adds the separate AAU
register file and fixed/normalized/unnormalized datapaths, then runs the final
instruction-family differential and coverage audit. The chronological queue
advances to the CDC 6600 only after the ordered P006C publication closes RCPU-006.

P006A merged in PR #13330: its 17 tests cover gate-backed
lifecycle, one-hot fixed/opcode decode, core-memory X groups, automatic
modification, all central single/double binary operations, multiply/divide,
all twelve shift/normalize paths, `MOV`, and atomic bounds failures. Core line
coverage was 86.11% (682/792), above the completion floor. P006B1 merged in PR
#13339 at `8992e9b`: its
acceptance boundary is DFF-backed decimal mode/carry and 19-bit clock state,
gate-only single/double BCD arithmetic and clock advancement, exact fixed-word
decode, oracle differentials, fail-closed validation, and above-floor coverage.
Its 23 combined tests include 48 seeded decimal vectors; core line coverage is
89.91% (1,257/1,398). P006B2 merged in PR #13349 at `709a6fb`: its 53 new
DFF-backed state bits and ten direct-I/O conformance tests spanning all seven
card modes, continuous DMA slots and sync/status words, shared N-device routing,
readiness branches, parity/overrun/priority alarms, atomic failures, and an
instruction-sequence differential against the functional oracle. The 33 combined
tests cover 83.48% of core lines (1,339/1,604), above the completion floor.
P006B3 merged in PR #13357 at `72e33f4`: its 1,085 additional DFF-backed bits
model eight controller banks, selector/API latches, group-32 interrupt state,
and widened X-group selection. Thirteen new
tests cover selector/status formats, modification, bounded opaque command
capture, condition branches, disabled/deferred ready events, card participation,
group-32 vector/return, SET PST/SET PBK, BRU target inhibition, reset, functional
lockstep, and atomic memory edges. The 46 combined tests cover 84.92% of core
lines (1,650/1,943), above the completion floor. P006C merged in PR #13365 at
`ed442aa`: 167 additional DFF-backed bits model separate 40-bit
AX/BX/QX/IX registers, mode/readiness, and four alert latches. Fourteen new
functional-oracle tests cover all AAU memory/general/status families, fixed and
floating arithmetic, normalization, signed/exponent edges, modification, odd
memory rules, reset, and atomic boundaries. The 60 combined tests cover 85.61%
of core lines (2,190/2,558), above the completion floor. Its green final build
audit closes RCPU-006.

RCPU-007 merged in PR #13377 at `f540399`. Its 17 integration
tests cover the repository's complete 22-short/14-long instruction surface,
four big-endian 15-bit parcels per 60-bit word, all eight X/A/B registers with
hardwired B0, 4,096 bounded memory words, parcel branches and subroutines,
exact 60/18-bit wrap and signed comparisons, program encoders, immutable
snapshots, atomic fetch/branch/memory errors, bounded execution, and seeded
Python-oracle vectors. Core line coverage is 90.76% (324/357), above the
completion floor. Its green required CI and squash auto-merge close the
functional cell and unblock RCPU-008.

RCPU-008 merged in PR #13391 at `4e56a500`. Its normative 07t2 contract and
`cdc6600-gatelevel` package put 246,529 persistent core-memory,
X/A/B/P, and halt bits in D flip-flops while keeping B0 hardwired to zero. A
64-line one-hot decoder selects all 36 instructions; gate vectors implement
boolean logic, 60/18-bit ripple add/subtract, signed compare, six-stage barrel
shift, 60-stage partial-product multiply, address calculation, branch
predicates, and P updates. Two unit and nine integration tests cover complete
state/trace lockstep, seeded datapaths, all branch paths, exact topology,
transport and public bounds, atomic decode/fetch/branch/memory failures, and
bounded workloads. Core line coverage is 98.22% (441/449), above the completion
floor. Its green required CI and squash auto-merge close the gate-level cell
and advance the chronological queue to the PDP-11 functional oracle audit.

RCPU-009 merged in PR #13405 at `67098ffa`. Its Rust package ports the full
59-mnemonic-variant Spec 07o/Python-oracle surface: all eight
addressing modes, twelve double-operand variants, 25 single-operand variants,
15 branch conditions, and HALT/NOP/RTI/RTS/JMP/JSR/SOB control over eight
16-bit registers, NZVC, and 64 KiB little-endian memory. Seven integration
tests cover every decode family, every addressing mode, byte/SP/PC stepping,
word/byte result and flag edges, branch paths, subroutine/interrupt stacks,
typed atomic failures, bounded loops, and three multi-instruction Python-oracle
workloads. Core line coverage is 92.98% (477/513), above the completion floor.
Its green required CI and squash auto-merge close the functional cell and
unblock its gate-level partner.

RCPU-010 merged in PR #13422 at `e054ea4bf2`. Its normative 07o2
contract and `pdp11-gatelevel` package put all 524,433 persistent memory,
register, PSW, and halt bits in D flip-flops. Gate networks provide complete
instruction decode, 8/16-bit arithmetic and logic, NZVC updates, effective
address side effects, branch predicates and offsets, stack/PC paths, and byte
sign extension across the same 59 mnemonic variants and all eight addressing
modes as the functional oracle. One unit and seven integration tests cover every
instruction and addressing family, all 15 taken and fall-through branch paths,
seeded workloads, calls/returns/interrupt stacks, exact topology, and atomic
failure boundaries. Core line coverage is 95.13% (644/677), above the completion
floor. Its green required CI and squash auto-merge close the PDP-11 pair and
advance the chronological queue to the Intel 4004 functional audit.

RCPU-011 merged through PR #13431 as `565d68071e`. The functional crate
already implemented the complete specified 46-instruction surface, including
all register/pair, control, three-level stack, BCD, RAM/status, and port paths.
The audit replaces panic-prone loading and stepping with typed checked results,
adds immutable owned snapshots of all state, rejects oversized programs,
truncated or illegal instructions, invalid direct/indirect targets, halted
execution, and corrupted public legacy selectors atomically, and preserves the
Nib compiler consumer. Its 82 unit and five integration tests cover reset,
load, bounded execution, every instruction family, full state, and failure
boundaries. Core line coverage is 98.30% (463/471), above the completion target,
and its green squash merge unblocked the gate-level partner audit.

RCPU-012 merged through PR #13437 as `623f70e7d9`. The existing package already
had gate ALU, decode, PC, register, stack, and RAM components plus 88 unit tests.
The audit moves the remaining RAM outputs, selectors, ROM port, halt state, and
stack pointer into D flip-flops for an exact 1,428 persistent bits; fixes SRC's
register-selector width; adds typed atomic load/fetch/decode/halt failures and a
full owned snapshot; and replaces the stale Python-oriented 07d2 text with a
normative Rust contract. Four integration tests differentially cover all 254
specified first-byte encodings plus memory/status/ports, indirect paths,
branches, stack, reset, topology, and failure boundaries against the functional
oracle. Core line coverage is 95.15% (686/721), above the completion target.
Its green squash merge closed the Intel 4004 pair and advanced to the Intel
8008 audit.

RCPU-013 is audit-complete locally atop RCPU-012. The existing functional crate
already implemented the specified instruction families, with 31 legacy unit
tests, but its public boundary could panic on invalid ports and out-of-range
loads, silently truncated oversized images, returned string execution errors,
allowed operand fetch to wrap at the end of memory, swallowed run errors, lacked
a complete state snapshot, failed strict rustdoc, and covered only 79.14% of its
core. The audit adds typed atomic lifecycle and port errors, deterministic
checked runs, a complete owned 16 KiB/stack/register/flag/I/O snapshot, and five
integration tests that classify all 256 first-byte encodings and pin invalid
load, port, opcode, end-of-memory, halt, reset, ownership, and repeatability
boundaries. All 36 simulator tests pass strict Clippy and rustdoc; core line
coverage is 89.46% (348/389), above the completion floor. Publication closes
RCPU-013 and unblocks the existing Intel 8008 gate-level partner audit.

RCPU-014 is audit-complete locally atop RCPU-013. The legacy package had a
gate-backed ALU, decoder, register file, stack, and PC plus 33 unit tests, but
kept its 16 KiB memory, flags, halt state, I/O latches, and stack depth in host
state; allocated a fictitious eighth register for M; performed INR/DCR and zero
detection with host operations; clamped invalid ports; truncated or panicked on
invalid loads; wrapped operand fetch at memory end; swallowed run failures;
exposed no full snapshot; and failed strict rustdoc on 34 links. The audit puts
all 131,504 persistent bits in D flip-flops, removes M's fictitious storage,
routes increment/decrement and zero through gates, shares the functional
oracle's typed atomic lifecycle and owned state, and replaces the stale Python/
host-memory specification with a normative Rust contract. Thirty-five unit and
five integration tests classify every first-byte encoding, compare complete
traces and 16 KiB states, cover multi-instruction memory/control/I/O workloads,
pin topology and atomic boundaries, and pass strict formatting, Clippy, and
rustdoc. Core line coverage is 99.82% (548/549), above the completion target.
Publication closes the Intel 8008 pair and advances to the Intel 8080 audit.

RCPU-015 is audit-complete locally atop RCPU-014. The legacy Rust package
implemented the 244 defined opcodes and passed 41 unit tests, but silently
truncated oversized program loads, converted undefined opcodes into HLT, could
silently ignore data and stack accesses outside caller-configured short memory,
returned no typed lifecycle errors, exposed no complete owned snapshot or
before/after trace, and failed strict rustdoc. The audit adds typed atomic
load/fetch/data-access/run boundaries, transactional bounded program execution,
complete owned register/flag/memory/control/I/O snapshots, and full traces.
Forty-one unit and four integration tests classify all 256 first bytes and pin
the full final state of every one of the 244 defined opcodes to a hash generated
by the repository's Python oracle. All tests plus the doctest pass formatting,
strict Clippy, and strict rustdoc; core line coverage is 92.68% (671/724), above
the completion floor. Publication advances to the Intel 8080 gate audit.

RCPU-016 is audit-complete locally atop RCPU-015. The legacy package covered
the instruction families but stored memory, flags, control state, ports, and
the register arrays as host values; used host arithmetic in several datapaths;
had unchecked lifecycle boundaries; exposed incomplete state; failed strict
rustdoc; and routed XTHL, PCHL, XCHG, and SPHL through an unreachable decoder
arm. The audit puts all 528,479 persistent bits in D flip-flops, routes the
remaining arithmetic and flag paths through gates, shares the functional
simulator's typed atomic lifecycle and owned state contract, and fixes the
decoder routing. Fifty-one unit, four integration, and five documentation tests
pin the exact topology, all failure boundaries, multi-instruction workloads,
and full-state equivalence for all 244 defined opcodes. Publication closes the
Intel 8080 pair and advances to the MOS 6502 functional audit. Core line
coverage is 95.53% (855/895), above the completion floor.

RCPU-017 is audit-complete locally atop RCPU-016. The legacy package already
implemented all 151 official NMOS 6502 opcodes and 13 addressing modes, but
converted undefined opcodes into halt, exposed no full owned snapshot or
before/after trace, silently accepted unchecked lifecycle boundaries, omitted
the Python reference's memory-mapped I/O, and failed strict rustdoc. The audit
adds typed atomic failures, transactional bounded runs, complete register/flag/
memory/control/I/O state, 240 input and output latches, and full traces. Sixty
unit, three integration, and one documentation test classify all 256 first
bytes and pin the complete post-state of all 151 official opcodes to hashes
generated from the repository's Python oracle. Strict formatting, Clippy, and
rustdoc pass; core line coverage is 93.87% (704/750), above the completion
floor. Publication advances to the MOS 6502 gate audit.

RCPU-018 is audit-complete locally atop RCPU-017. The legacy package decoded
all 151 official opcodes, but most persistent state used host values, lifecycle
boundaries were unchecked, snapshots were incomplete, zero detection used host
iteration, decimal subtraction disagreed with the functional oracle, and
strict rustdoc failed. The audit backs all 528,184 persistent memory, register,
flag, halt, and I/O bits with D flip-flops; shares the functional simulator's
typed transactional lifecycle, full owned state, traces, and results; and
routes corrected decimal adjustment and zero detection through gates.
Eighty-one unit, five integration, and seven documentation tests pin the exact
topology, every failure boundary, IRQ/NMI entry, multi-instruction workloads,
memory-mapped I/O, full-state equivalence for all 151 official opcodes, and
atomic rejection of all 105 undefined bytes. Strict formatting, Clippy, and
rustdoc pass; core line coverage is 96.60% (994/1,029), above the completion
floor. Publication closes the MOS 6502 pair and advances to the Zilog Z80
functional audit.

RCPU-019 is audit-complete locally atop RCPU-018. The three internal slices
replace the variable-memory/string lifecycle with an architectural 64 KiB
machine, typed atomic failures, complete owned state/traces, transactional
bounded runs, checked ports, and maskable/NMI interrupt entry; complete the
ED arithmetic/load/special-register/interrupt/block families; and complete
DD/FD direct, displacement, arithmetic, stack, and branch forms plus every
DDCB/FDCB indexed rotate/shift/bit/set/reset form. A deterministic full-state
Python differential covers 1,160 defined vectors across the base, CB, ED, DD,
FD, DDCB, and FDCB spaces, hashing registers, flags, all 64 KiB of memory, and
both 256-port banks. Sixty unit, five integration, one documentation, and 16
backend-consumer tests pass. Strict formatting, Clippy, and rustdoc pass; core
line coverage is 98.49% (1,366/1,387), above the completion floor. Publication
advances to the Z80 gate audit.

RCPU-020 is audit-complete locally atop RCPU-019. The legacy gate ALU already
covered most execution semantics, but all claimed persistent hardware was host
state and its lifecycle silently accepted partial failures. The audit replaces
memory, both register banks, IX/IY, PC/SP, I/R, interrupt/halt state, and both
port banks with an exact 528,597-DFF topology; shares the functional typed
transactional API and full owned state/traces; completes RLD/RRD and all eight
block-I/O operations; adds interrupt delivery; and publishes normative Spec
07k2. A 1,160-vector Python full-state oracle covers every defined base, CB, ED,
DD, FD, DDCB, and FDCB encoding. Fifty-eight unit, four integration, and eight
documentation tests pass. Strict formatting, Clippy, and rustdoc pass; core
line coverage is 97.64% (1,489/1,525), above the completion floor. Publication
closes the Z80 pair and advances to the Intel 8086 functional audit.

RCPU-021 is audit-complete locally atop RCPU-020. The legacy Rust package had
61 passing tests but deliberately implemented only register-immediate and
register-only ModRM data/ALU operations, INC/DEC, NOP, and HLT. It rejected all
memory operands and omitted prefixes, stack/control flow, strings, interrupts,
shifts, multiply/divide, BCD adjustment, segment-register operations, and I/O;
memory size was caller-selected and lifecycle failures were unchecked. The
audit completes the Python oracle's specified instruction surface and all 24
ModRM effective-address forms; fixes memory at the architectural 1 MiB; adds
complete state, traces, port banks, typed atomic failures, and transactional
runs; and retains the legacy API for consumers. A reproducible 461-vector
Python differential classifies every first byte, all dense group extensions,
all memory-addressing modes, and focused prefix/string/control/stack/I/O cases,
comparing every register/flag and hashes of full memory and both port banks.
Sixty-one unit, seven lifecycle, one aggregate differential, and documentation
tests pass with strict formatting, Clippy, and rustdoc. Core line coverage is
95.17% (1,340/1,408), above the completion target. Publication advances to the
Intel 8086 gate audit.

RCPU-022 is audit-complete locally atop RCPU-021. The audit replaces host-backed
memory, register, flag, halt, and port state with an exact 8,392,922-DFF
topology; shares the functional simulator's complete owned state, typed atomic
lifecycle, trace, and bounded-run contract; and replaces host multiply/divide
with fixed partial-product and restoring gate networks. The functional oracle's
reproducible 461-vector full-state fixture covers every first byte, every dense
group extension, all effective-address forms, prefixes, strings, control flow,
stack, and I/O. It exposed and pinned LES/LDS register-form behavior, repository
INT halt boundaries, multi-count overflow flags, and invalid group extensions.
Sixty unit, six lifecycle, one aggregate differential, and fourteen
documentation tests pass with strict formatting, Clippy, and rustdoc. CPU-core
line coverage is 95.67% (1,481/1,548); total package coverage is 94.41%, well
above the completion floor. Publication closes the Intel 8086 pair and advances
to the Motorola 68000 functional audit.

RCPU-023 is audit-complete locally atop RCPU-022. The audit completes indexed
and both PC-relative effective addressing plus every previously deferred Spec
07n immediate/bit, NEGX/PEA/SR/CCR, multiply/divide, ADDX/SUBX, and memory
shift/rotate family. The checked constructor now models the exact 16 MiB
big-endian machine, while complete owned state/traces and typed atomic
load/restore/step/run errors close the silent bounds and partial-mutation gaps;
legacy caller-sized APIs remain for consumers. A reproducible 82-vector Python
full-state differential covers every addressing form and decode line, edge
flags, stack/control, and division failures. Fifty-seven unit, nine lifecycle,
one aggregate differential, and one documentation test pass; the 225-test
Python oracle remains green at 92.61%. Sixteen backend and six encoder tests
also pass with strict formatting, Clippy, and rustdoc. Total Rust line coverage
is 85.96% (1,488/1,731), with 80.24% in the instruction engine, above the
completion floor. Publication advances to the Motorola 68000 gate audit.

RCPU-024 is audit-complete locally atop RCPU-023. All 134,218,289 persistent
memory, D/A-register, PC, SR, and halt bits now have stable D-flip-flop identity;
packed stable-Q memory retains exact master/slave boundary state without a
512 MiB expansion. MULU/MULS use fixed 16×16 partial products and DIVU/DIVS use
a fixed 32÷16 restoring network. The gate CPU shares the functional simulator's
complete state, traces, results, and typed transactional lifecycle. Its original
67-test baseline is now 70 unit, four lifecycle, and one aggregate 82-vector
Python-oracle full-state differential tests. Strict formatting, Clippy, and
rustdoc pass. Total line coverage is 81.82% (2,210/2,701), with 81.89% in the
CPU engine and 95.71% in the state layer, above the completion floor.
Publication closes the Motorola 68000 pair and advances to Intel 8051.

RCPU-025 is audit-complete locally atop RCPU-024. The original complete
instruction dispatcher now exposes owned PC/IRAM/SFR/code/XDATA/halt state,
complete before/after traces and final results, deterministic checked loading,
and typed atomic load/restore/step/run failures. A reproducible Python generator
hashes complete Harvard state for every one of the 256 opcode bytes; the Rust
implementation matches all hashes. Thirty-five unit, five lifecycle, and one
aggregate differential test pass. Total Rust line coverage is 94.53%
(1,297/1,372), with 98.20% in the instruction engine. The 200-test Python oracle
remains green at 86.10%, and sixteen backend plus seven encoder tests pass with
strict formatting, Clippy, and rustdoc. Publication advances to the Intel 8051
gate-level audit.

RCPU-026 is audit-complete locally atop RCPU-025. The simulator now owns an
exact 1,050,641-DFF persistent topology across both 64 KiB Harvard banks,
IRAM/SFRs, PC, and halt. It shares the functional state/error/trace/result
contract, rejects checked lifecycle failures atomically, preserves CY for
RL/RR, preserves A/B on DIV-by-zero, and uses fixed partial-product and
restoring-divider networks. Seventy unit, six lifecycle, one aggregate
all-256-opcode full-state differential, and 23 doctests pass. Strict formatting,
Clippy, and rustdoc are green; total line coverage is 97.51% (1,876/1,924), with
97.01% in the CPU engine. Publication advances to the ARM1 functional audit.

RCPU-027 is audit-complete locally atop RCPU-026. The simulator now offers the
exact 64 MiB architectural machine while preserving caller-sized compatibility,
owns all 27 physical banked registers and memory in complete state snapshots,
implements force-user block transfers and mask-aware IRQ/FIQ entry, and exposes
deterministic typed transactional load/restore/step/run boundaries with complete
before/after traces and final results. A reproducible 599-vector Python
full-state corpus spans every condition, ALU operation, load/store and block
addressing variant, branches, software interrupts, and coprocessor behavior.
Fifty-one unit, nine lifecycle, one aggregate differential, and one documentation
test pass; the 21-test gate consumer and 143-test Python oracle also remain
green. Strict formatting, Clippy, and rustdoc pass. Total Rust line coverage is
92.29% (1,675/1,815), above the completion floor. Publication advances to the
ARM1 gate-level audit.

RCPU-028 is audit-complete locally atop RCPU-027. The simulator now owns the
exact 536,871,777-DFF persistent topology: 536,870,912 memory bits, 864 bits
across all 27 physical registers, and one halt bit. Packed stable-Q storage
clocks simulator-owned writes through the sequential-gate primitive without a
multi-gigabyte latch expansion. The gate machine shares the functional
complete state/error/trace/result contract, adds deterministic typed
transactional load/restore/step/run boundaries, checked direct access,
force-user block transfers, and mask-aware IRQ/FIQ entry. The original 21 unit
tests, eight lifecycle tests, and one aggregate 599-vector Python/functional
full-state differential pass. Strict formatting, Clippy, and rustdoc are green;
total line coverage is 93.12% (1,042/1,119). Normative Spec 07e2 replaces the
stale Python/25-register sketch. Publication closes the ARM1 pair and advances
to the MIPS R2000 functional audit.

RCPU-029 is audit-complete locally atop RCPU-028. The simulator now exposes the
exact 64 KiB Spec 07q machine, complete PC/GPR/HI/LO/memory/halt/load-range
state, deterministic checked loading and restore, typed direct access, complete
before/after traces and final results, and transactional step/run failures for
truncation, halt, alignment, BREAK, unknown instructions, signed overflow, and
division by zero. PC/effective addresses follow normative wrapping and memory
is big-endian. LWL/LWR/SWL/SWR close the functional gap with the gate partner.
A reproducible 218-vector Python corpus covers every Python decode/fault line
with full-state hashes; separate lifecycle coverage pins all four unaligned
merge operations. Thirty-two unit, six lifecycle, one aggregate differential,
and one doctest pass. The 130-test Python oracle and Rust gate consumer's 33
unit plus 21 doctests remain green. Strict formatting, Clippy, and rustdoc pass;
total Rust line coverage is 94.51% (1,481/1,567). Publication advances to the
MIPS R2000 gate-level audit.

RCPU-030 is audit-complete locally atop RCPU-029. The gate simulator now owns
the exact 525,409-DFF topology: 524,288 memory bits, 1,120 bits across all 32
GPRs plus HI/LO/PC, and one halt bit. Packed stable-Q storage clocks
simulator-owned memory, register, PC, and halt writes through the sequential
gate primitive. The machine shares the functional complete
state/error/trace/result contract, validated restoration, typed direct access,
deterministic checked loading, atomic one-step transitions, and transactional
bounded runs. All original 33 unit tests and 21 doctests, seven lifecycle
tests, and the aggregate 218-vector Python/functional full-state differential
pass. Strict formatting, Clippy, and rustdoc are green; total Rust line
coverage is 94.30% (1,539/1,632). New normative Spec 07q2 pins the topology,
gate networks, lifecycle, and conformance boundary. Publication closes the
MIPS R2000 pair and advances to the SPARC V8 functional audit.

RCPU-031 is audit-complete locally atop RCPU-030. The functional simulator now
owns the exact 64 KiB machine, PC/nPC pipeline, all 56 physical registers,
window depth, PSR/Y, loaded range, memory, and halt state through validated
restore, typed direct access, deterministic checked loading, complete atomic
step traces, and transactional bounded runs. The V8 manual-correct six-bit
multiply/divide-cc encodings replace the unreachable seven-bit Spec/Python
constants; divide-cc overflow and the `i64::MIN / -1` host-overflow edge are
pinned. All 52 unit tests, five lifecycle tests, the 248-vector Python
full-state decode/fault differential, and 21 encoder/backend consumer tests
pass. Strict formatting, Clippy, and rustdoc are green; total Rust line
coverage is 93.45% (827/885). Publication follows RCPU-030 and advances to the
SPARC V8 gate-level audit.

RCPU-032 is audit-complete locally atop RCPU-031. The gate simulator now owns
the exact 526,185-DFF topology: 524,288 memory bits, 1,792 bits across all 56
physical registers, 64 PC/nPC bits, 32 Y bits, four PSR bits, two CWP bits, two
window-depth bits, and one halt bit. Packed stable-Q storage clocks all
simulator-owned writes through the sequential gate primitive. Fixed-round
gate networks now implement unsigned and signed division without native host
division. The machine shares the completed functional lifecycle contract;
all 16 Bicc predicates, MULScc, RD `%y`, trap/fault atomicity, and documented
UDIVcc/SDIVcc saturation flags match the V8 manual and functional oracle. All
42 unit tests, six lifecycle suites, and the aggregate 248-vector
Python/functional full-state differential pass. Strict formatting, Clippy,
and rustdoc are green; total Rust line coverage is 94.72% (1,650/1,742). New
normative Spec 07r2 pins the topology, gate networks, lifecycle, and
conformance boundary. Publication follows RCPU-031 and advances to RCPU-033,
the DEC Alpha AXP 21064 functional implementation.

RCPU-033 is complete locally atop RCPU-032. The new Rust functional simulator
implements the complete Layer 07s integer surface over the exact 64 KiB
little-endian machine, all 32 64-bit GPRs with hardwired r31, PC/nPC, halt,
and installed-program lifecycle state. It adds public structured encoders,
validated complete restore, origin-aware deterministic loading, checked direct
register and byte/word/long/quad access, typed atomic instruction failures,
complete before/after traces, and transactional bounded runs. Three unit tests,
four lifecycle/workload tests, and the aggregate 624-vector Python full-state
decode/fault differential pass; the Python oracle's 146 tests remain green.
Strict formatting, Clippy, and rustdoc pass; total Rust line coverage is 88.08%
(532/604). Spec 07s now defines the normative Rust completion boundary. No
Alpha Rust encoder/backend consumer existed to validate. Publication follows
RCPU-032 and advances to RCPU-034, the Alpha gate-level implementation.

RCPU-034 is complete locally atop RCPU-033. The new gate simulator stores the
exact 526,465 persistent bits in D flip-flops: 524,288 memory, 2,048 GPR,
128 PC/nPC, and one halt bit. Combinational decode and 64/128-bit gate networks
cover the complete Spec 07s integer surface, including fixed-round
partial-product multiplication and UMULH. Five lifecycle/workload suites, two
gate-network tests, and the aggregate 624-vector Python full-state differential
pass. Strict formatting, Clippy, and rustdoc are green; total Rust line coverage
is 96.80% (726/750). Normative Spec 07s2 pins topology, gate boundaries,
lifecycle, and conformance. Publication follows RCPU-033 and advances to
RCPU-035, the PowerPC 601 functional implementation.

RCPU-035 is complete locally atop RCPU-034. The new functional simulator owns
the exact 64 KiB big-endian machine, all 32 GPRs, LR/CTR/XER/CR/CIA, halt, and
installed-program range. It implements the complete Layer 07u integer decode
surface, public structured encoders, validated restore and origin-aware load,
checked direct access, typed atomic faults, complete traces/results, and
transactional bounded runs. Six unit tests, four lifecycle/workload tests, and
the aggregate 239-vector Python full-state differential pass. Strict format,
Clippy, and rustdoc are green; total Rust line coverage is 89.83% (627/698).
Spec 07u now pins the normative Rust completion boundary. No PowerPC Rust
consumer existed. Publication follows RCPU-034 and advances to RCPU-036, the
PowerPC 601 gate-level implementation.

## Cross-language wave

After RCPU-050, freeze the Rust APIs and golden conformance vectors, then port
the completed pairs architecture by architecture in the same chronological
order. The initial port matrix is the fourteen non-Rust languages supported by
the scaffold generator: Python, Go, Ruby, TypeScript, Elixir, Perl, Lua, Swift,
Haskell, OCaml, Java, Kotlin, C, and C++. Existing implementations are audited
rather than overwritten. Before that wave starts, inventory additional package
ecosystems present in the repository (currently C#, Dart, F#, and Starlark) and
add scaffold support or an explicit exception so "all languages" remains an
auditable statement rather than an assumption.

The port wave has three infrastructure items before its generated per-target
queue:

- PORT-001: publish language-neutral instruction/state golden vectors for every
  completed Rust pair.
- PORT-002: inventory every repository language and record its scaffold/build
  support or explicit exception.
- PORT-003: generate the chronological architecture-by-language queue and mark
  existing packages as audit items.

## Discovery log

| Date | Item | Priority | Disposition |
|---|---|---|---|
| 2026-08-31 | RCPU-039 is complete. | Resolved; selects RCPU-040 by pair priority | `arm-simulator` now preserves the legacy wrapper while adding an exact checked 64 KiB functional machine, mirrored R15/PC, NZCV, the complete Spec 07b data-processing/memory/branch/condition/HLT surface, typed atomic lifecycle, structured encoders, 388 reproducible Python common-surface full-state vectors, direct spec suites, strict checks, and 90.15% package line coverage (714/792). |
| 2026-08-31 | RCPU-039 baseline audit found that `arm-simulator` has only MOV-immediate, register ADD/SUB, and a custom HLT, with three Rust tests. It ignores every condition field, the S bit and NZCV, silently advances unknown opcodes, accepts caller-sized legacy CPU memory, and has no exact state restore, origin-aware loader, typed failures, transition-atomic step, bounded transactional run, or full traces/results. Strict Clippy passes but strict rustdoc fails on the unescaped `[31:28]` text. The 30-test Python package adds immutable snapshots and protocol-shaped results but executes the same three-opcode surface, reports constant-false flags, and retains the non-atomic legacy lifecycle. Spec 07b additionally requires CMP, AND, ORR, LDR, STR, B, BEQ/BNE and conditional execution. | P0, chronological functional completion, blocks RCPU-040 | Preserve the legacy wrapper for consumers while building an exact 64 KiB checked functional machine with 16x32-bit registers, PC and NZCV state; implement the complete documented data-processing, memory, branch, condition, and HLT surface; fail closed on malformed/unknown/truncated/alignment/range faults; add structured encoders, complete lifecycle suites, a reproducible Python full-state corpus plus spec-only manual vectors, strict checks, consumers, and at least 80% coverage. |
| 2026-08-31 | RCPU-038 is complete. | Resolved; selects RCPU-039 | `x86-64-gatelevel` now has the exact 525,382-DFF topology, an independent complete integer execution path, repository-gate arithmetic/flags/Boolean/barrel-shift/rotate/address/condition networks, fixed 64-round multiply and 128-round divide, the shared atomic lifecycle, all 262 Python full-state vectors, direct manual-correct CQO and fault suites, Spec 07w2, strict checks, and 87.13% package line coverage (711/816). |
| 2026-08-31 | RCPU-038 has no Rust or Python gate-level package and no Spec 07w2. The completed functional oracle establishes 524,288 memory bits, 1,024 GPR bits, 64 RIP bits, five specified RFLAGS bits, and one HALT latch: exactly 525,382 persistent DFFs; installed-range fields remain validated lifecycle metadata. | P0, chronological gate completion | Create `x86-64-gatelevel` with the exact DFF topology, repository-gate decode/ALU/shift/rotate/multiply/divide/address/condition networks, its own complete instruction execution path, the shared atomic lifecycle, all 262 functional vectors plus manual-correct CQO and fault suites, Spec 07w2, strict checks, and at least 80% coverage. |
| 2026-08-31 | RCPU-037 is complete. | Resolved; selects RCPU-038 | `x86-simulator` now has a separate exact 64 KiB wrapping functional machine, complete typed transactional lifecycle/traces/results, the full Spec 07w integer surface, seven lifecycle suites, 262 reproducible Python full-state vectors, preserved backend/SSE consumers, strict checks, and 89.28% line coverage (2,065/2,313). |
| 2026-08-31 | RCPU-037 differential construction found Python/spec boundary defects. The documented flag state exposes only CF/PF/ZF/SF/OF while the backend-oriented Rust state also retained AF, so the normative functional boundary must mask AF. Python resets RSP to `0xFFF8` while the spec's divergence prose incorrectly said zero. More seriously, Python has no `CQO` (`REX.W 99`) handler: it silently treats the instruction as undefined and consumes the following byte, advancing RIP by three for `48 99 F4`. | P0 inside RCPU-037 | Match the documented five-flag state, correct reset prose to `0xFFF8`, keep the manual-correct Rust `CQO`, cover it with direct tests, record it as an oracle defect, and omit only that defective Python case from the otherwise reproducible full-state differential corpus. |
| 2026-08-31 | RCPU-037 audit found that the existing `x86-simulator` is a backend-runtime lane rather than the complete Spec 07w functional oracle. Its 66 Rust tests execute current backend/SSE output, but state omits owned memory, halt, installed range, input position, and loader metadata; memory is caller-sized 1 MiB and strictly bounded instead of the normative wrapping 64 KiB machine; execution terminates through a private return sentinel rather than HLT; there is no public reset/load/restore/direct-access/complete trace or transactional checked lifecycle; and a late decode/execution/memory/host-call failure can retain earlier state mutations. Strict Clippy also fails on current stable. The Python oracle passes 97 tests at 84.45% line coverage over the broad Spec 07w integer surface, but uses legacy SIM00 lifecycle and has no reproducible full-state differential corpus. | P0, chronological functional completion, blocks RCPU-038 | Preserve the backend harness and SSE extensions as consumers while adding a distinct exact 64 KiB Spec 07w machine boundary, complete typed atomic lifecycle, the full Python integer decode surface, public structured encoding helpers where useful, reproducible full-state differentials, strict checks, consumers, and at least 80% Rust line coverage. Then build the DFF/gate partner as RCPU-038. |
| 2026-08-31 | RCPU-036 has no Rust package or normative Spec 07u2. The Python gate oracle passes 316 tests at 86.81% line coverage and routes broad combinational arithmetic through gates, but its register bank is mutable host bit lists rather than DFFs, memory is a host `bytearray`, halt is a host boolean, fetch mutates CIA before decode, and it inherits the legacy non-transactional SIM00 boundary: no complete restore, installed range, typed faults, atomic step/run, or full-state corpus. It also silently truncates loads, rounds misaligned access, wraps addresses, synthesizes zero on divide-by-zero, and mutates halt/CIA on illegal decode. | Resolved by RCPU-036; selects RCPU-037 chronologically | Completed as `powerpc601-gatelevel` with exactly 525,473 persistent DFFs, its own complete combinational decode and gate execution path, fixed-round multiply/divide, the shared atomic lifecycle, all 239 full-state vectors, Spec 07u2, strict checks, and 94.46% Rust line coverage. |
| 2026-08-31 | RCPU-035 differential audit found the Python oracle assigns zero to `divw`/`divwu` on a zero divisor even though the ISA result is architecturally undefined, making an execution appear successful and indistinguishable from a real zero quotient. | P0 correctness within RCPU-035 | Preserve Python differential coverage for every defined nonzero-divisor case; expose zero divisors as distinct typed, step-atomic signed/unsigned faults in Rust and cover both in lifecycle tests. |
| 2026-08-31 | RCPU-035 has no Rust package or Rust encoder/backend consumer. Normative Spec 07u and the Python oracle cover the 32-bit big-endian PowerPC 601 integer subset with 137 passing tests at 98.11% line coverage, but expose only legacy SIM00 state: no installed-program range, restore, origin-aware loading, typed direct access/errors, complete before/after traces, or transactional runs. Oversized programs silently truncate, aligned loads/stores silently mask misaligned addresses, already-halted steps succeed as no-ops, unknown instructions return string error traces, and no reproducible full-state differential corpus exists. | P0, chronological functional oracle, blocks RCPU-036 | Port the complete specified/oracle surface in one `powerpc601-simulator` Rust crate with exact 64 KiB state, 32 GPRs plus LR/CTR/XER/CR/CIA, public structured encoders, typed atomic failures, complete lifecycle, exact big-endian access and PowerPC bit numbering, all branch/CR/XER edges, reproducible Python full-state vectors, strict checks, consumers, and at least 80% line coverage. |
| 2026-08-31 | RCPU-034 has no Rust package or normative Spec 07s2. The Python gate oracle is broad (351 passing tests, 98.91% line coverage) but its 32 registers are host bit lists rather than sequential primitives, memory is a host `bytearray`, nPC and halt are host scalars, and it exposes only the legacy non-transactional SIM00 lifecycle inherited from the functional oracle. Its standalone `uv` project also omits local source declarations for all four repository dependencies, so the audit required an isolated test environment with explicit source paths. | Resolved by RCPU-034; selects RCPU-035 chronologically | Completed as `alpha-axp-gatelevel` with exactly 526,465 persistent DFFs, complete combinational decode and gate datapaths, the shared atomic lifecycle, all 624 full-state vectors, Spec 07s2, strict checks, and 96.80% Rust line coverage. Keep Python packaging enrollment as a P1 infrastructure follow-up rather than changing the oracle during the Rust wave. |
| 2026-08-31 | RCPU-033 has no Rust package or Rust encoder/backend consumer. Normative Spec 07s and the Python oracle provide a broad 64-bit little-endian integer ISA baseline with 146 passing tests at 88.83% line coverage, but the oracle exposes only legacy SIM00 state: no installed-program range, restore, typed direct access, typed errors, complete atomic traces/results, or transactional runs. Invalid instructions, PAL calls, and alignment failures are converted into a mutated halt state; an already-halted step succeeds; loads cannot select an origin; and no reproducible full-state differential corpus exists. | Resolved by RCPU-033; selects RCPU-034 by pair priority | Completed as `alpha-axp-simulator` with the exact machine, public encoders, validated complete state, typed atomic lifecycle, 624-vector Python full-state differential, all 146 Python tests, strict checks, and 88.08% Rust line coverage. No Alpha Rust consumer existed. Build the DFF/gate partner next. |
| 2026-08-31 | RCPU-032 baseline has 42 passing tests but only 59.60% package line coverage. All 526,185 persistent architectural bits (524,288 memory, 1,792 physical-register, PC/nPC, CWP/depth, PSR, Y, and halt) are host values and nPC is absent. The gate branch-condition table assigns most condition numbers to the wrong predicates; divide-cc is missing; divide-by-zero saturates instead of faulting; loads retain stale memory; misalignment is accepted; halted/illegal/faulting steps mutate PC or state; non-sentinel traps can halt; state/restore/complete traces/results and typed transactional load/step/run are absent; the differential corpus is absent; and strict formatting already fails. | Resolved by RCPU-032; selects RCPU-033 by chronological pair priority | Completed with the exact 526,185-DFF topology, shared functional lifecycle, fixed-round gate division, all 16 corrected Bicc predicates, atomic trap/fault handling, documented UDIVcc/SDIVcc paths, the complete 248-vector differential, normative Spec 07r2, strict checks, and 94.72% line coverage. |
| 2026-08-31 | RCPU-032's full-state differential exposed two additional legacy gate defects after the baseline audit: MULScc used a different recurrence from the completed functional/manual contract, and RD `%y` rejected valid encodings whenever `rs1` was nonzero. Native host division also remained beneath the existing UDIV/SDIV facade. | Resolved, P0 functional and gate fidelity | Match the functional MULScc recurrence, accept the architecturally ignored RD `%y` source field, and replace native division with fixed 64-step unsigned/signed restoring gate networks. Pin all fixes through the aggregate differential and divide-cc lifecycle edges. |
| 2026-08-31 | RCPU-031 completion audit closes the full Rust functional boundary with 58 tests, 248 reproducible Python full-state vectors, manual-backed cc multiply/divide corrections, strict checks, green encoder/backend consumers, and 93.45% line coverage. | Completed; selects RCPU-032 by chronological pair priority | Audit the SPARC V8 gate model against the completed functional lifecycle and all vectors; add the manual-defined divide-cc gate paths, preserve the already-correct multiply-cc encodings, and keep publication serialized behind RCPU-031. |
| 2026-08-31 | Correcting the reachable SPARC divide-cc paths exposed two latent execution defects: `UDIVcc`/`SDIVcc` cleared V even when the saturated quotient overflowed, contrary to the V8 manual, and signed `Y:rs1 = i64::MIN` divided by `-1` could panic in Rust before saturation. | P0, condition-code fidelity and panic safety, blocks RCPU-031 | Track the pre-saturation quotient, set N/Z from the saturated result, set V exactly on divide overflow, clear C, and use checked signed division for the unique host-overflow pair. Pin ordinary, positive-overflow, negative-overflow, and `i64::MIN / -1` transitions. |
| 2026-08-31 | RCPU-031 ISA audit found that Spec 07r, the Python oracle, and the Rust functional constants assigned `UMULcc`/`SMULcc`/`UDIVcc`/`SDIVcc` op3 values `0x5A`/`0x5B`/`0x5E`/`0x5F`, even though Format 3 op3 is only six bits wide. Those cases are unreachable after decode masking. The SPARC V8 manual encodes them as `0x1A`/`0x1B`/`0x1E`/`0x1F`; the existing Rust gate model already uses the correct multiply-cc pair but omits both divide-cc cases. | P0, unreachable documented instructions, blocks RCPU-031 | Correct the Rust functional constants, encoders, normative table, decode/execution tests, and checked-fault preflight now. Record the Python oracle correction for the later cross-language wave, exclude its invalid cc encodings from the Python differential, and require RCPU-032 to add both documented divide-cc gate paths plus full functional differentials. |
| 2026-08-29 | RCPU-031 audit found 49 passing Rust tests and a broad ISA port, while the 123-test Python oracle is green at 93.70% coverage. The Rust boundary is incomplete and diverges from normative 64 KiB semantics: caller-sized memory; unchecked, non-resetting loads; unmasked PC/JMPL/branch/effective addresses that can panic; no alignment checks; unknown instructions silently no-op; divide, window, and trap faults collapse into halt; nPC and complete physical-register/load-range state are absent; snapshots, restore, typed direct access, complete traces/results, and atomic checked load/step/run are absent; strict rustdoc fails; and there is no reproducible full-state Python differential. | P0, functional completion contract, follows RCPU-030 | Preserve the 49-test ISA core while adding the exact 64 KiB/nPC/window state contract, typed transactional lifecycle and faults, wrapping/alignment semantics, complete physical snapshots and direct access, a reproducible Python full-state decode/fault corpus, strict checks, consumer verification, and at least 80% total Rust line coverage before selecting RCPU-032. |
| 2026-08-28 | RCPU-030 audit found a broad 33-test gate implementation at 90.86% line coverage with gate ALU, shifts, multiply/divide, decode, unaligned transfers, and 21 doctests, but all 525,409 persistent memory/GPR/HI/LO/PC/halt bits were host-backed stable values rather than clocked DFF state; memory and halt were plain host storage; complete state/traces/results, restore, and typed transactional lifecycle were absent; the gate-specific error contract diverged from the completed functional oracle; no 218-vector full-state differential existed; and strict Clippy failed. | P0, gate completion contract, follows RCPU-029 | Resolved locally with the exact 525,409-DFF topology, sequentially clocked simulator writes, the shared complete functional lifecycle, atomic transition validation against all 218 Python vectors, normative Spec 07q2, strict checks, and 94.30% total line coverage. Publish after RCPU-029, then audit RCPU-031. |
| 2026-08-28 | RCPU-029 audit found 32 passing Rust tests at 96.25% line coverage and a 130-test Python oracle at 93.27%, but the Rust boundary is deliberately divergent and incomplete: caller-sized memory and unwrapped PC/effective addresses disagree with normative 64 KiB Python semantics; loads can panic and retain stale state; misalignment is silently accepted; unknown instructions no-op; BREAK, overflow, and division faults collapse into halt; state snapshots, traces, restore, and typed atomic load/step/run are absent; bounded results omit final state/traces/errors; strict rustdoc fails; no full-state Python differential exists; and the functional ISA omits LWL/LWR/SWL/SWR already present in the Rust gate partner. | P0, functional completion contract, follows RCPU-028 | Resolved locally with exact 64 KiB wrapping state, complete typed transactional lifecycle/traces/results, distinct atomic faults, all four unaligned merge operations, 218 Python full-state vectors, consumer verification, strict checks, and 94.51% total line coverage. Publish after RCPU-028, then audit RCPU-030. |
| 2026-08-28 | RCPU-028 audit found 21 passing tests and working gate ALU, condition, barrel-shifter, and broad instruction execution, but all 536,871,777 persistent memory/register/halt bits are host-backed; memory is caller-sized; out-of-range access silently fabricates or drops data; loads truncate and retain stale bytes; halted stepping mutates state; block arithmetic can panic; force-user transfers and external IRQ/FIQ are absent; state/results/traces and typed transactional lifecycle boundaries are absent; the differential covers only nine small programs instead of the completed 599-vector functional corpus; and Spec 07e2 retains a Python/25-register design sketch. Baseline strict checks pass at 81.78% line coverage. | P0, gate completion contract, follows RCPU-027 | Resolved locally with exact 536,871,777-DFF state, the shared typed transactional lifecycle, force-user transfers and IRQ/FIQ entry, all 599 full-state functional transitions, normative Rust Spec 07e2, strict checks, and 93.12% total line coverage. Publish after RCPU-027, then audit RCPU-029. |
| 2026-08-28 | RCPU-027 audit found a broad 51-test Rust ARMv1 implementation at 91.63% line coverage with strict checks already green, but memory is caller-sized instead of exposing the 64 MiB architectural constructor; out-of-range fetch/data reads return zero, writes and oversized loads silently truncate, halted steps keep mutating PC, unchecked block-transfer arithmetic can panic, LDM/STM's force-user bit is decoded but ignored, external IRQ/FIQ entry promised by Spec 07e is absent, traces omit banked registers/full memory, state restore and typed atomic load/step/run boundaries are absent, and no Python full-state differential exists. | P0, functional completion contract, follows RCPU-026 | Resolved locally with the exact architectural constructor, complete physical-register/memory state, force-user transfers and IRQ/FIQ entry, deterministic typed transactional lifecycle, 599 Python full-state vectors, consumer verification, strict checks, and 92.29% total line coverage. Publish after RCPU-026, then audit RCPU-028. |
| 2026-08-28 | RCPU-026 audit found 69 passing unit tests and a broad instruction dispatcher, but all 1,050,641 persistent code/XDATA/IRAM/PC/halt bits are host-backed; checked atomic load/restore/step/run state and complete traces are absent; no functional full-state differential exists; RL/RR incorrectly modify CY; MUL/DIV use host shifts and data-dependent control; legacy loads retain stale code/XDATA and can panic at the boundary; strict rustdoc fails; and total line coverage is 78.94% with the CPU engine at 69.98%. | P0, gate completion contract, follows RCPU-025 | Resolved locally with exact DFF state, shared typed transactional lifecycle, corrected rotate/divide edges, fixed MUL/DIV networks, normative Spec 07p2, all 256 full-state functional transitions, strict checks, 97.01% CPU-engine coverage, and 97.51% total coverage. Publish after RCPU-025, then audit RCPU-027. |
| 2026-08-28 | RCPU-025 audit found a broad Rust dispatcher with 35 passing tests, but oversized loads and invalid indirect inputs panic, truncated instructions intentionally wrap across the 64 KiB code boundary, reserved opcodes panic after partial fetch, loads retain stale code beyond the new program, bounded results omit traces/final state/errors, complete owned state and typed transactional load/restore/step/run boundaries are absent, no Python full-state differential exists, and strict rustdoc fails. | P0, functional completion contract, follows RCPU-024 | Resolved locally with deterministic typed transactional lifecycle, complete Harvard state/traces/results, reproducible exhaustive 256-opcode Python full-state hashes, all consumers, strict checks, 98.20% instruction-engine coverage, and 94.53% total coverage. Publish after RCPU-024, then audit RCPU-026. |
| 2026-08-28 | RCPU-024 audit found 67 passing unit tests and broad instruction coverage, but all 134,218,289 persistent memory/register/SR/PC/halt bits were host-backed; MUL/DIV used host arithmetic; oversized loads were silently truncated; invalid, misaligned, division-by-zero, and halted transitions collapsed into an unchecked halt; state/results/traces and transactional failure boundaries were absent; no functional full-state differential existed; and Spec 07n2 still described a Python package and declared complete instruction families that the Rust dispatcher did not implement. | P0, gate completion contract, follows RCPU-023 | Resolved locally with exact stable DFF state, fixed gate multiply/divide, shared typed transactional state/traces/results, the complete 82-vector full-state corpus, corrected normative Rust documentation, strict checks, 81.89% CPU-engine coverage, and 81.82% total coverage. Publish after RCPU-023, then begin RCPU-025. |
| 2026-08-28 | RCPU-023 audit found 55 passing Rust tests but an intentionally reduced port: indexed and both PC-relative addressing modes; all immediate and bit operations; NEGX/PEA/SR/CCR moves; MUL/DIV; ADDX/SUBX; and memory shifts were deferred. The caller-sized memory silently returned zero or dropped writes out of range, and lifecycle APIs had no typed atomic failure, full owned state, or complete traces. | P0, functional completion contract, follows RCPU-022 | Resolved locally with the complete Spec 07n/Python surface, exact 16 MiB checked machine, typed transactional lifecycle/full state/traces, 82 full-state vectors, consumer verification, strict checks, and 85.96% total line coverage. Publish as one cell after its predecessors, then audit RCPU-024. |
| 2026-08-28 | RCPU-022 audit found broad gate-backed execution semantics, but all persistent registers, flags, memory, ports, and halt state were host values; multiply/divide used host arithmetic; lifecycle failures were unchecked; state snapshots were incomplete; and the gate documentation described stale subset and interrupt behavior. | P0, gate completion contract, follows RCPU-021 | Resolved locally with the exact 8,392,922-DFF topology, shared typed transactional lifecycle, fixed gate multiplier/divider networks, 461 full-state functional differentials, strict checks, 95.67% CPU-core coverage, and 94.41% total package coverage. Publish as one cell after its predecessors, then audit RCPU-023. |
| 2026-08-28 | RCPU-021 audit found 61 passing tests but a deliberately curated Rust implementation: no memory ModRM forms, prefixes, stack/control flow, strings, interrupt return/halt behavior, shifts/rotates, multiply/divide, BCD adjustment, segment-register transfers, or I/O; caller-variable memory; unchecked lifecycle; incomplete state/results; and stale subset documentation. | P0, functional completion contract, follows RCPU-020 | Resolved locally with complete Python-oracle execution, architectural 1 MiB memory, typed transactional lifecycle, full state/traces/ports, 461 full-state differential vectors, consumer compatibility, strict checks, and 95.17% core coverage. Publish as one cell after its predecessors, then audit RCPU-022. |
| 2026-08-28 | RCPU-020 audit found 58 passing unit tests and broad gate-ALU instruction coverage, but every claimed register and all memory/port/interrupt/halt state were host-backed, runs were unchecked and non-transactional, snapshots omitted ports, undefined instructions silently consumed bytes, interrupt modes could be selected but interrupts could not be delivered, ED omitted RLD/RRD and all eight block-I/O operations, no normative 07k2 gate spec existed, and strict rustdoc failed. | P0, gate completion contract, follows RCPU-019 | Resolved locally with exact 528,597-DFF storage, shared typed transactional state/lifecycle, complete ED and interrupt entry, Spec 07k2, all 1,160 Python-oracle vectors, 70 tests, strict checks, and 97.64% core coverage. |
| 2026-08-28 | RCPU-019 audit found 60 passing unit tests and substantial 8080-compatible/CB behavior, but the Rust package deliberately omits the entire ED-prefixed space and nearly all IX/IY displacement, stack, arithmetic, load, jump, and indexed-bit forms required by Spec 07k. Undefined instructions silently halt, load/fetch/stack/memory boundaries can panic, runs swallow faults, snapshots and traces are incomplete, interrupts are absent, memory size is caller-variable rather than architectural, and strict rustdoc has 23 failures. | P0, functional completion contract, follows RCPU-018 | Resolved locally through RCPU-019A/B/C: fixed 64 KiB typed transactional lifecycle and interrupts; complete ED and DD/FD/DDCB/FDCB execution; 1,160-vector Python full-state oracle; 66 simulator and 16 consumer tests; strict checks; and 98.49% core coverage. Publish as one cell after its predecessors. |
| 2026-08-28 | RCPU-014 full audit found host-backed memory/flags/halt/I/O/depth, a fictitious stored M slot, host INR/DCR and zero detection, port clamping, truncating/panicking loads, wrapping operands, swallowed run failures, trace-only differentials, no owned full state, stale Python/bytearray documentation, inconsistent gate counts, and 34 strict-rustdoc failures. | P0, gate completion contract, follows RCPU-013 | Resolved locally with exact 131,504-DFF persistent topology, gate datapaths, shared typed transactional APIs/state, exhaustive 256-encoding and workload full-state differentials, normative Rust documentation, strict checks, and 99.82% core coverage. |
| 2026-08-27 | RCPU-013 consumer validation found the existing `intel8008-gatelevel` tests still lockstep with the hardened functional API, but its strict rustdoc build fails on 34 bracketed bit/slot references and its lifecycle methods retain the same unchecked/panic-prone boundary shape found in the functional crate. | P0, gate completion contract, follows RCPU-013 | Keep RCPU-014 next after the functional audit publishes. Fix documentation as part of a full gate audit with typed atomic lifecycle/I/O errors, owned full-state snapshots, exact persistent topology, and exhaustive functional differentials rather than a documentation-only detour. |
| 2026-08-27 | RCPU-013 audit found a complete instruction dispatcher and 31 passing unit tests, but load and I/O methods could panic or truncate, step used post-fetch string errors and wrapped operands at the end of memory, run swallowed execution errors, no complete immutable snapshot existed, strict rustdoc failed, and core coverage was 79.14%. | P0, functional completion contract, follows RCPU-012 | Preserve instruction behavior while adding typed atomic load/step/run/port boundaries, deterministic runs, full owned snapshots, exhaustive 256-byte encoding-width coverage, consumer validation, strict Clippy/rustdoc, and above-floor core coverage. |
| 2026-08-27 | RCPU-012 audit found 88 passing unit tests and complete instruction dispatch, but no functional differentials or typed failure boundary. Oversized ROMs were silently truncated, a two-byte instruction at ROM end wrapped its operand to address zero, halted steps panicked, SRC retained four register-select bits instead of two, RAM outputs/selectors/ROM port/halt/stack pointer used host state, strict rustdoc failed, and Spec 07d2 described a stale Python/control-FSM design with inconsistent gate counts. | P0, gate completion contract, follows RCPU-011 | Preserve the gate-backed ISA while moving every mutable architectural bit into D flip-flops, publish the exact topology, share typed atomic errors with the functional oracle, add full snapshots and exhaustive encoding/full-state differentials, correct SRC, replace stale 07d2, pass strict Clippy/rustdoc, and exceed 95% core coverage. |
| 2026-08-27 | RCPU-011 audit found a complete, highly tested 46-instruction Rust implementation (82 passing tests, 98.83% pre-audit core coverage) whose legacy load/step/run API could panic on oversized programs, halted steps, truncated instructions, and out-of-range fetch/branch/FIN addresses. Spec 07d also still described a stale GenericVM/Python execution model. | P0, functional completion contract, blocks RCPU-012 | Preserve the complete ISA behavior while adding typed atomic failures, full owned snapshots, deterministic clearing loads, caller-bounded checked runs, direct and Nib-compiler consumer tests, and corrected Rust execution/API documentation. Require strict Clippy/rustdoc and at least 95% post-audit core coverage. |
| 2026-08-27 | RCPU-010 planning audit found no separate PDP-11 gate-level specification or Rust package. The completed behavioral model's orthogonal operand machinery makes effective-address side effects and byte/word stepping the largest fidelity risk; its 64 KiB memory dominates persistent topology. | P0, chronological gate partner, follows RCPU-009 | Create normative `07o2` and one `pdp11-gatelevel` crate. Require DFF-backed memory/register/PSW/halt state; gate decode; 16/8-bit ALU and NZVC networks; gate-backed address increments/decrements/indexing, branch offsets, SP/PC paths, and byte sign extension; exact topology; complete state/trace differentials across all modes and 59 mnemonic variants; atomic failures; docs/build integration; and at least 80% core coverage. |
| 2026-08-27 | RCPU-009 audit found a mature Python PDP-11 oracle and normative 07o spec but no Rust package. The oracle has 163 passing tests and 98.63% total line coverage. Its behavioral surface is 64 KiB little-endian memory, eight 16-bit registers, NZVC, all eight addressing modes, 12 double-operand word/byte variants, 25 single-operand variants, 15 branches, and HALT/NOP/RTI/RTS/JMP/JSR/SOB control. The Python package is not enrolled in the root uv workspace or repository BUILD graph, so its audit required an explicit local editable dependency on `simulator-protocol`. | P0, chronological functional oracle, blocks RCPU-010 | Port the complete specified/oracle surface in one `pdp11-simulator` Rust crate with typed atomic failures, bounded traces, immutable full-memory snapshots, public encoders, all addressing-mode side effects, byte-register MOV sign extension, exact NZVC edge rules, Python differential vectors, BUILD/workspace/docs integration, and at least 80% core coverage. Keep Python BUILD enrollment as a P1 infrastructure discovery; it does not block the Rust port. |
| 2026-08-27 | The audited Python `pdp11-simulator` has a valid package manifest and comprehensive tests but is absent from the root uv workspace and has no BUILD recipe, so normal repository discovery does not exercise it. | P1, build-graph completeness, non-blocking | Add a dedicated follow-up infrastructure item after the chronological Rust pair is secure: enroll the package and its local `simulator-protocol` dependency in the supported Python build graph without displacing RCPU-009/RCPU-010. |
| 2026-08-27 | RCPU-008 planning audit found no separate CDC 6600 gate-level specification or Rust package. The completed behavioral subset is nevertheless compact enough for one reviewable partner: 22 short and 14 long instructions over 60-bit X, 18-bit A/B, parcel P, and 4,096-word memory, with multiplication and variable shifts as its largest datapaths. | P0, chronological gate partner, follows RCPU-007 | Create a normative `07t2` gate contract and one `cdc6600-gatelevel` crate. Require DFF-backed persistent state except hardwired B0; one-hot gate decode; gate-vector add/subtract, compare, barrel shift, 60-stage partial-product multiply, address and branch networks; exact topology metrics; complete state/trace differentials; fail-closed preflight; documentation, BUILD/workspace integration, and at least 80% coverage. |
| 2026-08-27 | RCPU-007 pre-push security review found that byte-transport decoding collected every caller parcel before checking the fixed 16,384-parcel memory capacity. An oversized input could therefore force an avoidable allocation before its deterministic rejection. | P0, allocation safety, blocks RCPU-007 | Validate the derived parcel count before decoding or allocating, preserve the loaded machine on failure, and pin the oversized canonical byte transport alongside the direct-parcel bound. |
| 2026-08-27 | RCPU-007 audit found a complete Python CDC 6600 behavioral oracle but no Rust package. The oracle has 109 passing tests and 94.95% line coverage across 22 short and 14 long instructions, while its public spec intentionally bounds memory to 4,096 60-bit words and omits timing-only scoreboard, peripheral-processor, floating-point, and exchange-jump behavior. | P0, chronological functional oracle, blocks RCPU-008 | Port the entire specified behavioral surface to Rust with immutable snapshots, checked parcel fetch/branch/memory boundaries, exact 60/18-bit masking and signed comparisons, all instruction encoders, bounded execution, Python differential vectors, README/changelog/BUILD/workspace integration, and at least 80% Rust coverage before starting the gate-level partner. |
| 2026-08-27 | P006C gate review found that a restoring divider's partial remainder must be one bit wider than its divisor. A same-width temporary can discard the carry while dividing by the most-negative fixed operand even though ordinary vectors pass. | P0, gate arithmetic fidelity, blocks P006C | Use 78-bit remainder/divisor wires for the 77-by-39 fixed divider and 65-bit wires for the 64-bit floating divider; retain fixed iteration bounds and signed quotient/remainder gates. |
| 2026-08-27 | P006B3 gate-state audit found that API service selects special X group 32 at core 0200-0203, one beyond the ordinary five-bit SXG encoding. The P006A selected-group register could represent only groups 0-31. | P0, architectural state width, blocks P006B3 | Widen selected X-group state to six DFFs, retain ordinary SXG decode at five bits, add a separate six-bit interrupted-group latch, and pin group-32 vectoring plus restoration of the interrupted ordinary group. |
| 2026-08-27 | P006B fidelity review found two independently auditable hardware domains inside the original optional-I/O slice: decimal/clock is a combinational central datapath, while direct devices and selector/API are bounded event-driven state machines. Combining them would make gate provenance and atomic error review unnecessarily difficult. | P0, scope clarity, blocks RCPU-006 | Split P006B into P006B1 decimal/clock, P006B2 direct card/paper-tape/typewriter, and P006B3 selector/API. Preserve chronological order and keep all three ahead of the AAU/final audit. |
| 2026-08-27 | P006B1 implementation review found that the deterministic clock API accepts a 64-bit tick count, including `u64::MAX`; host modulo would violate the gate-level arithmetic contract even though instruction stepping remained gate-backed. | P0, gate fidelity, blocks P006B1 | Reduce and advance the external tick vector through a 65-bit restoring-division/add/subtract gate network, preserving both daily wrap and the documented 19-bit out-of-day recovery path. |
| 2026-08-27 | P006A pre-push security review found that `run(max_steps)` passed the caller-controlled bound directly to `Vec::with_capacity`, so `usize::MAX` panicked before executing even one fail-closed instruction. | P0, allocation/panic safety, blocks P006A | Grow traces only as accepted instructions execute, and pin an oversized bound that must report the first unknown instruction instead of preallocating from the bound. |
| 2026-08-27 | AAU pre-push security review found that normalized floating divide tried to normalize an exact zero quotient by left-shifting zero forever. A caller could load a noncanonical AX/QX pair and make one `FDV` step consume an unbounded CPU loop. | P0, denial-of-service safety, blocks AAU publication | Stop zero before the normalization loop, preserve its deterministic quotient/remainder result, and pin the externally loaded zero-quotient case. |
| 2026-08-27 | Propagating the P005C atomic core preflight into the AAU slice exposed the same late-failure shape in all sixteen AAU status branches: a not-taken skip beyond installed memory could update IX or clear overflow/underflow holds before failing. | P0, state-corruption safety, blocks AAU/final audit | Share a pure AAU status predicate between preflight and execution, validate the exact skip target before accepting the instruction, and pin full-state end-of-memory atomicity. |
| 2026-08-27 | Each CPU feature-branch push and pull-request event starts a separate full CI workflow, but branch protection accepts only the pull-request `CI gate`; the redundant push matrices can remain queued after merge and delay the next chronological slice. | P1, CI throughput, non-blocking | During the simulator wave, cancel only superseded push-event runs after confirming the corresponding pull-request run is retained. Schedule a dedicated workflow-trigger/concurrency audit without displacing architecture-correctness work. |
| 2026-08-27 | Final P005C fail-closed review found that an out-of-range second word or decision skip was detected only after I/P and, for some instructions, operand state had changed. The affected paths included even double-word operations, BXL/BXH, CAB/DCB, fixed readiness/register branches, and controller `BCS`; raw LDX/STX, SPB, and MOV also relied on execution-time checks. | P0, state-corruption safety, blocks RCPU-P005C | Add a single pre-execution core preflight for pair/raw/X/branch/MOV destinations and the exact taken skip, share pure branch predicates with execution, and pin full-state atomicity at installed-memory boundaries before publishing P005C. |
| 2026-08-27 | P006A lifecycle review found that its initial gate loader rejected a zero-word load exactly at installed-memory end, while the functional oracle accepts that empty half-open range; the same boundary matters to zero-length `MOV`. | P0, lifecycle parity, blocks P006A | Accept exactly-at-end empty loads and empty checked ranges, continue rejecting non-empty or beyond-end ranges without mutation, and pin end/oversized-origin regressions before publication. |
| 2026-08-27 | RCPU-006 inherits a much larger fidelity surface than the earlier gate-level machines: the completed GE-225 oracle includes central single/double arithmetic, modification and shifts, decimal/clock options, three direct devices, selector/API control, and a separate AAU with 40-bit fixed/floating datapaths. Treating that as one PR would make gate provenance and differential completeness difficult to audit. | P0, scope clarity, blocks RCPU-006 | Split RCPU-006 into P006A gate storage/decode and central binary core, P006B optional central/direct-I/O/controller/API state, and P006C AAU plus final full-family differential audit. Every persistent architectural bit is flip-flop-backed; arithmetic and logic results use `logic-gates`/`arithmetic`; host integers remain limited to addresses, loop/control sequencing, queues, and trace bookkeeping. |
| 2026-08-27 | The AAU manual makes the final functional slice a distinct coprocessor model rather than aliases over central A/Q. It has separate 40-bit AX/BX/QX/IX registers; fixed, normalized floating, and unnormalized floating modes; exact `30`/`31`/`32`/`33`/`35`/`36` arithmetic/data-transfer opcodes; eleven general words; and sixteen plug-7 `BAR` words. Overflow/underflow indicators are transient, their hold indicators persist, and hold tests conditionally clear the corresponding hold. | P0, architecture completeness, blocks RCPU-005 and RCPU-006 | Implement the separate public AAU state and exact encodings, paired/odd memory behavior, CPU X modification and IX capture, deterministic integer mantissa/exponent arithmetic, readiness/mode/address preflight, normalization and exponent alerts, all status branches, reset behavior, and signed regression vectors. Close RCPU-005 only after the combined package remains above the coverage floor. |
| 2026-08-27 | The API hardware description adds two control-flow constraints beyond basic ready-event vectoring: the optional priority X group is the special group 32 at core 0200-0203 (not encodable by ordinary `SXG`), and an interrupt may not occur between any `BRU` and its target—so even `BRU *` is uninterruptible. Priority return requires `SET PST` followed by a modified `BRU`; inserting `SET PBK` after `SET PST` must still leave priority mode while returning to the main program with API disabled. | P0, architecture/control-flow correctness, blocks RCPU-P005C | Reserve and select special group 32 only during API service, save P at 0201 and vector to 0204, restore the interrupted X group on the armed modified branch, retain the return arm across `SET PBK`, and suppress interrupt recognition for the first target access after every `BRU`. Pin immediate, deferred, disabled-return, and `BRU *` regressions. |
| 2026-08-27 | The primary manual makes RCPU-P005C a controller-selector **and API** slice, not just a generic device bus. The selector has eight fixed-priority plugs (0 highest), must alert-halt if `SEL P,X` (`2500P20`) is issued while busy, transmits the following two words without CPU execution, leaves P at the third sequential word, and clears controller errors on selection. `BCS` conditions are controller-specific. Optional API must remember enabled devices' not-ready-to-ready transitions even while interrupts are disabled, interrupt only at instruction boundaries, select X-group 32, store the main-program continuation at octal 0201, branch to octal 0204, enter priority mode with interrupts disabled, and require `SET PST` plus a modified branch to leave priority mode; card reader and punch participate, while typewriter and paper tape do not. | P0, architecture/control-flow correctness, blocks RCPU-005 | Make selection/busy/error behavior, opaque two-word command delivery, per-plug priority and readiness, controller-specific `BCS` predicates, API masks/latches/modes, exact group-32 save/vector behavior, deferred interrupts, and card ready transitions the acceptance boundary of RCPU-P005C. Keep controller timing deterministic through explicit service events and expose a public generic controller adapter before adding device-specific controller manuals. |
| 2026-08-27 | The GE-200 punched-card and GE-225 paper-tape subsystem manuals make RCPU-P005B larger and more exact than the current host-record abstraction. Cards require `RCD`/`RCB` continuous modes, `RCF` single-card mode, `HCR`, `WCD`/`WCB`/`WCF`, reader/punch ready branches, fixed data and synchronization-word layouts, and fail-closed not-ready behavior. Paper tape is a streaming N-register peripheral: `RON`/`PON` select mutually exclusive power paths, `RPT` streams frames and asserts N-ready, `HPT` stops motion, `WPT` emits one frame, and `OFF` powers the path down; unread frames can overrun N. The same `2500006` word means `TYP`, `RPT`, or `WPT` according to the powered N-register device, and `HPT` also enables the optional typewriter keyboard-input path. | P0, architecture and peripheral correctness, blocks RCPU-005 | Make those public contracts the acceptance boundary of RCPU-P005B. Preserve deterministic bounded host queues and output capture, decode the shared command from explicit device-selection state, model timing as explicit readiness/events rather than wall-clock sleeps, and pin all six card data layouts plus synchronization/status words, power switching, readiness branches, atomic memory failures, paper-tape overrun, typewriter input, and output bounds before selecting RCPU-P005C. |
| 2026-08-27 | The RCPU-P005A reprioritization audit found that the corrected programming manual treats direct M/N-register devices separately from controller-selector peripherals, delegates full punched-card and paper-tape behavior to subsystem manuals, and gives controller operations their own selection, three-word command, status, and interrupt model. A single combined I/O item would conceal two independently testable public contracts. | P0, scope clarity, blocks RCPU-005 | Split the remaining I/O work chronologically into RCPU-P005B for deterministic direct card/paper-tape/typewriter contracts and RCPU-P005C for controller-selector selection, status, command-block, interrupt, and generic device-controller contracts. Keep both ahead of AAU and the final functional audit. |
| 2026-08-27 | RCPU-P005A audit found that `SET DECMODE` only toggled exposed state: `ADD`, `SUB`, `DAD`, `DSU`, `ADO`, and `SBO` continued to execute binary arithmetic. The optional real-time-clock `LAC`/`LCA` instructions were absent, and opcode 24 was exposed as noncanonical `MOY` despite the corrected manual's `MOV` definition. | P0, architecture correctness, blocks RCPU-005 | Implement the documented three-digit-per-word BCD layout, ten's-complement signed fields, end-of-field overflow and carried lower fields; expose bounded deterministic clock control with exact `LAC`/`LCA` transfers; rename opcode 24 and its diagnostics to `MOV`; pin the manual's single/double arithmetic and clock examples. |
| 2026-08-27 | RCPU-P004 manual audit found that `SXG` was hard-coded as `2506013` and selected a group from A, while General Electric's corrected manual specifies the variable `2506YY3` form and selects encoded group Y. Fixed/shift automatic modification was not decoded, the I register retained the unmodified operand, invalid modified targets advanced P, non-overflowing single operations cleared the latched overflow indicator, and N-input shifts ignored N readiness. | P0, architecture correctness, blocks RCPU-005 | Implement encoded SXG groups 00-31, operand-field modification through selected core X words with the 31-place shift bound and modified I value, fail-closed target preflight, latched single overflow through `BOV`/`BNO`, N-ready preflight, corrected-manual regressions, and an above-floor coverage audit. |
| 2026-08-27 | RCPU-P003 manual audit found that A/Q was combined as a conventional signed 40-bit integer even though the GE-225 uses one sign plus two 19-bit data fields and ignores or replaces Q's duplicated sign. This corrupted DAD, DSU, DCB, MPY, DVD, and SRD; zero-count double shifts skipped required sign transfers; NOR/DNO wrote their remainder into the selected X group instead of absolute location 0000. | P0, architecture correctness, blocks RCPU-005 | Implement the 39-bit architectural conversion, correct arithmetic/divide-overflow and A/Q shift/normalize semantics, and pin the manual's published octal examples before continuing the remaining integer/automatic-modification audit. |
| 2026-08-27 | RCPU-004 fidelity audit found that an initial floating implementation delegated FAD/FSB/FMP/FDH/FDP results to host `f64`, violating the gate-level completion contract even though simple differential tests passed. | P0, gate-level fidelity, blocks RCPU-004 | Replaced it before PR with exact bit-vector alignment, gate add/multiply/restoring divide, 53-bit round-to-nearest-even intermediates, and 512 seeded oracle comparisons including divide remainders. |
| 2026-08-27 | RCPU-005 primary-manual audit found that `07g` explicitly scoped the implementation as an MVP, while the Rust package silently wrapped effective addresses and multiword/device transfers, kept modification words in detached host arrays instead of reserved core, partially mutated state on range errors, and covered only 7 tests. The manual also exposes deferred automatic-modification, optional central-processor, controller-I/O, and AAU families plus incorrect double-length representation in the current model. | P0 memory/correctness prerequisite, then chronological architecture completeness | Add RCPU-P002 ahead of RCPU-005 for fail-closed installed-memory and architectural X-word storage. Continue RCPU-005 in manual-backed integer/shift, optional CPU and I/O, and AAU slices; do not start RCPU-006 until all slices close. |
| 2026-08-27 | RCPU-003 manual audit found five historical errors inherited by `07h` and the Python oracle: CAL targeted Q instead of P, HPR and DVH used transfer PCs, TNO retained a set overflow indicator, TNX was omitted, and +0240 FDH was mislabeled as +0241 FDP. | P0, architecture correctness, blocks RCPU-003 | Correct the Rust simulator and `07h` against IBM's 1955 manual, add targeted regressions, and record the Python implementation for repair during its later cross-language audit. |
| 2026-08-27 | RCPU-003 pre-push audit found that transport decoding could allocate an oversized temporary word vector before comparing it with configured memory, and an empty load with an unbounded origin could reach an invalid slice. | P0, allocation/panic safety, blocks RCPU-003 | Validate canonical length, origin, and decoded word count before allocation or slicing; add end-of-memory and `usize::MAX` origin regressions. |
| 2026-08-27 | RCPU-003: the repository already had a complete Python IBM 704 simulator and conformance suite. | P0, correctness aid | Use the Python implementation as the behavioral oracle while retaining the canonical Rust encoder as the transport authority; port its v1 semantics and architecture programs to Rust. |
| 2026-08-27 | RCPU-P001: the Rust IBM 704 encoder shifts an idealized 9-bit opcode into bits 35–27, labels `+0420` as HTR, emits little-endian words, and the backend treats a `CLA` address as an immediate. These conflict with the 1955 IBM Type B format and `07h`'s executable big-endian transport contract. | Resolved prerequisite | Corrected by merged PR #13234; RCPU-003 now consumes the canonical encoder and transport. |
| 2026-08-27 | The existing C and C++ IBM 704 encoders mirror the same legacy idealized layout. | P1, non-blocking for Rust | Preserve Rust-first ordering; add these packages to the IBM 704 cross-language port/audit item after the Rust matrix is complete. |
| 2026-08-27 | Current stable Clippy flags a collapsible ECALL condition in the already-affected `riscv-simulator`, blocking RCPU-P001's CI-equivalent graph lint. | P0, CI-blocking | Preserve behavior with a direct boolean assignment and verify the package tests in RCPU-P001. |
| 2026-08-27 | Current stable Clippy flags two nested phase-transition conditions in the already-affected `system-board`, blocking RCPU-P001's CI-equivalent graph lint. | P0, CI-blocking | Express the conditions as match guards without changing the phase transitions, then verify the package tests and affected graph in RCPU-P001. |
| 2026-08-27 | Pre-push security review found that independently compiled IBM 704 functions used function-local literal addresses after concatenation and did not enforce a module-wide 32K bound. | P0, correctness/data-corruption, blocks RCPU-P001 | Add absolute load-address relocation to the backend, enforce the remaining address space per function, and pin a two-function `lang-aot` regression test. |
| 2026-08-27 | Pre-push security review found that the Type A encoder could silently create Type B-discriminated words for prefixes `000` and `100`. | P0, encoding correctness, blocks RCPU-P001 | Reject non-Type-A and oversized prefixes through a typed, non-panicking encoder error and add boundary tests. |
| 2026-08-27 | Pre-push security review found that the backend applied its 32K guard only after using caller-controlled CIR length for allocation. | P0, allocation safety, blocks RCPU-P001 | Bound the minimum emitted word count before allocation and retain the exact post-lowering bound check. |
| 2026-08-27 | Existing Rust simulator APIs are not yet unified by a Rust equivalent of SIM00. | P1, non-blocking | Add an API-convergence design/audit before the cross-language golden-vector freeze; new crates meanwhile expose the five common lifecycle operations. |
| 2026-08-27 | The older CPU roadmap records Python completion, not Rust pair completion, and its gate-level list is stale. | P1, non-blocking | This file is the canonical Rust wave ledger; link it from the older roadmap in RCPU-001. |
| 2026-08-27 | Several existing Rust functional crates openly implement subsets (notably 07b ARMv7 and the x86-64 runtime lane). | P1 | Preserve their audit status and create precise follow-ups during their chronological audit items. |
