# VOS delivery backlog

Implementation queue for [VOS00](VOS00-verified-os-program.md), the verified OS
program. Reconcile fetched `origin/main` and open PRs before selecting each
item. Local work, an open PR and a verified merge are different completion
states, and this table records which one each row is in.

## Delivery rules

* One bounded change per PR.
* The spec is committed before tests and code.
* A security sub-agent review runs before every push.
* Babysit each PR to green, then enable squash auto-merge.
* After every merge, reprioritize: safety and correctness first, then missing
  validation, then critical-path prerequisites, then independent extensions.
* Log every discovery in "Discovered work" below the moment it is found, with
  its source. Do not leave a found defect in a PR's follow-up list.
* Late rows that hide large machine work (a PDP-11 MMU, an S/370 privileged
  slice, an Atlas or AGC simulator) are split into several PRs when scheduled;
  they are never one row's worth of work.
* The S-series teaching crates, including `os-kernel` (S07), are not touched.
  New crates use the `vos-*` and `emu-*` prefixes.

## Tracks

| Track | Meaning |
|---|---|
| E | Emulator framework: contracts, core, devices, boards, CPU adapters |
| L0-L12 | History ladder: one rung per row, numbered as in VOS00 section 6.1 |
| M | Modern: component pool, kernel model, HAL, spine, profiles, proofs |
| all | Spans the program (umbrella, contract model, linking) |
| CP | Critical path flag, shown beside the ID |

Ladder order is the chronology of the concept; build order is the order of
this table. They differ on purpose (VOS00 section 6.2).

## Backlog

| ID | Track | Title | Deliverable | Depends on | State |
|---|---|---|---|---|---|
| VOS-001 CP | all | Umbrella | VOS00 and this backlog | - | done (#19485) |
| VOS-002 CP | all | Component contract model | VOS01: anatomy, the eight rules, by-value steps, tiers, X1-X3 | 001 | in review (this PR) |
| VOS-003 CP | E | Emulator framework spec | EMU00: contracts, machine loop, board format, determinism, relation to SIM00, VER13 entries | 002 | queued |
| VOS-004 CP | all | Interface contracts and profile/board linking | VOS02: contract packages, manifests, UFCS rule, `-any` discipline, VER29 amendment text | 002 | queued |
| VOS-005 CP | E | Emulator core crate | `emu-core`: bus map and validator, machine loop, device traits, recording bus | 003 | queued |
| VOS-006 CP | E | First devices | Console/teleprinter and paper tape, with Warrant claims | 005 | queued |
| VOS-007 CP | L0 | EDSAC board and loader rung | EDSAC CPU adapter, board, an original initial-orders-style loader, loader contract | 006 | queued |
| VOS-008 | E | Tape and card devices; IBM 704 I/O orders | Devices and a simulator extension (spec first) | 006 | queued |
| VOS-009 | L1 | Batch monitor rung | 704 resident monitor; job-control and I/O-supervisor contracts | 008 | queued |
| VOS-010 CP | M | Scheduler contract | VOS10 and `vos-sched.warrant` (safety and fixed-priority tiers) | 002, 004 | queued |
| VOS-011 CP | M | Bitmap fixed-priority scheduler | `vos-sched-fp-bitmap`, exhaustive at N=8 unique priorities | 010, D002 | queued |
| VOS-012 CP | M | Most-general scheduler and a second implementation | `vos-sched-any` and a round-robin scheduler (safety tier); record that nothing else regenerates | 010 | queued |
| VOS-013 | E | Interrupt controller and timer devices | Generic contracts and two devices | 005 | queued |
| VOS-014 | E | Disk device; 8080 board | Block device, port console and disk, 8080 adapter | 006 | queued |
| VOS-015 | L7 | CP/M-style rung | BIOS-style HAL contract, flat file system contract, console loop booting from disk | 014 | queued |
| VOS-016 | L3 | Time-sharing rung | GE-225 board, timer, several teletypes, the round-robin scheduler from 012 | 012, 013 | queued |
| VOS-017 | L5 | Semaphore and layering rung | Semaphore contract; layered monitor on the PDP-8 | 013 | queued |
| VOS-018 | E | AGC simulator spec and crate | New CPU row; functional first | 003 | queued |
| VOS-019 | L4 | Executive and Waitlist rung | Priority jobs (same policy contract as 011), timed tasks, restart | 011, 018 | queued |
| VOS-020 CP | E | RV32 privileged slice 1 | Bus-generic RISC-V step; M and U modes; full trap CSRs | 005 | queued |
| VOS-021 CP | E | RV32 PMP, timer and UART board | PMP with exhaustive check claims; a QEMU-virt-like map | 013, 020 | queued |
| VOS-022 CP | M | Kernel K0 spec and executable model | VOS20 and `vos-kmodel` over the `-any` components | 012 | queued |
| VOS-023 | L6 | Message-passing rung | IPC contract (send, wait, answer; bounded buffers); PDP-11 nucleus | 013, 022 | queued |
| VOS-024 CP | M | Leaf components | Thread table, static capability table, IPC endpoint, PMP plan, partition table (one PR each) | 022 | queued |
| VOS-025 CP | M | HAL contract and RV32 HAL | VOS21 and `vos-hal-rv32` | 015 (shape), 021 | queued |
| VOS-026 CP | M | Spine and `rtos-min` profile (L11) | Boots on the RV32 board: two user tasks under PMP, with preemption | 024, 025 | queued |
| VOS-027 CP | M | Refinement monitor | Checks every trap against `vos-kmodel`; reused by rungs | 026 | queued |
| VOS-028 CP | M | Proof-route spec and solver capacity probe | VOS30 | 002 | queued |
| VOS-029 CP | M | Symbolic RV32 executor and first machine proof | VER13 section 4.3 item 2; `pick` proved on the image | 011, 028, WX-SP1 spec | queued |
| VOS-030 | L8 | Unix-style rung | PDP-11 MMU and devices (several PRs), then processes, files and pipes | 023 | queued |
| VOS-031 | M | Noninterference on the model; partitioned profile spec | Unwinding conditions; VOS41 | 022 | queued |
| VOS-032 | L9, L2, L10 | Deferred rungs | S/370 privileged and VMM; Atlas simulator and supervisor; 8086 microkernel; RV64 GPOS; H extension | later | queued |

Spec numbering is in VOS00 section 13.

## Discovered work

Rows are added here the moment work is discovered, with the source (a
review, a CI failure, a measurement, a PR) and the item it came from.

| ID | Found by | Description | Priority | State |
|---|---|---|---|---|
| VOS-D001 | VOS-001 security review | Tape, paper-tape, card and disk images and board definitions are untrusted input to the emulator. EMU00 must bound them: maximum image and record sizes, maximum board-definition size and range count, fail-closed parsing with no panics, and a fuzz target per parser (VER06). | P1: fold into VOS-003 (EMU00); implement in VOS-005/006/008/014 | queued |
| VOS-D002 | VOS-002 (VOS01 drafting) | No analyzer checks most of the component restrictions that VOS00 rules 2 and 7 and the footprint of section 4.4 rely on: VER08a does not require `no_std`, classes `alloc` as pure, and knows nothing of statics, interior mutability, `dyn` or function pointers, recursion, loop bounds, division, panic paths, macros or fixed-width types. VOS01 section 1.4 defines the rules SH1-SH15; build the lexical shape check (reusing the VER08a lexer) and the two image-level companions (no path to the panic handler, static stack bound). | P1: before VOS-011; VOS02 names the tool | queued |
| VOS-D003 | VOS-002 (VOS01 drafting) | VOS00 sections 4.5 and 4.7 call an implementation a "binding of" the contract's `.warrant`, but a `.warrant` stem is the package directory (VER29 section 3.1) and a document has no imports (VER29 section 15). Needs a generated, digest-checked per-implementation document (contract block copied verbatim plus an implementation block) and a drift check, and the interface-id decision must cover all four id kinds (functional, timing, resource, capability), three of which VER29 does not publish yet (slice 2). | P1: fold into VOS-004 (VOS02) and the VER29 amendment (LV-005) | queued (folded into VOS-004) |
| VOS-D004 | VOS-002 (VOS01 drafting) | VER29 section 7.1 classes a bench observation as `example`, so a complete emulator sweep of instruction counts (for example all 256 masks of the toy) cannot earn more than `example`. Need a bench-kind manifest entry that declares its domain complete, and a bound observer named in RV32 instructions. | P2: VOS-027 and VOS-028 | queued |
| VOS-D005 | VOS-002 (VOS01 drafting) | Claims over models alone (contract-level closure, tier ordering, spine invariants) are rejected by the current generator (`extract_claim_cases` answers W0702); VER04m V9 specifies a lemma kind with no published strength. Also VER04m E8 prints `closure:` and `init:` lines for `state=explicit` operations only, but by-value operations are `state=none`, so VOS00's X1 ("closure and init evidenced") has no report line. Needs the lemma kind and a by-value closure and init convention (small extension of VER04m E8). | P1: sequence with WX-S1; VOS-004 for the convention | queued |
| VOS-D006 | VOS-002 (VOS01 drafting) | The by-value binding cannot build a record with private fields (it needs public fields, VER04j struct line), which conflicts with VOS00 rule 2; VER04m E11's `fn:` builder exists for `inout` codecs only. Also missing: generator faults for tuple and variant results (`drop-state-update`, `const-variant:<Ctor>`) that the refinement and `-any` coverage claims need as negative controls, and a manifest and report field that prints configuration constants. | P1: fold into VOS-004 (VOS02), request via the VER backlog | queued (folded into VOS-004) |
| VOS-D007 | VOS-002 (VOS01 drafting) | VOS00 rule 4 and section 8 say `-any` covers every implementation for system theorems. True only for single-trace properties: refinement can remove nondeterminism in a way that breaks noninterference (a hyperproperty), and over `-any` no liveness or progress theorem holds. The partitioned-profile spec must evidence noninterference over deterministic policy-tier components or state how the safety tier's choice may depend on secrets. | P1: fold into VOS-031 | queued |
| VOS-D008 | VOS-002 (VOS01 drafting) | VOS00 section 8 lists three items re-proved on a swap (refinement, bounds, footprint). It omits the per-image artifacts that regenerate (monitor run, instruction counts, X3 image proofs), the profile manifest row and tier-eligibility check, and the role of contract ceilings in keeping the entry-level timing bound valid; section 4.5 also says a call site "names its implementation" where VOS01 section 1.2 has it name the interface through the profile. Amend VOS00 sections 4.5 and 8 after VOS01 lands. | P2: doc follow-up PR | done (this PR) |

## Decision log

### 2026-11-16: advisor review of the original direction

The owner's direction of 2026-11-16 (a real OS, proven components, replaceable
parts, one pool, our own emulator, a history ladder) was reviewed against the
repository before this spec was written. The review changed the design in six
places. Each change is recorded here with the fact that forced it.

1. **By-value steps instead of Mode E objects.** The original direction
   assumed stateful `inout` objects. VER04m (stateful objects) is spec only,
   its slice S3 is deferred, WX-D, WX-F and WX-G are unimplemented and WX-S1 and
   WX-SP1 are queued, so Mode E is not available. Components are pure
   `op(State, In) -> (Out, State)` steps, which VER30 section 4.5 item 1 names
   T0 today. `inout` is adopted at the HAL edge only, after WX-G and WX-S1.
2. **Binary proofs instead of Rust-to-WIR.** The VER25a Rust subset has no
   struct, enum, match, while, trait or impl, so kernel Rust cannot be lowered
   to WIR. The proof route is a refinement monitor over the booted image now,
   and symbolic RV32 execution of the compiled kernel entries later (VER13
   section 4.3 item 2).
3. **Static configuration first.** Dynamic capabilities, revocation and
   retype need the deferred derived inductive row and an arrays theory. The
   RTOS, flight and separation profiles are static; untyped and retype are
   GPOS only. ISA order is RV32 with PMP, then RV64 Sv39, then the H
   extension; the PMP-partitioned kernel is the separation-kernel deliverable
   in the meantime.
4. **VER29 plus contract packages plus `-any` for replaceability.** Loose
   coupling needs a contract that outlives any implementation. A contract
   package owns the Warrant document, implementations bind it, each interface
   ships a most-general `vos-<iface>-any`, and system theorems are evidenced
   over `-any`. VER29 ties ids to the implementing package, so the
   interface-id decision is open (VOS00 section 4.7) and the amendment goes
   through LV-005, never a fork.
5. **The emulator framework replaces the privileged-simulator prerequisite.**
   The original plan treated "privileged simulator features" as a prerequisite
   to finish elsewhere. The repository has 26 functional and gate-level ISA
   pairs but none with a bus-generic step, devices or PMP, and several defer
   I/O or privilege. An owned, verified emulator framework (EMU series, track
   E) is now part of this program, using adapters over audited simulators.
6. **The history ladder was added.** Rungs L0 to L12 recreate OS ancestors on
   period machines and feed the pool. Build order differs from chronology
   because cost differs. Rungs that need a missing simulator (Atlas, AGC) are
   late, stand-in machines are labelled, and every historical fact carries a
   source or an `[unverified]` mark.

### 2026-11-16: VOS00 reconciled with the VOS01 contract model

Findings F1-F9 of VOS01 section 9 were applied to VOS00 in the same PR rather
than left as a follow-up: section 8 gets the full re-prove list, sections 4.5 and
4.7 the interface-through-profile call sites, generated per-implementation
documents and four id kinds, rule 4 and section 8 the single-trace limit, section
9 the by-value X1 wording, the `example` cap on emulator measurements and the
configuration-field gap, and rules 2 and 7 and the footprint are marked reviewed,
not checked, until the shape check exists. Backlog consequences: VOS-D008 done;
D002 gates VOS-011 because the analyzer must exist before the first component
code; D003 and D006 are folded into VOS-004; D002, D004, D005 and D007 stay
queued.
