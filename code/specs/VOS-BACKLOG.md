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
| VOS-002 CP | all | Component contract model | VOS01: anatomy, the eight rules, by-value steps, tiers, X1-X3 | 001 | done (#19491) |
| VOS-003 CP | E | Emulator framework spec | EMU00: contracts, machine loop, board format, determinism, relation to SIM00, VER13 entries | 002 | done (#19494) |
| VOS-004 CP | all | Interface contracts and profile/board linking | VOS02: generated per-implementation documents with contract-owned obligations (R1-R9), the ghost face (SH17, SH18), profile and lock manifests, SH16, the four id kinds, the VER29 amendment text (AM1-AM5) and an acceptance spike | 002 | in review (this PR) |
| VOS-005 CP | E | Emulator core crate | `emu-core`: bus map and validator, machine loop, device traits, recording bus | 003 | queued |
| VOS-006 CP | E | First devices | Console/teleprinter and paper tape, with Warrant claims | 005 | queued |
| VOS-007 CP | L0 | EDSAC board and loader rung | EDSAC CPU adapter, board, an original initial-orders-style loader, loader contract | 006 | queued |
| VOS-008 | E | Tape and card devices; IBM 704 I/O orders | Devices and a simulator extension (spec first) | 006 | queued |
| VOS-009 | L1 | Batch monitor rung | 704 resident monitor; job-control and I/O-supervisor contracts | 008 | queued |
| VOS-010 CP | M | Scheduler contract | VOS10 and `vos-sched.warrant` (safety and fixed-priority tiers) | 002, 004 | queued |
| VOS-033 CP | M | Document generation and drift check | `vos-manifest` (contract file parser) and `vos-link gen-doc` / `check-doc`: assemble `code/specs/<impl>.warrant` from the contract package's `.part` files and the implementation block, marker digests, strict single-pass marker parser (no `-- vos02:` or CR in parts, each marker once, `carries` computed from parsed claims), path containment and size caps in `vos-manifest`, regeneration check, VOS02 negative controls NC1, NC2, NC17, NC18 | 004, 010 | queued |
| VOS-011 CP | M | Bitmap fixed-priority scheduler | `vos-sched-fp-bitmap`, exhaustive at N=8 unique priorities | 010, D002, 033 | queued |
| VOS-012 CP | M | Most-general scheduler and a second implementation | `vos-sched-any` and a round-robin scheduler (safety tier); record that nothing else regenerates | 010, 033 | queued |
| VOS-034 CP | M | Profile manifest check and SH16 | `vos-manifest` (profile and lock parsers) and `vos-link check-profile` (eligibility, `needs` subset of `carries`, ceilings, config pins, board ISA, read-only in CI), `vos-link build` (the only sanctioned image build: argv array, `--locked --offline`, scrubbed environment, isolated config, argv and log bound into the lock), `vos-link sh16` steps 1-8 (preconditions including resolver 2/3, environment and `.cargo/config*` pins, closed world with no `build.rs` or proc macro, per-unit features against a hard-coded policy, compiled `--cfg` allowlist from the verbose build log, interim symbol-table and strings check of the image) and `vos-link check-owners`; the link-map origin check (step 9) arrives with VOS-026; VOS02 controls NC3-NC11, NC15-NC20 and S1-S13 | 012, 033 | queued |
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
| VOS-026 CP | M | Spine and `rtos-min` profile (L11) | Boots on the RV32 board: two user tasks under PMP, with preemption; SH16 step 9 (symbol origin on the link map) and the image-level shape companions | 024, 025, 034 | queued |
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
| VOS-D001 | VOS-001 security review | Tape, paper-tape, card and disk images and board definitions are untrusted input to the emulator. EMU00 must bound them: maximum image and record sizes, maximum board-definition size and range count, fail-closed parsing with no panics, and a fuzz target per parser (VER06). | P1: fold into VOS-003 (EMU00); implement in VOS-005/006/008/014 | resolved in EMU00 spec; implementation in VOS-005/006/008/014 |
| VOS-D002 | VOS-002 (VOS01 drafting) | No analyzer checks most of the component restrictions that VOS00 rules 2 and 7 and the footprint of section 4.4 rely on: VER08a does not require `no_std`, classes `alloc` as pure, and knows nothing of statics, interior mutability, `dyn` or function pointers, recursion, loop bounds, division, panic paths, macros or fixed-width types. VOS01 section 1.4 defines the rules SH1-SH15 (SH3 bans statics outright; SH9 covers variable shift amounts; SH15 bans `include*!`, `#[path]`, `#[link_section]`, `#[no_mangle]`, `#[export_name]`, `build.rs` and the `links` key); build the lexical shape check (reusing the VER08a lexer as a library, in `vos-shape-check`; never a mode of `ca-capability-analyzer`) and the two image-level companions (no path to the panic handler, static stack bound). VOS02 adds SH17 (the ghost wrappers are exhaustive destructure, one UFCS call, restructure) and SH18 (no `ghost::` path in non-ghost class K source), and the check reads the class tag of VOS01 section 1.6 (K, C, S, M, P, H). Lexical half first (SH1-SH15, SH17, SH18, the class tag); the image-level companions arrive with VOS-026 and VOS-028. | P1: before VOS-011; VOS02 names the tool | queued |
| VOS-D003 | VOS-002 (VOS01 drafting) | VOS00 sections 4.5 and 4.7 call an implementation a "binding of" the contract's `.warrant`, but a `.warrant` stem is the package directory (VER29 section 3.1) and a document has no imports (VER29 section 15). Needs a generated, digest-checked per-implementation document (contract block copied verbatim plus an implementation block) and a drift check, and the interface-id decision must cover all four id kinds (functional, timing, resource, capability), three of which VER29 does not publish yet (slice 2). | P1: fold into VOS-004 (VOS02) and the VER29 amendment (LV-005) | resolved in VOS02 (section 1.3: option (c), rules R1-R9, the four-kinds table of section 8); implementation: VOS-033 (gen-doc, check-doc), VER-BACKLOG LV-008 (VER29 slice 7) |
| VOS-D004 | VOS-002 (VOS01 drafting) | VER29 section 7.1 classes a bench observation as `example`, so a complete emulator sweep of instruction counts (for example all 256 masks of the toy) cannot earn more than `example`. Need a bench-kind manifest entry that declares its domain complete, and a bound observer named in RV32 instructions. | P2: VOS-027 and VOS-028 | queued |
| VOS-D005 | VOS-002 (VOS01 drafting) | Claims over models alone (contract-level closure, tier ordering, spine invariants) are rejected by the current generator (`extract_claim_cases` answers W0702); VER04m V9 specifies a lemma kind with no published strength. Also VER04m E8 prints `closure:` and `init:` lines for `state=explicit` operations only, but by-value operations are `state=none`, so VOS00's X1 ("closure and init evidenced") has no report line. Needs the lemma kind and a by-value closure and init convention (small extension of VER04m E8). | P1: sequence with WX-S1; VOS-004 for the convention | queued |
| VOS-D006 | VOS-002 (VOS01 drafting) | The by-value binding cannot build a record with private fields (it needs public fields, VER04j struct line), which conflicts with VOS00 rule 2; VER04m E11's `fn:` builder exists for `inout` codecs only. Also missing: generator faults for tuple and variant results (`drop-state-update`, `const-variant:<Ctor>`) that the refinement and `-any` coverage claims need as negative controls, and a manifest and report field that prints configuration constants. | P1: fold into VOS-004 (VOS02), request via the VER backlog | resolved in VOS02 section 4 (private State plus a ghost face; SH17, SH18; the interim vocabulary of section 4.4); implementation: VOS-D002 (SH17, SH18) and VER-BACKLOG WX-SV (generator support) |
| VOS-D007 | VOS-002 (VOS01 drafting) | VOS00 rule 4 and section 8 say `-any` covers every implementation for system theorems. True only for single-trace properties: refinement can remove nondeterminism in a way that breaks noninterference (a hyperproperty), and over `-any` no liveness or progress theorem holds. The partitioned-profile spec must evidence noninterference over deterministic policy-tier components or state how the safety tier's choice may depend on secrets. | P1: fold into VOS-031 | queued |
| VOS-D008 | VOS-002 (VOS01 drafting) | VOS00 section 8 lists three items re-proved on a swap (refinement, bounds, footprint). It omits the per-image artifacts that regenerate (monitor run, instruction counts, X3 image proofs), the profile manifest row and tier-eligibility check, and the role of contract ceilings in keeping the entry-level timing bound valid; section 4.5 also says a call site "names its implementation" where VOS01 section 1.2 has it name the interface through the profile. Amend VOS00 sections 4.5 and 8 after VOS01 lands. | P2: doc follow-up PR | done (this PR) |
| VOS-D009 | VOS01 security review | Profile gate SH16: no `vos-*-any` crate and no `ghost` feature in the resolved dependency and feature graph of an `image`-class profile, checked on the resolved features (`cargo metadata` or `cargo tree -e features`), because feature unification can enable `ghost` through another crate. The oracle-write path lives in the ghost module only. VOS02 must key the check on the implementation kind recorded in the profile manifest (`-any` vs. production), not only on the crate-name pattern. | P1: fold into VOS-004 (profile manifests and validator); the image-side check also feeds VOS-028 | resolved in VOS02 section 7 (kind keyed on evidence, R6; closed world; features from the image build's own invocation); implementation: VOS-034 (check-profile and sh16 steps 1-6), image part with VOS-026 and VOS-028 |
| VOS-D010 | VOS-003 (EMU00 drafting) | VOS00 section 5.2 and VOS01 section 1.4 apply SH1-SH16 to the emulator, but the simulators it wraps use heap, `String` and `std::error`, and the emulator is not an image. EMU00 section 3.3 scopes the rules by crate class (C, S, M, P, H). VOS01 section 1.4 and the conformance checklist need an amendment saying which class each SH rule binds, and the shape check (VOS-D002) must read the class tag. | P2: doc follow-up with VOS-005 | done (this PR) |
| VOS-D011 | VOS-003 (EMU00 drafting) | Audited simulators lack accessors the adapters and snapshots need: `restore` for `ibm704-simulator`, `intel8080-simulator`, `ge225-simulator` and `pdp11-simulator`; incremental `feed_input` and `take_output` for `edsac-simulator` (its `output` Vec is unbounded and `set_input` rewinds); typed errors in place of `String` in `ge225-simulator`. Add each as an additive PR with a test that the existing API is unchanged. | P1: before VOS-007 (EDSAC) and VOS-008, VOS-014, VOS-016 for their ISAs | queued |
| VOS-D012 | VOS-003 (EMU00 drafting) | Every simulator step clones O(memory) state (8080 and Z80: twice per step for the trace; RV32I checked: three times; PDP-11: the whole simulator). Throughput of a long boot is bounded by it. Measure at VOS-007 before deciding on a non-cloning fast path. | P3: measure at VOS-007 | queued |
| VOS-D013 | VOS-003 (EMU00 drafting) | No row builds the PDP-11 machine work (traps and vectors, interrupts, I/O page, bus-generic `emu-cpu-pdp11`) that VOS-023 (L6) needs; VOS-030 is only the later MMU and devices. `pdp11-simulator` has no traps, interrupts or devices. Add a row before VOS-023 and make 023 depend on it. | P1: before VOS-023 is scheduled | queued |
| VOS-D014 | VOS-003 (EMU00 drafting) | VER13 section 5.2 `family` list lacks `emu`, and `mc.isa.model-faithful` already exists for RISC-V, so the new `mc.<isa>.model-faithful` ids must refine it and `emu.board.<name>.faithful` needs the family added. One-word schema amendment through the catalogue's own process (AS-001a rules AC-14). | P2: with VOS-005 | queued |
| VOS-D015 | VOS-003 (EMU00 drafting) | The 704 FP path (`powi`, `round_ties_even` on host `f64`) and `RiscVSimulator` host ecalls (`f64` services) make deterministic replay host-dependent. Audit and make them integer, or keep the per-host caveat in every report that replays those models. | P2: before VOS-027 claims cross-host replay | queued |
| VOS-D016 | VOS-003 (EMU00 drafting) | VOS00 sections 2.3 and 5.4 disagree with the code: EDSAC and GE-225 hold their peripherals inside the simulator (not hook-only); 8080 and Z80 expose latches (adapter must peek); `riscv-simulator` has `mscratch`, fetches only inside the loaded program, reports faults as errors not traps, and `mret` sets only `MIE`; Z80 has `interrupt` and `nmi`, the 8080 no delivery. Amend VOS00 after EMU00 lands. | P2: doc follow-up PR | done (this PR) |
| VOS-D017 | VOS-003 (EMU00 drafting) | Model-fidelity gaps to list in the `mc.<isa>.model-faithful` entries and to fix in the adapters: Z80 has no EI one-instruction delay; 8080 has no interrupt delivery (so VOS-014/015 can have no timer interrupt); the Z80 adapter (`emu-cpu-z80`, the L7 "then z80" step) has no backlog row. | P3: schedule with VOS-015 | queued |
| VOS-D018 | VOS-003 (EMU00 drafting) | VER06 has no coverage instrumentation (its audit item 10), so fuzzing the `emu-image` parsers is black-box. `emu-image` should expose an edge counter behind a `cov` feature, or the VER06 backlog should supply a source. | P2: VOS-005 | queued |
| VOS-D019 | VOS-003 (EMU00 drafting) | VOS-016 (L3) needs several teletypes on the GE-225; the simulator has eight controller plugs but no row builds a multi-terminal device behind them. Add it to VOS-016's scope or to VOS-013. | P2: before VOS-016 | queued |
| VOS-D020 | EMU00 security review | Known guest-reachable panic paths in class M models: `IBM704Simulator::reset` calls `expect`; `Rv32ISimulator`'s CSR arm contains `unreachable!()`. Remove or guard them in the adapters (or additively upstream), and audit every wrapped simulator for `unwrap`, `expect`, indexing and `unreachable!` reachable from guest bytes. `emu-host` contains the rest with `catch_unwind` (EMU00 section 7.5). | P2: with each adapter (VOS-007, 008, 014, 016, 017, 020) | queued |
| VOS-D021 | VOS-004 (VOS02 acceptance spike) | Today's evidence generator cannot bind the natural form of a by-value step: `export=` takes a bare identifier resolved through `use <crate>::*`; no fault applies to a tuple result whose second component is not an integer, a record of `Bool`s, a nullary scalar or a scalar with composite parameters (and every `op` needs one); a negative control attaches to a claim by its first-called operation; `field-zero` is inert on a zero initial state; the evidence manifest has no `features`, `cfg` or `config` field. Filed as VER-BACKLOG WX-SV (item 2 is P1) and LV-008 (AM4); the interim projection vocabulary of VOS02 section 4.4 is the workaround, and its sunset changes the obligations digests (a planned contract-text change). | P1: item 2 of WX-SV before VOS-012 needs negative controls for the `-any` coverage claims | filed (WX-SV); workaround in VOS02 |
| VOS-D022 | VOS-004 (VOS02 acceptance spike) | The generated evidence harness needs `coding_adventures_sha256` and `coding_adventures_test_bootstrap` as dev-dependencies, and VOS01 section 1.4's SH12 does not say whether dev-dependencies count. VOS02 section 7 excludes them from the closed world (the image build resolves normal and build dependencies only) and adds the `[package.metadata.vos]` keys `kind` and `evidence-features`. State both in SH12 and in the shape check. | P2: with VOS-D002 | queued (folded into VOS-D002) |
| VOS-D023 | VOS-004 (VOS02 section 8.1) | VER29 section 7.2.1 withholds resource, timing and capability strengths for any used dependency with no contract of that kind, `core` included; every class K crate depends on `core`. A VER13 catalogue entry for `core` is needed before those strengths can print for a VOS component. Filed as VER-BACKLOG AS-008. | P2: before any resource or timing strength is claimed for a VOS component | filed (AS-008) |
| VOS-D024 | VOS-004 (VOS02 sections 5.2 and 5.4) | The lexical audit of the `impl Profile` block (one impl per profile type, no path into an implementation crate from the spine) and the match between UFCS call sites and `site_<entry>_<step>` claims have no checker; the rules are written and checked by hand. Add them to `vos-shape-check` (second half) with VOS02 controls NC12 (block part) and the stale-claim case. | P2: with the spine (VOS-026) | queued |
| VOS-D025 | VOS-004 (re-checking the renamed VOS01 listings) | The generator rejects the linear scan's builder: `cells_of` (VOS01 section 5.4) calls `fixed<8>(...)` and generation stops with W0701 "call to `fixed`", although the document type-checks. The toy's second implementation (VOS01 section 5.4) cannot be generated until the builder is rewritten without `fixed<8>` or a WX request lands, and a byte-array state in VOS-012's second scheduler would hit the same limit. Also, a claim that calls no operation (the contract lemmas) is W0701 (VOS-D005), so the contract block's lemmas stay unevidenced. | P2: before a second implementation of any interface is evidenced | queued |
| VOS-D026 | VOS-004 security review (VOS02 section 7.6) | The authority files (`profile.lock`, every `*.part`, `contract.vos`, `[package.metadata.vos]`, `profile.vos`) need CODEOWNERS rules and branch protection, because a regenerated lock otherwise blesses tampering and a commit trailer is written by the change's own author. Add the CODEOWNERS entries and `vos-link check-owners`; the report prints the lock digest and the base-branch lock digest. | P1: with VOS-034, before the first image profile | queued |
| VOS-D027 | VOS-004 security review (VOS02 sections 7.2-7.5) | The SH16 gate must be the shipped invocation: release CI either calls `vos-link build --build image` or has its exact argv pinned in the lock and checked by `sh16 --argv-file`. Audit the release workflows for any other image build, and for `RUSTFLAGS`, `.cargo/config*` and `RUSTC_WRAPPER` use. Spike facts: resolver 1 and `RUSTFLAGS` cfgs hide `ghost` from `cargo tree`; `--unit-graph` is nightly-only. | P1: with VOS-034 | queued |
| VOS-D028 | VOS-004 security review (VOS02 section 7.8) | Evidence-feature authority must be central, not self-declared: the central list lives in `vos-link`, and `vos-shape-check` cross-checks every `cfg(feature = ...)` and `cfg_attr` in a class K crate against `[features]` and `evidence-features` (SH18 extension). | P2: with VOS-D002 | queued (folded into VOS-D002) |

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

### 2026-11-16: VOS00 and VOS01 reconciled with the simulator ground truth

EMU00 section 2 was read from the simulator sources and disagreed with VOS00 in
places. VOS00 sections 2.3, 5.2 and 5.4 now state the real I/O and interrupt
shapes (EDSAC and GE-225 peripherals inside the simulator, 8080 and Z80 latches
with a peek, no 704 or PDP-11 I/O, the RISC-V simulator's five CSRs, errors
instead of traps, code-only fetch, no M, `mret` setting `MIE` only) (D016). VOS01
gets a new section 1.6 scoping SH1-SH16 by crate class K, C, S, M, P and H, with
class K the default and SH16 unchanged (D010).

### 2026-11-16: VOS02 decisions, and the accepted ordering

VOS02 (backlog VOS-004) settled the open interface decision and **reversed
VOS00 section 4.7's stated direction**. Ids stay stock VER29,
`contract.<impl-package>.<claim>`, published by the implementation; the contract
package owns the text (the contract block, one obligations block per tier, the
ceilings); each implementation's document is generated from them with digests and
a regeneration check; conformance is digest equality (rules R1-R9). VER29 is
amended only in its reach rules and its feature rule (slice 7, LV-008), never in
its id shapes, discharge conditions, strength rules or exact-linking rule. VER04n
(opaque types) does not solve the by-value private-state problem; it forbids the
shape, so a ghost face (a public-field state image with forwarding wrappers,
SH17 and SH18) is the workaround and WX-SV (by-value explicit state, item 2 at P1,
independent of WX-G) is the request. The acceptance spike ran the VOS01 toy
through today's generator (10 exhaustive claims, 328,960 cases) and found the
generator constraints now logged as VOS-D021 to VOS-D025.

Accepted ordering of the next work: **1.** VOS-004 (this PR). **2.** VOS-D002,
lexical half (SH1-SH15, SH17, SH18, the class tag): small, and the first code; it
and the VOS-033 tool start out checked, each shipping its negative controls before
anything relies on it. **3.** VOS-028 (VOS30 and the solver and enumeration
capacity probe). **4.** VOS-005 (`emu-core`), started once D002's class-tag
reader exists. The image-level companions of D002 (panic path, stack bound) are
deferred to VOS-026 and VOS-028. VOS-033 and VOS-034 follow, before VOS-011 and
with VOS-012 respectively.
