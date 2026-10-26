# Layer 07ax2 — ARM7TDMI Educational Gate-Level Simulator

## 1. Status, authority, and boundary

This specification defines the gate-level partner of the checked ARM7TDMI
functional model in Spec 07ax.  Its architectural sources remain Arm's
*ARM Architecture Reference Manual* for ARMv4T, DDI 0100, and the
*ARM7TDMI Technical Reference Manual*, DDI 0029 revision G.  The functional
package is the differential oracle in tests; it is not a production dependency
or an execution backend.

The package is an explicitly educational, repository-owned ISA-level gate
topology.  It models settled combinational networks and persistent D flip-flop
state at instruction boundaries.  It does not claim Arm's proprietary
ARM7TDMI netlist, transistor topology, layout, pipeline timing, bus timing, or
silicon gate count.

The complete admitted transition boundary is exactly Spec 07ax: ARMv4T ARM
and Thumb-1 execution, 31 physical general-purpose registers, five SPSRs,
CPSR flags/mode/state, pipeline-visible PC operands, bank exchange, all admitted
data-processing/multiply/status/memory/multiple/control families, checked
64 KiB little-endian memory, exact trace bytes, and typed transactional
lifecycle.  Unsupported exceptions, coprocessors, EmbeddedICE/JTAG, caches,
AMBA, interrupts, peripherals, timing, power, and SoC integration remain
excluded.

## 2. Exact authoritative persistent topology

Every architecturally persistent bit is represented by one stable-Q DFF bit:

| Bank | DFFs |
|---|---:|
| 64 KiB unified memory | 524,288 |
| 31 physical GPRs, including PC | 992 |
| CPSR | 32 |
| five SPSRs | 160 |
| HALT | 1 |
| installed-image origin | 32 |
| installed-image length (0..65,536) | 17 |
| **Exact total** | **525,522** |

`FLIP_FLOP_COUNT` must derive this total from named bank widths.  Stable-Q is
the sole authority for snapshots, reads, fetch, observation, execution,
restore, reset, load, halt, run/execute results, and rollback.  A public or
private host-language architectural mirror may not steer execution.  Metadata
such as diagnostic gate counts is not architectural state and is excluded.

The repository's `logic-gates::sequential::register` primitive clocks every
changed bit through both phases.  Packed memory is permitted for scale only
when every read observes packed stable-Q and every write passes through the
same DFF clock helper.  Reset, load, restore, instruction commit, and rollback
must use that transition chain rather than assigning Q storage directly.

## 3. Independent decode and gate authority

Production code contains its own strict ARM and Thumb decoder.  It must not
depend on `arm7tdmi-simulator`, copy a functional result into gate storage, or
call an oracle through a helper.  The functional crate is dev-only.

ARM and Thumb field equality, fixed-bit masks, family selection, condition
selection, and destination enables are built from explicit gate equality,
AND/OR/NOT, and mux networks.  Reserved neighbors fail closed.  The selected
one-hot family/operation wires authoritatively drive transport width,
legality, executor routing, flags, write enables, memory width, PC control,
and mnemonic selection.  A second host `match` must not re-authorize an
operation after gate decode.

The ARM condition network consumes CPSR N/Z/C/V through primitive gates and a
16-way one-hot mux.  All arithmetic results and flags use the repository
arithmetic/gate primitives: fixed 32-stage ripple add/subtract, gate reductions
for zero, sign wires for N/V, mux-selected carry/shifter carry, fixed mux
barrel-shift stages, and fixed-round multiply accumulation.  Native host
arithmetic may be used only for checked host indexing, bounded ranges, trace
bookkeeping, and address metadata where the spec explicitly does not claim a
gate datapath.

Register banking, CPSR/SPSR selection, instruction-state choice, PC-visible
operand offsets, ARM/Thumb width selection, BX exchange, writeback, and
multiple-transfer destination enables are explicit decoded control/mux paths.
Every memory transfer validates its complete range before any DFF clocks.

## 4. Checked lifecycle and atomicity

The package owns gate-native `Arm7TdmiState`, `StepTrace`, `ExecutionResult`,
and typed `Arm7TdmiError` values compatible in meaning with Spec 07ax.
`new`, `reset`, `load`, `restore`, checked register/memory access, `step`,
`run`, and `execute` expose the complete boundary.

Each step snapshots stable-Q, fetches exact original bytes, performs strict
decode and the complete transition on disposable candidate state, validates
the result, then clocks one commit.  A failure before or during commit restores
the exact entry Q image.  Bounded `run` and `execute` are transactional across
the whole invocation.  Halt is a successful committed transition; a later
step is typed `Halted` and leaves all Q unchanged.

Test-only deterministic allocation/commit fault seams must cover snapshot,
trace retention/final-state materialization, and mid-commit rollback.  They
must be unavailable in production and must prove complete stable-Q recovery,
including memory, banked registers, CPSR/SPSRs, PC, HALT, and image metadata.

## 5. Differential and structural completion gate

Completion requires all of the following:

1. full-state and exact-trace differential execution against
   `arm7tdmi-simulator` for every admitted ARM and Thumb family and every typed
   fault class, including the functional completion matrix;
2. all ARM condition outcomes, shifter boundaries, add/sub carry/overflow,
   multiply edges, status masks, every bank/mode/SPSR path, ARM/Thumb exchange,
   pipeline-visible PC cases, every memory width/addressing mode, multiple
   transfers, overlap rejection, self-modifying trace capture, and target
   faults;
3. reset/load/restore/halt/post-halt/run/execute/limit/allocation/commit-fault
   lifecycle tests with full-state rollback;
4. exact inventory and two-way authority tests that deliberately corrupt any
   observer/candidate copy without changing Q, and deliberately change Q
   without updating an observer, proving Q wins in both directions;
5. parsed-token exact locks for the complete source-to-sink chains: decoder to
   one-hot family/operation wires, condition/shifter/ALU/flags, bank/status/PC
   muxes, memory/control routing, stable-Q reads, DFF clocks, commit, rollback,
   and public step/run/execute entry points;
6. anchored, compile-valid hostile full-source mutations for direct Q writes,
   direct host arithmetic/condition/decode authority, oracle substitution,
   helper indirection, dead gate anchors, post-gate overwrite, and post-commit
   overwrite; each mutation must compile before the validator rejects it;
7. no normal dependency on the functional simulator, no unsafe code, stable
   nonempty error text, README/CHANGELOG/BUILD/workspace wiring, and an explicit
   statement that this is not a proprietary silicon netlist;
8. package-scoped Rustfmt, strict Clippy and rustdoc, release tests, affected
   repository BUILD, lessons validation, and release line coverage above 80%;
   and
9. independent exact-clean-HEAD security/correctness review before publication.

## 6. Ordered follow-ups

Real exception/IRQ/FIQ entry and return, AMBA transactions, external aborts,
wait states, DMA, interrupt controllers, timers, peripherals, clock/reset
domains, cache behavior, physical pipeline timing, and SPICE/device descent
remain separate source-backed cells.  The first ARM7TDMI SoC composition may
use this educational core only after the shared bus/interrupt/DMA primitives
land; it must not convert this topology into a silicon-identity claim.
