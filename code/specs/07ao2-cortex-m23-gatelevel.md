# Layer 07ao2 — Arm Cortex-M23 Gate-Level Simulator

## 1. Status, sources, and claim boundary

**Status:** independently reviewed specification; implementation authorized.
The exact reviewed spec-only head is
`f191dc6fc2f2c0eacf2b8e414c3018eb13ffa955`.

This cell specifies `cortex-m23-gatelevel`, an educational architectural gate
companion to the checked `cortex-m23-simulator`. It is deliberately bounded to
the processor configuration in which the optional Security Extension is
absent. The modeled machine is therefore permanently Non-secure and executes
the complete inherited integer surface already admitted by Spec 07ao. This
profile is useful as a gate-authority proof without silently turning the
functional package's limited security hooks into a claim of complete
TrustZone hardware.

The normative public architecture sources are:

- Arm, *Cortex-M23 Devices Generic User Guide*, DUI 1095A,
  <https://developer.arm.com/documentation/dui1095/latest/>;
- Arm, *Cortex-M23 Processor Technical Reference Manual*, DDI 0550,
  <https://developer.arm.com/documentation/ddi0550/latest/>;
- Arm, *Armv8-M Architecture Reference Manual*, DDI 0553,
  <https://developer.arm.com/documentation/ddi0553/latest/>; and
- Arm, *Cortex-M23 Processor Datasheet*,
  <https://developer.arm.com/-/media/Arm%20Developer%20Community/PDF/Processor%20Datasheets/Arm%20Cortex-M23%20Processor%20Datasheet.pdf>.

Those sources establish the Armv8-M Baseline programmer-visible profile,
Thumb instruction transport, and the configurability of security and system
integration features. They do **not** publish the licensed Cortex-M23 netlist.
Consequently this package defines a repository-designed, ISA-level Boolean
topology. It is not Arm RTL, a reverse-engineered implementation, a transistor
count, or a claim about a particular Cortex-M23 revision, MCU, physical
layout, pipeline, power result, or silicon timing.

The repository's M0 and M3 gate cells provide the local implementation
conventions: `logic-gates` supplies Boolean and sequential primitives,
`arithmetic` supplies gate-built adders, packed stable-Q memory reconstructs
and clocks the repository DFF primitive, and the functional package appears
only as a development oracle. At the time of this specification the Cortex-M7
functional cell exists, but no Cortex-M7 gate package or `07an2` specification
is present on `origin/main`; this contract therefore does not invent or cite
an unavailable M7 gate topology.

## 2. Exact bounded functional profile

The gate cell implements precisely this configuration of Spec 07ao:

- `SecurityExtension::Disabled` for construction, reset, load, restore, step,
  run, and execute;
- permanent `SecurityState::NonSecure`, with all owned addresses attributed
  Non-secure;
- the same 64 KiB little-endian teaching memory and installed-image bounds;
- R0-R12, Non-secure MSP/PSP, LR, PC, APSR N/Z/C/V, IPSR, Non-secure PRIMASK
  and CONTROL, Thread/Handler mode, halt/sleep/event state, and checked
  lifecycle as the functional oracle; and
- the complete inherited Cortex-M0 execution surface accepted by Spec 07ao:
  `ShiftAddSubtract`, `Immediate`, `DataProcessing`, `SpecialData`,
  `LiteralLoad`, `RegisterMemory`, `ImmediateMemory`, `StackMemory`,
  `AddressGeneration`, `Miscellaneous`, `MultipleMemory`,
  `ConditionalBranch`, `UnconditionalBranch`, `BranchLink`, `Barrier`, and
  `SpecialRegister`.

This is a strict profile, not an accidental projection. A complete public
state includes the functional `CortexM23State` security fields, but the
excluded configuration has one canonical representation:

| Field | Required value |
|---|---|
| `security_extension` | `Disabled` |
| `security_state` | `NonSecure` |
| Secure MSP and PSP | `0x0001_0000` |
| Secure PRIMASK and CONTROL | clear |
| `sau_enabled` | false |
| `sau_all_non_secure` | true |
| all eight SAU descriptors | disabled, zero base/limit, NSC false |

These values are constant wires, not mutable architectural storage. Restore
rejects any noncanonical excluded field before clocking a DFF. The gate state
can therefore be converted losslessly to the functional oracle's complete
state, and every differential assertion compares the entire structure rather
than ignoring security fields. `attribute` always reports Non-secure;
SAU-configuration and security-transition APIs return the same typed
`SecurityExtensionUnavailable` boundary without mutation.

No Armv8-M-exclusive opcode is added here. SG, BXNS, BLXNS, exception entry or
return, TrustZone banking and attribution, stack-limit registers, MPU/IDAU,
NVIC/SysTick, interrupt delivery, debug/trace, bus transactions, caches,
prefetch, stalls, pipeline timing, clocks, reset synchronizers, power, and
vendor peripherals are excluded. Security-enabled gate execution requires a
later source-backed cell with its own storage inventory, decode proof, fault
priority, attribution, and transition tests.

## 3. Authoritative persistent topology

Every mutable bit in the bounded profile is owned by a repository D flip-flop.
The stable Q outputs are the sole architectural authority for reads, fetch,
decode operands, predicates, snapshots, traces, results, restore, rollback,
reset, load, and every commit. A public value struct or byte cache may observe
Q, but it must be reconstructible and must never feed execution or decide a
fault. There is no parallel authoritative host integer or Boolean.

| Component | D flip-flops |
|---|---:|
| 65,536-byte memory | 524,288 |
| R0-R12 | 416 |
| Non-secure MSP, PSP, LR, and PC | 128 |
| APSR N/Z/C/V | 4 |
| IPSR | 6 |
| Non-secure PRIMASK | 1 |
| Non-secure CONTROL nPRIV/SPSEL | 2 |
| Thread/Handler mode | 1 |
| Halt, sleep, and event | 3 |
| Installed-image origin | 32 |
| Installed-image length, including 65,536 | 17 |
| **Total** | **524,898** |

EPSR.T and every canonical excluded-security field in section 2 are fixed
wires. Derived caches such as a set of nonzero memory addresses may exist only
to make reset proportional to changed bytes; they are not counted state and
must be rebuilt from Q. Packed memory stores stable Q bytes, reconstructs the
full master/slave `FlipFlopState` for a selected write, and clocks low then
high through `logic_gates::sequential::register`, matching the established
repository convention. Reset, restore, program installation, direct writes,
instruction writes, and rollback all use that same clocked path.

`flip_flop_count()` must report exactly 524,898. Its conformance test must
derive widths from the concrete storage owners and independently sum them; a
test that merely repeats the exported total is tautological and fails this
contract. A stable educational gate estimate may use six primitive gates per
master/slave DFF plus a separately documented combinational allowance, but it
must be labeled an estimate rather than an Arm silicon count.

## 4. Gate-derived transition authority

The production package may use host code to allocate bounded snapshots,
iterate fixed networks, index storage after selection, sequence clock edges,
and format errors or traces. Host arithmetic, comparisons, matches, shifts,
or Boolean short-circuit expressions must not independently decide an
architectural result. The following paths are gate-authoritative:

1. **Transport and decode.** Little-endian halfword assembly, 16/32-bit width
   selection, all sixteen admitted `InstructionClass` lines, sub-opcode lines,
   fixed fields, and reserved-neighbor rejection are built from explicit bit
   wiring, equality/reduction gates, and muxes. A host dispatcher may route on
   one-hot settled lines; it may not re-decode the raw instruction.
2. **Register and stack selection.** Source and destination register muxes,
   high-register selection, LR/PC paths, active MSP/PSP selection, CONTROL
   masks, privilege checks, and register write enables consume Q and gate
   lines. Handler mode and SPSEL precedence match the functional cell.
3. **Integer data path.** Add, ADC, subtract, SBC, compare, negate, address
   generation, PC sequencing, and flag carry/overflow use fixed-width ripple
   networks from `arithmetic` and repository gates. AND/OR/XOR/BIC/MVN and
   zero/negative reductions use Boolean networks. LSL/LSR/ASR/ROR use staged
   mux networks, including zero, 31, 32, and greater-than-32 register-shift
   boundaries. MUL uses a fixed 32-round gated partial-product accumulator;
   no native multiplication supplies the architectural result.
4. **Flags and conditions.** Every N/Z/C/V write enable and value, carry
   preservation rule, signed overflow, and all conditional-branch predicates
   are derived from settled wires. Host ordering or native flag recomputation
   after the network is forbidden.
5. **Memory and control.** Visible-PC bases, BL/LR formation, Thumb-target
   validation, scalar/stack/multiple addresses and writeback, little-endian
   byte lanes, sign extension, alignment, nonwrapping owned-range checks, and
   memory write enables settle before any DFF clock. CPS, MRS/MSR, barriers,
   WFE/WFI/SEV, SVC, breakpoint handling, and the repository `BKPT #0` halt
   convention use gate decode and control lines.

The engine may assemble a fallible value-level candidate after those networks
settle. It commits only the selected candidate bits by clocking their DFF
banks. A decode, semantic, validation, allocation, or trace failure clocks
nothing. Production dependencies are limited to the repository gate and
arithmetic foundations; `cortex-m23-simulator`, `cortex-m0-simulator`, and
other CPU engines are development oracles only and have no production call
path.

## 5. Checked lifecycle and atomicity

The public checked surface mirrors the bounded functional profile: canonical
construction and reset; fallible complete snapshot; validated restore;
nonempty, halfword-aligned in-range image load; checked register and naturally
aligned byte/halfword/word memory access; event signaling; `step`; bounded
loaded `run`; and atomic load-plus-run `execute`.

Successful traces preserve exact raw little-endian bytes, width, mnemonic,
PC-before/after, constant Non-secure before/after attribution, and complete
before/after states. The 64-trace and 8 MiB complete-snapshot retention bounds
remain identical to Spec 07ao. Outer trace storage and both nested 64 KiB
snapshots are reserved fallibly before mutation.

Every functional-profile error is typed and atomic: invalid profile or
restore, empty/misaligned/out-of-range program, invalid register, halted or
sleeping step, misaligned/out-of-image fetch, misaligned/out-of-range data,
invalid Thumb target, unknown 16/32-bit instruction, unsupported breakpoint,
SVC, retention exhaustion, step exhaustion, and allocation failure. `run` and
`execute` restore every DFF and derived cache on any failure. Fault precedence
matches the functional oracle, including decode/operand validity before any
candidate memory effect. Fixed excluded-security calls return
`SecurityExtensionUnavailable` without changing DFFs.

## 6. Required conformance and anti-shortcut evidence

### 6.1 Functional differential

`cortex-m23-simulator` is a dev-dependency and is always constructed with
`SecurityExtension::Disabled`. Tests must compare exact results or errors,
raw bytes, width, mnemonic, PC transition, and the **entire** canonical
`CortexM23State` before and after each transition.

The generated matrix must:

- classify every 16-bit first halfword against the oracle and cover every
  admitted 32-bit prefix with all meaningful sub-opcode/field partitions and
  reserved immediate neighbors;
- execute every supported operation in all sixteen instruction classes, not
  just one representative per broad class;
- cover low/high registers, both Thread and Handler mode, MSP/PSP selection,
  privilege success and denial, every N/Z/C/V edge, every condition outcome,
  shift-count boundaries, multiply edge operands, every scalar memory
  width/sign/addressing mode, stack and multiple-transfer ordering and
  overlap, PC/LR bases, taken/not-taken control flow, barriers, special
  registers, WFE/WFI/SEV, BKPT/SVC, and halt; and
- seed nonzero values in every mutable state bank and changed memory bytes so
  a missing clock, wrong mux line, accidental reset, or partial rollback is
  observable.

### 6.2 Lifecycle, storage, and hostile-backend tests

Completion also requires:

1. reset/load/restore/direct-access/step/run/execute round trips over complete
   state, arbitrary valid origins, maximum image length, all invalid ranges,
   retention boundaries, post-halt/sleep behavior, and deterministic
   allocation-failure seams before snapshot, candidate, trace, and result
   allocation;
2. invalid canonical-security fields one at a time, proving rejection before
   any DFF or derived cache changes;
3. independent concrete-owner topology enumeration, stable-Q reset checks,
   and deliberate two-way divergence of every compatibility mirror to prove
   snapshots, execution, faults, and repair consume Q only;
4. pure truth-table/property tests for decode lines, add/subtract,
   comparison, condition, shift, multiply, address, byte-lane, flag, and
   write-enable networks, including legal neighbors of every invalid case;
5. a syntax-aware production-routing guard over the external file-backed
   state, gates, decoder, executor, validator, and crate-root commit path; and
6. hostile fixtures that the guard must reject: functional-oracle or other CPU
   execution in production, host add/subtract/multiply/shift/condition/range
   shortcuts, direct Q or packed-memory assignment, persistent host-state
   mirrors consumed by production,
   shadowed gate helper names, dead gate calls or dead dispatch, aliased,
   attributed, inline, or macro-generated routing modules, comment/string/
   character-literal confusers, expectation shadowing, candidate mutation
   after validation, post-dispatch overwrite, and a commit path that bypasses
   the reviewed DFF clocks.

The structural guard complements behavioral differential testing; neither is
accepted as a substitute for the other. Its own adversarial fixtures must
prove each rejected pattern actually reaches the parser/validator under test,
so a dead test backend cannot create false confidence.

## 7. Documentation, build, and publication gates

The implementation cell must add the package to the Rust workspace and
package `BUILD`, and update its README, changelog, root Rust package index,
root changelog, and this backlog without rewriting unrelated rows. The README
must repeat the no-proprietary-netlist boundary, exact 524,898-DFF inventory,
fixed Security-disabled profile, commands, test counts, and measured coverage.

Publication requires all of the following on a clean head descended from the
then-current `origin/main`:

- release package tests, the complete functional differential, topology and
  gate-integrity suites, and doctests;
- package-scoped Rustfmt, Clippy over all targets with warnings denied, and
  rustdoc with warnings denied;
- package BUILD plus affected workspace/BUILD integration and lessons
  validation;
- regenerated committed vectors or tables with a clean reproduction diff;
- more than 80% measured Rust line coverage, with a target above 90%, reported
  as both covered and total lines rather than a rounded percentage alone;
- `git diff --check`, exact-head clean-tree evidence, and independent
  correctness/security review before publication.

Only after these gates pass may the backlog row move from specified to local
complete. TrustZone/SAU, interrupt and exception integration, buses and SoCs,
and lower SPICE/device models remain separate follow-up cells with their own
public evidence and review.
