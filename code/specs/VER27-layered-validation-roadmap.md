# VER27 — Layered Validation Roadmap: Warrant from Specification to Gates

**Status:** Draft roadmap. Nothing here is implemented by this document, and no
code changes with it. It records the owner's vision for Warrant and layered
validation, maps the existing and planned work onto that vision, and proposes
an order. Where it proposes catalogue entries or backlog rows, those are later
slices with their own PRs.

**Parent:** [VER00](VER00-verified-shipping-program.md) (§7.5 assume-guarantee,
§7.5.2-§7.5.6 assumptions and owned layers, §10 phases 7 and 8).

**Builds on:** [VER04i](VER04i-polyglot-warrant-evidence.md) (polyglot lanes,
draft), [VER13](VER13-assumption-catalogue.md) (assumption catalogue and the
constant-time discharge ladder), [VER20](VER20-resource-timing-proofs.md),
[VER25](VER25-wir-core-interpreter.md), [VER25a](VER25a-rust-subset-grammar.md),
[VER25b](VER25b-rust-to-wir-frontend.md), [VER26](VER26-ct-compare-pilot.md),
[VER10](VER10-bit-blasting.md), [VER11](VER11-drat-proofs.md),
[VER19](VER19-symbolic-execution.md), [VER04j](VER04j-warrant-v1-extensions.md),
[VER04k](VER04k-warrant-v1-wave4.md), [LANG84](LANG84-grammar-driven-versioned-frontends.md),
[PY01](PY01-python-grammars.md), [CS01](CS01-csharp-grammars.md),
[JV01](JV01-java-grammars.md), [FS01](FS01-fsharp-grammars.md),
[CPU-SIMULATOR-ROADMAP](CPU-SIMULATOR-ROADMAP.md),
[RUST-CPU-SIMULATOR-BACKLOG](RUST-CPU-SIMULATOR-BACKLOG.md).

**Backlog:** the "Layered validation roadmap (VER27)" section of
[VER-BACKLOG](VER-BACKLOG.md) (rows LV-001..LV-007), plus the existing WD-*,
LP-*, SH-*, AS-*, WX-*, CAP-RT-* and CA-OG* rows this document orders.

**Direction (owner, 2026-11-12):** the vision in section 2, recorded in the
owner's terms and paraphrased only for structure. The same day the owner added
the layered-contract rules (P9, section 4.13), the Intel 4004 chain (section
4.15), the mandatory runtime jail (P10, section 6.1), the finding rules (P11
and P12, section 7.4) and the release standard (P13, section 8.1). They are
recorded in the decision log (section 11.1).

**Why the number.** The verification programme's child specifications are
numbered `VER01`..`VER26` (VER00 §11). `VER27` is the first unused number:
`VER28` is already cited by the layer diagrams of VER05, VER06 and VER07 as
part of the ship gate ("VER01 / VER28"), so it is taken. The repository's
roadmaps that belong to a numbered family keep the family number and add a
`-roadmap` suffix (`AOT00-native-aot-robustness-roadmap.md`,
`LANG74-lang-vm-frontend-coverage-roadmap.md`, `VLT00-vault-roadmap.md`), so
this file is `VER27-layered-validation-roadmap.md`. A bare name such as
`VER-ROADMAP.md` (by analogy with `VER-BACKLOG.md`) was rejected because a
number can be cited from backlog rows and catalogue `since` fields the way
every other VER document is.

---

## 1. Why this exists

The verification programme already has most of its parts written down, but
in different places and at different altitudes:

- VER00 describes the whole programme, including the idea that proofs chain
  down through owned layers (§7.5.6).
- VER04i describes how one Warrant file produces evidence in eighteen language
  lanes.
- VER13 names every assumption the two converted packages rely on and gives a
  four-rung ladder for one claim (constant time) from WIR to a netlist.
- VER-BACKLOG holds five separate "user direction" programmes (polyglot
  evidence, leaf conversion, bottom-up hardening and self-hosting, assumption
  reduction, Warrant extensions) as unconnected tables.
- LANG84 and the PY01/CS01/JV01/FS01 grammar specs describe versioned language
  front ends, but for the IIR compiler pipeline, not for verification.

Nothing says how these fit together, which one moves which layer of the
stack, or what order the owner's rules imply. This document does that. Think
of it as the map on the wall of a building site: each crew (thread) has its
own drawings, and the map shows which floor each crew is working on, what
it is waiting for, and what the finished building is meant to be.

---

## 2. The vision, as principles

The owner's vision, stated as thirteen principles. Each row says what the
principle rules out, because a principle that rules nothing out is a slogan.

| # | Principle | What it rules out | Where it is already written down |
|---|---|---|---|
| P1 | **One `.warrant` file is the single source of truth** for a component or a system. Warrant is a specification language for stating precisely what a component does. | Hand-written test oracles, per-language copies of the contract, evidence numbers typed into a report | VER04, VER04h ("evidence is never written by hand"), VER04i §0 |
| P2 | **One specification, evidence in every language.** From one `.warrant` file Warrant generates unit (example) tests, integration tests, property tests, fuzz tests, exhaustive tests and SAT/proof-based tests derived from the stated assumptions. One file tests the C port, the Rust port and every other port. | A port whose tests have nothing to do with the specification; a proof that exists in one language and is presented as covering the others | VER04i §1 (three layers), VER-BACKLOG "Polyglot Warrant-derived evidence" |
| P3 | **Equal rigour in every lane.** Every lane in `rust python typescript go ruby c cpp java kotlin csharp fsharp elixir lua perl haskell swift dart ocaml` gets the same level of rigour, not only Rust. | A permanent second class of lanes whose proof layer is "not-live" with no plan to make it live | New here as a hard rule; VER04i §10 states the gap without a plan |
| P4 | **Every assumption is explicit.** For languages such as Ruby and Python the starting assumption is that the language does what its own specification says. That assumption is recorded as a catalogue entry, and is later discharged by going one layer down (runtime or VM) or by coming from one layer up (a validated language specification). | An assumption that exists only in prose, or only in a reviewer's head; a label that hides the stack it was proved on | VER00 §7.5.2, VER13 §1 ("an assumption is not a defect; an unnamed one is") |
| P5 | **The long arc goes down the stack.** Validated building blocks (gates, netlists, CPU and ISA, compilers and language implementations) let a claim proved at a high level hold for real. Example: proving that `ct-compare` runs in constant time on an owned RV32I gate-level CPU instead of assuming everything below the source. | Stopping at the source level and calling the result final | VER00 §7.5.6, VER13 §4, VER-BACKLOG AS-* |
| P6 | **Warrant is validated in Warrant**, and self-hosted suites keep an **independent oracle** cross-check, because a harness cannot be the only judge of itself. | A tool whose only evidence is evidence it generated about itself | VER-BACKLOG "Bottom-up hardening and self-hosting" (trust anchor paragraph) |
| P7 | **Adoption beats features.** No Warrant feature is added that no real library uses. Feature work is justified by named adopters. | Landing a language feature ahead of the first package that needs it; ranking features by a survey estimate alone | New here as a hard rule; the leaf-adoption analysis (section 8) applies it |
| P8 | **Every version of a language is supported.** Rust grammars run from 0.1 to today, with macros, `cfg` and `derive` handled as Rust's "preprocessor". The same holds for every lane. | A front end that knows only the newest release; treating macro-generated code as invisible | LANG84 ruling 4 ("every tagged release of every language"), PY01, CS01, JV01, FS01, LANG82 |
| P9 | **Each layer is verified on its own, and every layer above takes that result as an assumption** (assume-guarantee; owner, 2026-11-12). Once a layer's contract is proved, that is enough for that layer. The layer above proves only how it composes the parts below, never the lower layers again. Three rules make this sound: the contract must cover everything the layer above relies on (including time), an upper layer reaches a lower one only through its proved contract, and the assumption must be exactly the claim the lower layer proved, linked by id | Re-proving a lower layer inside a higher one; a timing claim resting on a contract that only states functional behaviour; an assumption that paraphrases the lower claim; a helper outside the proved contract that the upper layer quietly relies on | VER00 §7.5.6 rule 1; section 4.13 here; backlog LV-005, LV-006 |
| P10 | **The runtime jail is mandatory** (owner, 2026-11-12). Every library's test program runs inside a jailed sandbox built from exactly its `required_capabilities.json`. Anything else that happens is a failure. This applies on top of all other validation | A conversion that skips the jail; a report that rises above `report-only` with no live runtime evidence; a gate that passes silently when it did not run | VER08b; section 6.1 here; backlog CAP-RT-002, CAP-RT-003, CAP-RT-1xx |
| P11 | **A proof refusal is a design finding first, and a tool gap second** (owner, 2026-11-12). Proving forces small, isolated, decomposable components that can be wired together. When the proof front end refuses code, the first question is whether the code should be shaped differently | Widening the front end to accept awkward code before asking whether the code is the problem | Section 7.4 here |
| P12 | **Every finding follows one protocol** (owner, 2026-11-12): backlog row, a test that fails before the fix, the fix, a Warrant rejection or claim that pins the behaviour, and a lesson. The owner expects hundreds of findings, some serious | A fix with no failing test first; a fix that nothing pins; a finding that leaves no lesson | Section 7.4 here; `lessons.py new` |
| P13 | **Nothing is published until it is proven, and what is published meets a very high standard** (owner, 2026-11-12). The owner holds all publishing until then. A publish gate must exist before the first release | A release whose only check is a tag format and a branch; publishing a package with no Warrant spec, no generated assurance report or no live jail evidence | Section 8.1 here; backlog LV-007 |

**The goal these principles serve:** validate, as fully as can honestly be
done, that the software works as its Warrant file defines, and write down
every assumption that remains. "Honestly" matters more than "fully": a short
list of named, checked assumptions is a better result than a long list of
proofs whose assumptions nobody can cite.

Two consequences follow directly and are used throughout:

1. **Tests observe; proofs model.** A test that runs the real export on the
   real runtime observes what that runtime did. It needs no assumption about
   the language's semantics, only that the run happened as recorded and on
   the toolchain it names. A proof reasons about a *model* of the code (WIR),
   so it needs the assumption that the model means what the language means,
   and that the runtime implements it. This is why the per-language
   assumptions of section 5 matter to proofs first and to tests hardly at all.
2. **Every owned layer is written in Rust.** The repository's own VMs,
   compilers, CPU simulators and gate models are Rust crates. So once a lane
   runs on an owned layer, its "one layer down" route converges on the Rust
   lane's ladder: whatever the Rust ladder proves about the owned stack, every
   lane that runs on that stack inherits. Section 5.3 uses this.

---

## 3. Vocabulary

**Validated block.** A component at one layer whose own claims are stated in
a Warrant file (or, below the source level, in the equivalent formal object:
an ISA step function, a netlist) and backed by checked evidence. A validated
block turns the assumptions of the layer above it into claims of its own
(VER00 §7.5.6 rule 1: the assumption and the claim are the same formula).

**Layer.** A level of the stack. This roadmap uses ten layers (section 4),
coarser than VER13's eight stack keys at the bottom and finer at the top.
The mapping is fixed:

| Roadmap layer | Name | VER13 layer key | VER13 families consumed here |
|---|---|---|---|
| L0 | physical | `phys` | `phys` |
| L1 | gate primitives | `gate` | `gate` (primitives, flip-flops) |
| L2 | netlist | `gate` | `gate` (netlist, memory cut, clocking) |
| L3 | CPU and ISA | `mc`, `uarch`, `gate` | `mc.isa.*`, `uarch`, `gate.isa-equivalence` |
| L4 | compiler and back end | `cc`, `mc` | `cc`, `mc.backend.*` |
| L5 | language runtime or VM | `cc` (proposed, section 5.2) | none today; proposed `cc.<lane>.*` |
| L6 | WIR | `wir` | `wir`, `tcb` |
| L7 | source | `src` | `src`, `att`; proposed `src.<lane>.*` |
| L8 | Warrant specification | `spec` | `spec`, `gov.catalogue.integrity` |
| L9 | system and integration | none | none |

Two families sit beside the ladder rather than on it, as in VER13: `os`
(allocator, kernel, vDSO, runtime-gate limits) and `gov` (approval
governance). Section 4.11 places them.

**Lane.** One of the eighteen implementation languages above. A lane is
complete for a package when every evidence kind of section 6 is live for it.

**Route.** How an assumption is discharged. *Down*: a lower layer that we own
proves the assumption as its own claim. *Up*: a validated specification of the
layer above (for a language, a checked language specification) replaces "the
language does what its document says" with "the language does what this
checked specification says". *Across*: conformance tests of the assumption on
the third-party layer we actually run on; this moves an entry from `assumed`
to `evidenced`, never to `discharged` (VER13 §2).

**Adopter.** A real package in `code/packages/` whose Warrant conversion needs
a feature. Test fixtures are not adopters.

**Stack.** The concrete artefact chain a guarantee is about (VER13 §4.3,
VER00 §7.5.6 rule 2): `host-x86_64` for today's `rustc` build, `owned-rv32i`
for the owned compile, and, by extension, one stack per lane and toolchain.

---

## 4. The layer ladder

```
 L9  system / integration    packages composed; one package's claims discharge another's assumptions
 L8  Warrant specification   .warrant files; claims, models, labels, bounds
 L7  source                  per-lane front ends over versioned grammars; subsets; analyzers
 L6  WIR                     the proof model; symbolic execution; SAT; DRAT
 L5  runtime / VM            CPython, Node, JVM, CLR, BEAM ... or an owned VM
 L4  compiler / back end     rustc+LLVM, gcc ... or owned wir-to-riscv
 L3  CPU / ISA               riscv-simulator (ISA model), RV32I step function
 L2  netlist                 HNL traced from riscv-gatelevel; memory cut at the port
 L1  gate primitives         logic-gates, arithmetic; transistors (CMOS)
 L0  physical                silicon, power, EM: the declared floor
```

A claim is made at L8 and proved at L6 or L7. Each layer below that either
*assumes* what the layer above needs (a catalogue entry) or *proves* it (a
validated block). The work of the long arc is to turn rows of the first kind
into rows of the second, from the leaves upward and from L6 downward. The
following subsections give, for each layer: what a validated block is, which
catalogue assumptions it discharges, what already supplies it, and what is
missing. Counts of catalogue entries are from `code/assumptions/catalogue-v1.json`
(84 entries, VER13 §3.9).

### 4.1 L0 physical

- **Validated block:** none; this is the floor. Software cannot discharge it.
- **Discharges:** nothing. It is where `phys.silicon.realization` and
  `phys.side-channels.out-of-scope` stay, with status `floor`.
- **Supplied by:** nothing, by design.
- **Missing:** nothing. The rule is only that these two entries are always
  printed under every `proved-to-gate-level` label (VER13 rule AC-2).

### 4.2 L1 gate primitives

- **Validated block:** each primitive the netlist uses (`and`, `or`, `not`,
  `xor`, `mux`, full adder, D flip-flop) is shown, exhaustively over its
  inputs, to compute its truth table in the CMOS model.
- **Discharges:** `gate.primitives.correct` (VER13 rung R4).
  `gate.sequential.ideal-dff` stays: it is an abstraction of timing, not a
  Boolean fact.
- **Supplied by:** `logic-gates`, `arithmetic`, `transistors` (CMOS and TTL
  gate construction).
- **Missing:** the R4 comparison itself (VER13 §4.5), and a Warrant file for
  `logic-gates`. VER00 §12 already names `logic-gates` as a candidate first
  package through the gate, so it is a natural self-hosting adopter
  (section 7, SH thread).

### 4.3 L2 netlist

- **Validated block:** a netlist object (cells and wires, not a Rust
  evaluation) for one CPU step, with the memory cut at its port, and a
  structural proof that each instruction takes exactly one clock step.
- **Discharges:** `gate.model.is-netlist`, `gate.memory.abstraction`,
  `gate.clocking.one-instruction-per-cycle`.
- **Supplied by:** `riscv-gatelevel` (RV32I; 525,505 D flip-flops; decode,
  ALU, shifts, comparisons, addresses and branches through repository gates;
  differential against `riscv-simulator` on 256 Python full-state vectors),
  `gate-netlist-format` (HNL), `hdl-ir`, `hdl-elaboration`, `synthesis`.
  There are 44 `*-gatelevel` crates in all (RUST-CPU-SIMULATOR-BACKLOG).
- **Missing:** the netlist itself. Today the gate crates *call* gate functions;
  nothing records the wiring. VER13 §4.4 prefers a tracing gate backend that
  records each primitive application as an HNL cell. VER18 (hardware
  equivalence checking, k-induction) has no document yet. The CPU backlog's
  completion contract also allows host integers "for control flow, addresses,
  and trace bookkeeping", so each package's netlist claim needs the
  gate-integrity audit (RUST-CPU-EXPANSION-BACKLOG) before it is relied on.

### 4.4 L3 CPU and ISA

- **Validated block:** a bit-level RV32I step function (the ISA as a Boolean
  function) and a DRAT-certified miter showing the L2 netlist computes it for
  every state and instruction.
- **Discharges:** `gate.isa-equivalence`, `mc.isa.timing-data-independent`,
  `mc.isa.model-faithful` (no longer needed once the netlist is compared with
  the bit-level model), and the five `uarch.*` entries structurally (in-order,
  non-speculative, cache-less core).
- **Supplied by:** `riscv-simulator`, `riscv-encoder`, `riscv-gatelevel`,
  `cdcl-sat`, `bit-blaster`, `drat-check`.
- **Missing:** the symbolic RV32I executor (VER13 §4.3 deliverable 2, shared
  with L4), the step-equivalence miter, and the bounded monolithic cross-check
  (VER13 §4.4 strategy B).

### 4.5 L4 compiler and back end

- **Validated block:** a compiler small enough to translation-validate on
  every run. For the constant-time ladder that is `wir-to-riscv` (new): one WIR
  instruction to a fixed RV32I sequence, a source map, and a per-function
  DRAT-certified miter between WIR paths and machine paths.
- **Discharges:** `compiler.preserves_timing` and its refinements
  (`cc.llvm-ir.no-secret-branch`, `cc.llvm-ir.addresses-public`,
  `cc.volatile-honoured`), `mc.backend.no-secret-branch`,
  `mc.backend.addresses-public`, and `cc.version-pinned`, all *for the owned
  stack only* (rule AC-7). On the host stack, R1 (IR and assembly taint
  checks on a pinned toolchain) moves the same entries to `evidenced`.
  `cc.caller-context` is `declared` and stays.
- **Supplied by:** host `rustc`/LLVM (trusted); `riscv-backend` (a reference
  for encoding, not the lane: it emits `mul`/`div`); the IIR back ends
  (`iir-to-llvm`, `iir-to-wasm`, `iir-to-jvm-class-file`, `iir-to-cil-bytecode`,
  `iir-to-beam`) for other pipelines.
- **Missing:** `wir-to-riscv`, the R1 taint checker (AS-003a), and the label
  `proved-to-isa` in the report.

### 4.6 L5 language runtime or VM

- **Validated block:** a runtime or VM with its own Warrant file whose claims
  are, word for word, the catalogue entries the lane's packages assume
  (VER00 §7.5.6 rule 1): integer semantics, sequence semantics, the standard
  library calls a package uses.
- **Discharges:** the proposed per-lane `cc.<lane>.implementation-conforms`
  entries (section 5.2) for the owned stack; for `rust` it is the
  `os.allocator.size-only` and `os.std.runtime-benign` entries that `std`
  touches. VER13 has no layer for this today (section 10, conflict C2).
- **Supplied by (third-party, assumed):** CPython, CRuby, Node.js, the JVM,
  the CLR, BEAM, GHC's runtime, PUC Lua, perl, the Dart VM, the OCaml runtime,
  the Go runtime, the Swift runtime. **Supplied by (owned, candidates):**
  `python-iir-compiler`, `ruby-iir-compiler`, `perl-iir-compiler` and
  `javascript-iir-compiler` (LANG79-LANG81, LANG84) with `vm-core`,
  `lang-runtime-core`, `dynval-runtime` and `lang-aot`; the Python VM of PY02;
  `jvm-simulator` (java, kotlin); `clr-simulator` (csharp, fsharp);
  `wasm-runtime`.
- **Missing:** catalogue entries for any lane but Rust; per-lane conformance
  tests of those entries (VER00 §7.5.3 requires them; none exist); a Warrant
  file for any owned runtime.

### 4.7 L6 WIR

- **Validated block:** a lowering of real source to WIR, differentially
  tested against the real runtime, plus the proof machinery (symbolic
  execution, bit-blasting, CDCL, DRAT) whose own correctness is audited and,
  in time, self-hosted.
- **Discharges:** the twelve `wir.*` entries (lowering faithfulness, inlining,
  cost model, provenance, bounded unrolling, heap and depth bounds, trivial
  miters, obligation binding) and the six `tcb.*` entries (DRAT checker,
  bit-blast encoding, symbolic engine, report checker, manifest honesty,
  toolchain trust).
- **Supplied by:** `warrant-ir` (VER25, VER19, VER20), `bit-blaster` and
  `constraint-engine` (VER10), `cdcl-sat` (VER09), `drat-check` (VER11),
  `ca-assurance-report` (VER01). The audits LH-001, LH-003, LH-005 and LH-006
  are merged (#19076, #19075, #19081).
- **Missing:** non-trivial 2-safety miters (WD-005; every certificate today is
  the collapsed `x and not x` refutation, `wir.proof.trivial-miter`);
  obligation binding (WD-009); heap and depth bound proofs; VER12
  verification-condition generation; VER10's EUF, LIA and array theories.

### 4.8 L7 source

- **Validated block:** for each lane, a front end that parses every release
  of the language from a versioned grammar (LANG84), restricts the package to
  a named subset whose meaning the catalogue pins, and lowers it to WIR; plus
  that lane's capability analyzer.
- **Discharges:** `src.rust.subset-conformance`, `src.rust.int-semantics`,
  `src.rust.safe-bounds-checks`, `src.rust.unsafe-region.sound`,
  `src.panic.paths-public`, the analyzer caveats `src.analyzer.G-1`..`G-9`,
  `src.analyzer.correct`, the two `src.effects.*` entries,
  `src.registry.excluded-rows`, the nine `att.*` attestations (by building the
  tools that produce them), and, once proposed, the per-lane `src.<lane>.*`
  entries (section 5).
- **Supplied by:** `warrant-ir/src/rust_subset` (VER25a, hand-written; VER25b
  lowering), `ca-capability-analyzer` (VER08a), `capability-runtime-conformance`
  (VER08b), the versioned grammars under `code/grammars/` (section 5.4), and
  the generic `grammar-tools`, `lexer` and `parser` crates.
- **Missing:** versioned Rust grammars (planned; section 7, thread T3); the
  Rust proof subset over them (LP-001a); a WIR front end for any other lane
  (VER14, not yet written); capability analyzers for other lanes (VER08);
  subset enforcement outside Rust (VER00 §7.5.4).

### 4.9 L8 Warrant specification

- **Validated block:** a Warrant file whose claims are non-vacuous, consistent
  with its models, strong enough that every planted fault is caught, and
  complete over the public symbols; and a Warrant toolchain that is itself
  specified in Warrant with an independent oracle.
- **Discharges:** `spec.secret-labels.complete`, `spec.obs-model.complete`,
  `spec.platform.declared-none`, `spec.corpus.ct01-faithful` (by negative
  controls and corpus diffs), and `gov.catalogue.integrity` (by the checker).
  `spec.obs-model.public-length` is `declared` (part of the claim) and stays.
- **Supplied by:** `warrant-grammar` (VER04a-VER04k), the TypeScript
  `warrant-compiler` (VER04g), `ct-compare.warrant`, `gf256.warrant`, the
  standard-library Python validators (`tfv.py`, `pbt_conformance.py`,
  `fuzz_conformance.py`, `bootstrap_harness_conformance.py`).
- **Missing:** self-hosting (SH-001..SH-003); spec-level SAT over models
  (vacuity and consistency, VER00 phase S3); mutation testing of claims
  (VER00 §13); the VER04i Python corpus oracle for other lanes.

### 4.10 L9 system and integration

- **Validated block:** a package that depends on other packages, whose
  environment assumptions are discharged by the claims of those packages
  (VER00 §7.5.6 rule 1, applied between packages rather than between layers).
- **Discharges:** nothing in VER13: every catalogue entry is about one
  package. The report's `closure` field lists one package for both adopters.
- **Supplied by:** within a package, multi-operation claims such as round
  trips (`decompress(compress(d)) == d`) are live today; binding-level
  sequence drivers for stateful objects are specified (WX-F, VER04k) but not
  implemented.
- **Missing:** Warrant v0 has no imports ("Imports, includes and host paths do
  not exist in this profile", VER04 §2), so no Warrant file can cite another
  package's claims. Under P7 that feature waits for a named adopter; the
  natural ones are `reed-solomon` (over `gf256`) and `hmac` (over
  `ct-compare`), both real Rust crates on `main`.

### 4.11 Beside the ladder: `os`, `gov` and `tcb`

- **`os` (9 entries).** Allocator, `std` runtime, vDSO, environment reads, the
  runtime-gate baseline and its limits, kernel trust, counters, Linux-only
  evidence. The runtime capability gate (VER08b, `capability-runtime-conformance`,
  merged in #19079) and the locked box (VER22) move them to `evidenced` for the
  tested paths. No VER document plans an owned kernel, so
  `os.kernel.trusted` has no down route today. That is recorded, not hidden.
- **`gov` (2 entries).** `gov.approval.blocked` is the reason every exemption
  is unapproved: approval is blocked by VER00 §12.1. It is not a technical
  assumption and no layer discharges it; the owner resolving §12.1 does.
- **`tcb` (6 entries).** Counted at L6. They are the verification tools'
  own correctness. Self-hosting (P6) is how they move, and it can never move
  them alone: see section 7, thread T5.

### 4.12 The ladder in one table

| Layer | VER13 entries consumed here | Live validated block today | First missing block | Thread that moves it |
|---|---|---|---|---|
| L0 physical | `phys` 2 (floor) | none (by design) | none | none |
| L1 gate primitives | `gate.primitives.correct`, `gate.sequential.ideal-dff` | `logic-gates`, `transistors` exist; no comparison | R4 truth-table comparison | T6 (AS) |
| L2 netlist | 3 `gate.*` | none | traced HNL step netlist | T6 (AS-004) |
| L3 CPU and ISA | 3 `mc`/`gate` + 5 `uarch` | `riscv-simulator`, `riscv-gatelevel` (differential only) | bit-level RV32I step function and miter | T6 (AS-004) |
| L4 compiler | 6 `cc` + 2 `mc.backend` | none (host `rustc` trusted) | R1 taint checker; `wir-to-riscv` | T6 (AS-003a, R2) |
| L5 runtime or VM | none today (proposed `cc.<lane>.*`) | none | catalogue entries; conformance tests | T8 (LV-001, LV-003) |
| L6 WIR | 12 `wir` + 6 `tcb` | `warrant-ir` lowering for the Rust subset; audited solver stack | non-trivial miters, obligation binding | T6, T5 |
| L7 source | 18 `src` + 9 `att` | VER25a/b Rust subset (ct-compare only) | versioned Rust grammar; gf256 subset | T3, T4 |
| L8 Warrant | 5 `spec` + `gov.catalogue.integrity` | `warrant-grammar`; two v0 adopters | self-hosting; claim mutation | T5, T2 |
| L9 integration | none | intra-package round trips only | cross-package claims | T2 (named adopter first) |

### 4.13 Layered, independent verification (owner, 2026-11-12)

The ladder is not proved in one piece. Each layer is verified on its own,
against a contract it states. Once a layer's contract is proved, for example
"gates behave as their truth tables", that is enough for that layer. Every
layer above takes that contract as an assumption and proves only how it
composes the parts below it. It never re-proves the lower layers. Verification
proceeds up the chain, the way a building is checked floor by floor: the
inspector of the third floor trusts the signed-off second floor and checks only
how the third floor sits on it. This is VER00 §7.5.6 rule 1 (the assumption and
the claim are the same formula) made into a working rule for every layer.

Three rules make composition sound. Each rules out a particular way a stack of
individually correct proofs can still be wrong:

| Rule | Statement | What it rules out |
|---|---|---|
| (a) Coverage | A layer's contract must cover everything the layer above relies on. A functional truth table says nothing about time, so a constant-time claim needs a timing contract at every layer below it, for example "one cycle regardless of data". | A constant-time proof resting on gate contracts that never mention time |
| (b) No reaching around | An upper layer may use a lower layer only through its proved contract. A helper that lives outside the contract needs its own claim. Example: `intel4004-gatelevel` has a `bits.rs` integer-to-bit-vector helper that is not part of the gate layer, so nothing the gate contract proves covers it. The gate-integrity audits in [CPU-SIMULATOR-ROADMAP](CPU-SIMULATOR-ROADMAP.md) are what enforce this rule for the CPU crates. | An upper layer that quietly calls host integers, or any code the lower contract does not describe |
| (c) Exact linking | The assumption an upper layer names must be exactly the claim the lower layer proved, identified by id, never a paraphrase. | A near-match: an upper assumption that is slightly weaker or stronger than the lower claim, so the proof chain has a gap nobody can see |

Why it matters: when every component is proved independently, the top-level
proof only has to show correct composition over the component contracts. That
can make a full-netlist SAT miter unnecessary. Whether it does for the RV32I
path of VER13 §4.4 is an open question (section 11, Q8); this section does not
change the R3 plan.

### 4.14 Layer contract linking (owner, 2026-11-12; backlog LV-005)

Rule (c) needs a mechanism, and none exists. Today `assume platform:` in a
Warrant file names dotted identifiers that resolve to the shared VER13
catalogue, and a module cannot import another (VER04 §2 and §3: "packages
cannot invent private platform axioms"). So a package has no way to say "I
assume exactly what that package proved".

The mechanism, to be specified in its own document before any code:

1. **Publish.** A package's proved Warrant claims are published as
   catalogue-style ids, for example `contract.logic-gates.and`.
2. **Name.** An upper layer's `assume platform:` names those ids.
3. **Discharge only on proof.** The assurance report counts such an assumption
   as discharged only if the lower package's evidence actually proves that
   claim, at the pinned revision. Otherwise the assumption stays open and is
   printed as open.

This is the first Warrant feature with a concrete adopter, the Intel 4004 chain
of section 4.15, so it complies with P7 and with the feature freeze (section 11,
Q7: no new feature before an adopter names it). It needs its own specification
before code. Two consequences for that specification are recorded here and
decided there: the `contract.` prefix is not in VER13's closed family set, and
VER04's rule that a platform id must resolve to the VER13 catalogue has to be
widened to cover ids published by a package (conflict C14). This roadmap edits
neither document and adds no entry to `code/assumptions/`.

### 4.15 The Intel 4004 chain: a second stack and a bottom-up pilot (owner, 2026-11-12; backlog LV-006)

The Intel 4004 chain complements C1: RV32I on `riscv-gatelevel` comes first
for the `ct-compare` constant-time proof, and the 4004 chain is a second,
independent stack. It is the bottom-up adoption pilot for sections 4.13 and
4.14: small enough that every layer can be proved exhaustively, and made of
real packages.

| Layer | Package(s) | Proves | Assumes | Size |
|---|---|---|---|---|
| Gates | `logic-gates` (on `transistors`) | each gate's truth table | the host language does what it says | 4 cases per gate, exhaustive |
| Combinational | mux, decoder (`logic-gates` combinational) | block function | gate contracts | small, exhaustive |
| Arithmetic | `arithmetic` (adders, ALU) | 4-bit result and carry = `(a+b+c) mod 16` | gate and combinational contracts | 512 cases, exhaustive |
| Sequential | flip-flops, registers | state transitions across clock edges | gate contracts | small; first stateful layer |
| CPU step | `intel4004-gatelevel` | per instruction, step = the ISA model from the datasheet | ALU, register, PC and stack contracts | per instruction, over contracts |
| Programs | Nib to 4004, Busicom | program claims | the CPU contract | varies |

Notes that go with the table:

- **Timing contracts.** Rule (a) applies. If the chain is ever used for a
  constant-time claim, each layer also carries a timing contract, for example
  "one cycle regardless of data" at the CPU step. The functional rows above
  say nothing about time.
- **One ISA model, many languages.** One `.warrant` ISA model can test every
  language's 4004 behavioural simulator. The repository has them in Rust, C,
  C++, Go, Python, Elixir, Lua and Perl. This makes the pilot a direct exercise
  of P2, "one spec, many languages".
- **Nibbles.** A nibble can be a `U8` refined to values below 16 today. A `U4`
  or fixed-width bits type would be a feature request, and under P7 it is made
  only when this chain, as the adopter, asks for it.
- **Trust boundary.** The chain proves that the simulators match the datasheet
  model. It assumes the datasheet transcription, the host language, the
  toolchain and Warrant itself. It proves nothing about real silicon, so the
  L0 floor entries stay printed.

---

## 5. Per-language assumption tiers

### 5.1 The starting assumption, said precisely

For every lane the starting position is the same sentence: *the language does
what its own specification says.* Said that loosely it cannot be checked, so
this roadmap splits it into three statements, each with a different owner and
a different route:

1. **Semantics as specified** (layer L7, VER13 key `src`). On the constructs
   the package uses, the language means what its defining document says:
   integer widths and overflow, evaluation order, sequence and string
   semantics, exceptions or panics, the standard-library calls used. This is
   what a WIR front end encodes when it lowers the lane's source.
2. **Implementation conforms** (layer L5, VER13 key `cc`). The runtime or
   compiler the lane actually runs on implements statement 1 for those
   constructs. This is what turns a proof about the model into a statement
   about the running program.
3. **Toolchain pinned** (layer L4/L5, VER13 key `cc`). Evidence is for one
   toolchain release, target and flag set, recorded in the manifest. This is
   the only one of the three that test evidence needs (section 2,
   consequence 1).

### 5.2 Proposed catalogue entries (a later slice, LV-001)

This document does not edit `code/assumptions/`. It proposes the entries
below for the VER13 catalogue, following its conventions (VER13 §5.2: id
pattern `[A-Za-z0-9._-]+`, the family is the id prefix, the closed family set
`spec src wir cc mc uarch gate phys tcb os gov att`, the eight stack layer
keys plus `os` and `gov`). `<lane>` is the lane name exactly as in the lane
list (`typescript`, not `javascript`; `cpp`; `csharp`; `fsharp`), so ids sort
by lane.

| Id template | Layer / family | Kind | Status on entry | Statement template | Relations |
|---|---|---|---|---|---|
| `src.<lane>.semantics-as-specified` | `src` / `src` | `trust` | `assumed` | "On the constructs the package uses, `<lane>` behaves as `<defining document>` for the release(s) the package declares." | Umbrella; `refined_by` the per-construct children below, added only when an adopter's claim needs one |
| `src.<lane>.<construct>` (for example `src.python.int-semantics`, `src.go.int-semantics`, `src.c.no-undefined-behaviour`) | `src` / `src` | `model` | `assumed` | One construct, quantifier-explicit (VER00 §7.5.3 examples: overflow wraps, `sort` is stable, a map returns the last value stored) | `refines` the umbrella |
| `cc.<lane>.implementation-conforms` | `cc` / `cc` | `trust` | `assumed` | "The `<runtime>` release pinned in CI implements `src.<lane>.semantics-as-specified` on the constructs the package uses." | `depends_on` the umbrella |
| `cc.<lane>.version-pinned` | `cc` / `cc` | `environment` | `evidenced` once the lane manifest records the toolchain tuple | "Evidence is for the recorded toolchain tuple only." | Generalises the existing Rust-only `cc.version-pinned` |

Rules for landing them, so that the catalogue does not grow ahead of its users
(P7 applied to assumptions):

- **Rust first, and only as an umbrella.** `src.rust.semantics-as-specified`
  is `refined_by` the existing `src.rust.int-semantics`,
  `src.rust.safe-bounds-checks` and `src.rust.subset-conformance`; no new
  Rust children until a claim needs one. The existing `cc.version-pinned`
  keeps its id, because VER13 §5.2 forbids renames; `cc.rust.version-pinned`
  is not created. Adding the umbrella changes
  the AC-4 closure of every Rust proof label, so `ct-compare` and `gf256`
  reports are regenerated in the same PR.
- **Other lanes land with their first proof label, not before.** AC-4 applies
  to proof labels, and AC-10 flags an entry no report uses. A lane's
  `src.<lane>.*` and `cc.<lane>.implementation-conforms` entries therefore
  arrive in the PR that gives that lane its first WIR front end (WD-4xx), and
  `cc.<lane>.version-pinned` arrives with the lane's first evidence manifest
  (WD-1xx).
- **`discharge.backlog`** for every new entry is LV-001 until a more specific
  row exists (AC-14 requires a real backlog id).
- **The VER13 §2 text for `cc` is widened** from "the compiler and its IR" to
  "the language implementation: compiler, interpreter or VM, and its IR", in
  the same slice (conflict C2). No new layer key is added, because `cc`
  already sits where a runtime belongs: below WIR, above machine code.

### 5.3 Routes to discharge

**Down, three ways, in increasing strength:**

1. **Across (conformance).** Every `src.<lane>.<construct>` entry gets a
   conformance test in its lane that runs on every supported toolchain
   (VER00 §7.5.3). The natural form is a small Warrant file per construct,
   run through the VER04i corpus like any package, so the conformance tests
   are themselves generated from one source (LV-003). This moves an entry to
   `evidenced` for that stack and turns a silent toolchain change into a named
   failure. It never reaches `discharged`.
2. **Through a front end.** A WIR front end built on the lane's versioned
   grammar (VER14, not yet written) lowers real lane source to WIR. The proof
   then rests on `wir.lowering.faithful` for that lane, which is differentially
   tested against the real runtime (`wir.interp.differential-only`). The
   semantics assumption does not disappear; it becomes a precise, tested
   statement about one lowering instead of a statement about a whole language.
3. **Onto an owned layer.** The lane's code runs on a runtime we own (section
   4.6 candidates) whose Warrant file claims the catalogue entries. On that
   stack the entries are `discharged` and the label becomes
   `proved-to-<layer>`; on the third-party stack they stay `evidenced`
   (VER00 §7.5.6 rule 2). Because every owned runtime is a Rust crate, its own
   correctness then rests on the Rust lane's ladder (section 2, consequence 2):
   the lanes converge.

**Up: a validated language specification.** Instead of trusting the defining
document's prose, the lane gets an executable semantics for its subset,
validated against the language's own conformance suite where one exists
(for example test262 for ECMAScript, ruby/spec for Ruby, CPython's
`Lib/test`, the Rust compiler's `ui` tests through the RS01 oracle). The
front end of route 2 is then checked against that semantics as well as
against the runtime, so two independent oracles must agree. This route is not
specified anywhere yet; it is recorded as the long-term target for lanes with
no owned runtime (go, cpp, kotlin, swift, dart, ocaml, lua, elixir,
haskell).

### 5.4 Per-lane table

"Grammars" counts the `.grammar` files on `main` in `code/grammars/<dir>/`,
including the one unversioned `<lang>.grammar` that python, typescript, ruby
and perl keep beside their release pairs (c has only the unversioned file);
they are the start of the lane's front end (LANG84). "Owned below" lists
repository crates that could become the L5 validated block. The defining
document is the one the umbrella statement will cite; where a language has no
normative specification, the entry says so and cites the pinned release's
reference documentation instead.

| Lane | Defining document | Runs on in CI | Grammars on `main` | Owned below (L5 candidates) | First discharge step |
|---|---|---|---|---|---|
| rust | The Rust Reference (and the Ferrocene Language Specification) | `rustc` + LLVM | none in `code/grammars/`; VER25a hand-written subset in `warrant-ir`; versioned grammars planned (T3) | WIR front end live (VER25a/b); `wir-to-riscv` (R2) and `riscv-gatelevel` (R3) planned | Umbrella over existing `src.rust.*`; LP-001a; AS-002 |
| python | The Python Language Reference, per release | CPython | `python` 18 (2.2 to 3.16.0a1; PY01) | `python-lexer`, `python-parser`, `python-iir-compiler` (LANG79); PY02 VM | WD-101 lane manifest; then int and sequence conformance |
| typescript | ECMA-262 for run-time meaning (types are erased); no current normative TypeScript specification | Node.js | `typescript` 7 (1.0 to 5.8), `ecmascript` 14 (ES1 to ES2025), `javascript` 1 | `typescript-lexer`, `typescript-parser`, `javascript-iir-compiler` | WD-102; test262 as the up-route oracle |
| go | The Go Programming Language Specification | `gc` toolchain | none | none | WD-103; int-wrap conformance |
| ruby | ISO/IEC 30170 (an older core) and ruby/spec; the pinned CRuby release | CRuby | `ruby` 6 (0.49, 0.50, 3.3.5-3.3.7) | `ruby-lexer`, `ruby-parser`, `ruby-iir-compiler` (LANG80) | WD-104 |
| c | ISO/IEC 9899 | gcc, clang, MSVC | `c` 1 (unversioned) | `c-lexer`, `c-parser`; checked C interpreter planned (VER16) | `src.c.no-undefined-behaviour` first; WD-4xx names C as the first proof lane |
| cpp | ISO/IEC 14882 | g++, clang++, MSVC | none | none | WD-106 |
| java | The Java Language Specification and the JVM Specification | OpenJDK | `java` 10 (1.0 to 21; JV01) | `java-lexer`, `java-parser`, `jvm-simulator`, `iir-to-jvm-class-file` | WD-107 |
| kotlin | The Kotlin language specification | Kotlin compiler on the JVM | none | `jvm-simulator` (shared JVM layer) | WD-108 |
| csharp | ECMA-334 and the C# language specification | .NET | `csharp` 12 (1.0 to 12.0; CS01) | `csharp-lexer`, `csharp-parser`, `clr-simulator`, `iir-to-cil-bytecode` | WD-109 |
| fsharp | The F# language specification | .NET | `fsharp` 15 (1.0 to 10; FS01) | `fsharp-lexer`, `fsharp-parser`, `clr-simulator` | WD-110 |
| elixir | No normative specification; the pinned release's reference documentation, on BEAM | Elixir on BEAM | none | `iir-to-beam` emits BEAM; no owned BEAM VM | WD-111 |
| lua | The Lua Reference Manual, per version | PUC Lua | none | none | WD-112 |
| perl | No normative specification; `perlsyn`/`perlop` of the pinned release, and the perl test suite | perl | `perl` 119 (1.000 to 5.45.3; LANG82) | `perl-lexer`, `perl-parser`, `perl-iir-compiler` (LANG81) | WD-113 |
| haskell | The Haskell 2010 Report (GHC extensions are outside it) | GHC | `haskell` 7 (1.0 to 1.4, 98, 2010) | `haskell-lexer`, `haskell-parser` | WD-114 |
| swift | The Swift Programming Language (reference part) | `swiftc` | none | none | WD-115 |
| dart | The Dart language specification | Dart VM / AOT | none | none | WD-116 |
| ocaml | The OCaml manual (language part) | `ocamlopt` | none | none | WD-117 |

The `WD-1nn` labels are positions in VER04i §13.2's order, not existing
backlog ids; the backlog has one family row, `WD-1xx`. Nine lanes (rust, go,
cpp, kotlin, elixir, lua, swift, dart, ocaml) have no versioned grammar yet,
so their route 2 starts with a grammar inventory under LANG84.

### 5.5 Every version, every lane

P8 has two halves:

- **Front ends parse every release.** LANG84 already requires one materialised
  grammar pair per tagged release, earliest first, with no aliases. The Rust
  plan (thread T3) extends it with an *edition* axis (one pair per release and
  edition, 2015/2018/2021/2024) and treats macros as Rust's preprocessor:
  invocations, attributes and `macro_rules!` are parsed as token trees, `cfg`
  is a pure evaluator over the parse tree, `macro_rules!` and the builtin
  macros of each release are expanded by a separate layer, and procedural
  macros, custom derives and `include!` are opaque nodes that every consumer
  needing expanded code refuses (fail closed).
- **Evidence names the release it ran on.** A lane manifest records its
  toolchain tuple (`cc.<lane>.version-pinned`). Running every package's corpus
  on every release of every lane in CI is not proposed here; which releases
  a package's evidence must cover is an owner decision (section 11, Q3).
  Warrant itself follows the same rule: a v0 document keeps its meaning under
  every later version of the tool (VER04j, VER04k).

---

## 6. Evidence kinds per lane

The owner's list of evidence kinds, mapped to how each is produced. "Corpus"
means VER04i's neutral corpus: the canonical Rust compiler expands the Warrant
file once, and a thin per-lane runner executes it against the lane's real
export. "Front end" means the lane's source lowered to WIR.

| Kind | Produced by | Route | Live today | Depends on (catalogue ids) | Label it may carry |
|---|---|---|---|---|---|
| Unit (example) | Warrant `example` rows | Corpus layer A | Rust (ct-compare, gf256) | `spec.*`; `tcb.evidence.manifest-honesty`; `cc.<lane>.version-pinned`; the runner and binding boundary (VER04i §11.3, no catalogue id yet) | `pass` per case |
| Exhaustive | Domain descriptors enumerated in-lane, cross-checked by `inputs_sha256`/`expected_sha256` against the Rust compiler and the Python oracle | Corpus layer A | Rust | as unit, plus the stated domain (never "all inputs" unless the domain is the whole type, as for `U8 x U8`) | `exhaustive over <domain>` |
| Property | Layer A: a materialised biased sample; layer B: the lane's own engine, gated by `pbt-v1` | Corpus layer A, native layer B | Rust; TypeScript engines exist (VER05a, VER06b) but produce no Warrant-derived lane evidence | as unit, plus the lane engine's conformance | `property: <n> cases, seed` |
| Fuzz | Lane's coverage-guided engine, gated by `fuzz-v1` | Native layer B | Rust | as property | `fuzz: <n> iterations` |
| Integration | Multi-operation claims within a package (live); binding-level sequence drivers (WX-F, specified); cross-package claims (no construct) | Corpus layer A (sequences need a corpus encoding first) | Rust, intra-package only | as unit, plus the dependency's claims once cross-package claims exist | as the underlying kind |
| Assumption conformance | One small Warrant file per `src.<lane>.<construct>` entry, run on every supported toolchain (LV-003) | Corpus layer A | none | the construct's own statement | moves the entry to `evidenced` |
| SAT over the model | Vacuity, consistency and model-level proofs of the Warrant file itself (VER00 phase S3) | Language-independent | none (S3 not started) | `spec.*` only | `proved-over-model` (below the E5 floor) |
| Solver-derived inputs | Witnesses and boundary inputs found by the solver from claims and assumptions, written into the corpus as ordinary cases | Corpus layer A | none | as unit | `pass` per case |
| Proof over source (E5) | Front end to WIR, symbolic execution, VER20 self-composition, DRAT | Front end, layer C | Rust only (ct-compare: 1 `proved`, 3 `bounded(128)`) | `src.<lane>.*`, `cc.<lane>.implementation-conforms`, all applicable `wir.*` and `tcb.*` | `proved`, `bounded(k)` |
| Proof down the stack | Owned compile and netlist (VER13 R2, R3) | Front end plus owned layers | none | as E5 minus what the rung discharges; `gate.*`/`phys.*` remain | `proved-to-isa`, `proved-to-gate-level` |

Two rules come from the table:

1. **A corpus kind is live in a lane only when that lane's runner ran it.**
   Rust results are never inherited (VER04i §11.5); "required by spec; not
   established in this lane" is the honest row until then.
2. **Solver-derived inputs are the bridge from P2's "SAT/proof-based tests"
   to every lane.** A proof exists only where a front end exists, but the
   inputs a solver finds while checking the Warrant file (boundary cases,
   counterexamples to weakened claims) are plain data. Put into the corpus,
   they run in all eighteen lanes on day one. That is the cheapest way to give
   lanes without a front end some of the proof layer's reach.

### 6.1 The runtime jail is mandatory (owner, 2026-11-12)

Every library's test program runs inside a jailed sandbox built from exactly
its `required_capabilities.json`. Anything else that happens is a failure. This
is not an evidence kind in the table above; it applies on top of every one of
them, whatever the lane and whatever the label. [VER08b](VER08b-runtime-capability-conformance.md)
and the `capability-runtime-conformance` crate provide it: an enforcement ring
(`no_new_privs`, Landlock and seccomp), an independent `strace` observation
ring, a Docker ring, and 18 negative controls that show the gate catches each
forbidden category.

**Scope of the jail.** The union of the declared capabilities of the package
and of its dependency closure, plus whatever the test program itself declares.
Nothing beyond that union is allowed.

**Platforms.** Linux is live. macOS and Windows report NOT-LIVE, and a NOT-LIVE
result never counts as a pass.

**Rules.**

1. An adopter conversion is not complete without the jail. This makes
   CAP-RT-003 a hard requirement, not an option.
2. A report with no live runtime evidence can never rise above `report-only`
   (CAP-RT-002).
3. The per-language lanes (arming the filter after interpreter or VM startup)
   are the CAP-RT-1xx rows.

**State on 2026-11-12.** Recorded so that nobody reads the gate as stronger
than it was:

- Only `ct-compare` and `wasm-leb128` (PR #19227, open) use the gate.
- No CI log had ever shown it running LIVE.
- CI installed no `strace` and set no `RCG_STRICT`.
- A NOT-LIVE gate exited 0, and the build tool hides the output of passing
  packages, so the gate could pass silently.
- PR #19227 adds a Linux-only strict `BUILD_linux` line and installs `strace`
  on Linux CI. Until it merges and a CI log shows the gate LIVE, the jail is
  specified but not evidenced in CI.

CAP-RT-004 (the CI wiring row) is not changed by this roadmap; PR #19227 owns
that edit.

---

## 7. Threads and how they compose

### 7.1 The threads

| Thread | Backlog ids | Layers it moves | What it unblocks | Depends on | State on `main` (2026-11-12) |
|---|---|---|---|---|---|
| T1 Polyglot lanes | WD-002 (spec), WD-003, WD-004, WD-1xx, WD-2xx, WD-3xx, WD-4xx | L8 to L7 sideways: the same claims, evidence in 18 lanes | P2, P3 for tests; per-lane manifests; per-lane reports | WD-003 (open PR #19177, owned by another session; referenced only), then WD-004 | VER04i is a draft; ct-compare ports exist in all 18 lanes (WD-3xx done, #19082); no non-Rust manifest |
| T2 Leaf adoption | LP-001..LP-007, plus the enablement fixes G1-G6 of the adoption analysis | L8 (more adopters), L9 (first integration claims later) | Justifies every feature (P7); exercises the report on new archetypes | PR-0 enablement (driver `hosts` pass-through, report profiles, generic proofs target); SEC-001 merged (#19059) for lzw | gf256 converted (#19077); ct-compare converted (#19054); both v0 documents |
| T3 Versioned grammars | LANG84 (all lanes); proposed RS01 for Rust (not yet a spec) | L7 | Front ends for every release (P8); the Rust analyzer's lexer; VER14 front ends for other lanes | `grammar-tools` engine batch for nested comments and prefix parsing (needs `ci-full-build`) | PY01, CS01, JV01, FS01, LANG82 and the TypeScript, ECMAScript, Ruby and Haskell grammars exist; no Rust grammar |
| T4 Rust proof subset | LP-001a | L7, L6 | gf256 proofs go live; CWE-400 step-proof exemption can retire for gf256 | T3 milestone 1 (modern lexers, partial modern grammar, differential against VER25a) | gf256 lowering refused (`&'static` at `src/lib.rs:193`, method call `log`) |
| T5 Self-hosting | LH-001..LH-008, SH-001..SH-003 | L8 and the `tcb` family at L6 | P6; moves `tcb.*` towards `evidenced` | Leaf audits merged; SH-001 gap analysis; an independent oracle for each self-hosted tool | LH-001/003/005/006 merged; SH-* not started |
| T6 Assumption reduction | AS-001a (done), AS-001b, AS-001c, AS-002, AS-003a, AS-004, AS-005, AS-006 | L6 down to L1 | P5; retires `EXEMPT-CWE-208-GATE-LEVEL` on the owned stack | AS-002 (Miri) before R1; R2 needs the symbolic RV32I executor; R3 needs the traced netlist and VER18 | Catalogue and AC-1..AC-14 merged (#19089, #19126); nothing discharged |
| T7 Feature work | WX-000/001 (specs), WX-A..WX-J, WX-RD1 | L8 (expressiveness) | Specific adopters only | A named adopter PR (P7) | WX-B/C/A/E/H and WX-D1 (#19209) merged; neither adopter uses a v1 feature |
| T8 Per-lane assumptions | LV-001, LV-003 (proposed) | L7, L5 | P4 for every lane; the AC-4 closure of non-Rust proofs | T1 manifests (for pins); WD-4xx (for semantics entries) | not started |
| T9 Tool honesty | CA-OG1, CA-OG2, WX-RD1, LV-002 (proposed), WD-005, WD-009, WD-014 | L7 (analyzer), L8 (evaluator), L6 (certificates), CI | Every row the report prints for a new adopter | none (security and correctness first) | open |
| T10 Layer contracts and the 4004 chain | LV-005 (spec, then code), LV-006 | L1 to L3, then L9 (contract links between packages) | P9; the first Warrant feature with a named adopter (P7); a second, independent stack beside RV32I | LV-005 spec first; for the chain, the gate-integrity audits of CPU-SIMULATOR-ROADMAP and a Warrant file for `logic-gates` (SH thread) | not started; no Warrant file for `logic-gates`, `arithmetic` or `intel4004-gatelevel` |
| T11 Jail and release | CAP-RT-002, CAP-RT-003, CAP-RT-004 (PR #19227), CAP-RT-1xx, LV-007 | CI, and the gate on publishing | P10, P13; no report above `report-only` without live runtime evidence | CAP-RT-001 (merged, #19079); PR #19227 for the Linux strict line and `strace` in CI | Linux gate specified, never seen LIVE in CI as of 2026-11-12; `publish.yml` checks only tag format and `main` |

### 7.2 Dependency graph

Arrows read "must land before". Chains that share no arrow are independent
and can run in parallel; T9 gates only the *reports* of new adopters, not the
other chains' code.

```
  T9  CA-OG1, CA-OG2, WX-RD1, LV-002 ─────────► (any new adopter's report)

  T1  WD-003 (#19177) ──► WD-004 ──► WD-1xx per lane ──► WD-2xx native suites
                                         │
                                         ├──► LV-001 pins   (cc.<lane>.version-pinned)
                                         └──► LV-003 conformance (entries become evidenced)

  T2  PR-0 enablement ──► wasm-leb128 ──► sha1 ──► md5 ──► lzw ──► des (+G2)
                                                                     │
  T7                                    G3, G4, G6 adopters ◄────────┘
                                                 │
                                                 └──► WX-F + WX-D2, WX-J, WX-I (each with its adopter)

  T3/T4  RS01 milestone 1 ──► LP-001a (gf256 live) ──► full modern grammar ──► expansion ──► 1.x ──► 0.x
                                  │
                                  └──► LV-004 (VER14 spec) ──► WD-4xx proof lanes, C first
                                                                   │
                                                                   └──► LV-001 semantics entries per lane

  T6  AS-002 (Miri) ──► AS-003a (R1) ──► R2 wir-to-riscv + symbolic RV32I ──► AS-004 R3 netlist ──► R4
                                              │
                                              └──► owned-runtime route for other lanes (section 5.3)

  T5  LH audits (done) ──► SH-001 ──► SH-002 (prng, tfv, verdict, drat-check, sha256) ──► SH-003

  T10 LV-005 spec ──► LV-006 chain, bottom-up: gates ──► combinational ──► arithmetic
                                               ──► sequential ──► CPU step ──► programs

  T11 PR #19227 (strict jail on Linux CI) ──► CAP-RT-002, CAP-RT-003 ──► LV-007 publish gate
                                                  └──► CAP-RT-1xx per-language lanes
```

The same graph as a table, one row per edge that crosses threads:

| From | To | Why |
|---|---|---|
| WD-1xx (T1) | LV-001 pins, LV-003 (T8) | the pin lives in the lane manifest; conformance files run on the lane runner |
| LP-001a (T4) | LV-004, VER14 | the Rust differential migration is the template for every other front end |
| WD-4xx (T1) | LV-001 semantics entries (T8) | AC-4 and AC-10 make an entry meaningful only once a proof label uses it |
| des and later conversions (T2) | G2-G6, WX-* (T7) | P7: a feature ships with the adopter that needs it |
| R2 (T6) | owned-runtime route (T8) | every owned runtime is a Rust crate, so it inherits the Rust ladder |
| T1 corpus oracle | SH-003 (T5) | self-hosted `warrant-grammar` suites need an oracle `warrant-grammar` did not build |
| T9 | every new adopter report | a report must not print a row a known tool gap makes false |
| LV-005 (T10) | LV-006 (T10) | the chain's upper layers need the link mechanism to name the lower layers' proved claims by id |
| T5 SH rows (`logic-gates` Warrant file) | LV-006 (T10) | the gate layer of the chain is also a self-hosting adopter |
| CAP-RT-002, CAP-RT-003 (T11) | LV-007 (T11) | the publish gate requires live runtime-jail evidence |
| T2 conversions, T10 | LV-007 (T11) | the publish gate requires a package's Warrant spec and a generated report, so a package needs both first |

### 7.3 Where the threads meet

- **T1 and T8 meet at the manifest.** The lane manifest is where a lane's
  toolchain pin is recorded, so `cc.<lane>.version-pinned` cannot exist before
  WD-1xx for that lane.
- **T3, T4 and T1 meet at VER14.** Per-lane front ends need a grammar (T3), a
  subset gate and differential method (T4 is the template), and an evidence
  lane to report into (T1). VER14 is the document that joins them; it should
  be written after T4's differential method has run once on gf256.
- **T6 and T10 are the threads that move layers L1 to L4.** T6 does so for one
  package (ct-compare) and one stack (`owned-rv32i`); T10 does so for a second,
  independent stack (the Intel 4004 chain, section 4.15) and is where layer
  contract linking (section 4.14) gets its first adopter. Every other package and
  lane benefits only through the owned-runtime route (section 5.3), so T6's
  results should be shaped as reusable blocks (the symbolic RV32I executor,
  the traced netlist), not as ct-compare-specific scripts.
- **T5 needs T1.** A self-hosted Warrant suite for `warrant-grammar` is only
  independent if something not built by `warrant-grammar` checks it. The
  VER04i Python corpus oracle, the TypeScript `warrant-compiler` (VER04g) and
  the existing Python validators are the candidates.
- **T7 waits on T2.** Every feature row names the adopter PR it ships with.
- **T10 is the adopter for LV-005.** Contract linking has no other named
  adopter, so it is specified and built together with the chain, bottom layer
  first, and a `U4` or fixed-width bits type waits for the chain to ask for it.
- **T11 gates what T2 and T10 may ship.** A conversion is not complete without
  the jail (section 6.1), and nothing is published without the gate of section
  8.1.

### 7.4 Proof refusals and findings (owner, 2026-11-12)

**A refusal is a design finding first (P11).** Proving forces small,
decomposable, isolated components that can be wired together. When the proof
front end refuses code, the first question is whether the code should be shaped
differently, and only the second whether the tool is missing a feature. Two
examples from the `wasm-leb128` conversion: a `Formatter<'_>` lifetime in the
signature, and an error type holding a `String` that hides which error
occurred. The fix pattern is the same each time: a small pure core with an
explicit contract, and a thin I/O or presentation shell around it. The `gf256`
const-table decision is an instance of this pattern. A front-end feature is
added only when the code is right as it is, and then under P7 with a named
adopter.

**The finding protocol (P12).** The owner expects hundreds of bugs, some
serious, to surface through this approach. Every finding gets five things, in
this order:

1. a backlog row;
2. a test that fails before the fix;
3. the fix;
4. a Warrant rejection or claim that pins the behaviour;
5. a lesson, written with `python3 code/scripts/lessons.py new`.

Worked example: SEC-006. Bounded LEB128 decoders accepted `max_bits > 64`. The
`wasm-leb128` conversion found it, and PR #19227 (open) fixes it.

---

## 8. Near-term ordering

The ordering rules, from the owner's principles and the backlog's own
delivery rules: security and correctness first; lanes and adoption before
features; a feature only together with a named adopter; one bounded change
per PR.

| Step | Work | Named adopter or reason | Principle | Depends on |
|---|---|---|---|---|
| 1 | Close the open security rows: CA-OG1, CA-OG2 (analyzer false negatives) and WX-RD1 (evaluator memory) | Correctness of rows every adopter's report prints | P4 (no hidden gaps) | none |
| 2 | WD-003 lands (PR #19177, other session), then WD-004 generalises the report beyond `rust/` | ct-compare, gf256 in every lane | P2, P3 | none (independent of step 1) |
| 3 | PR-0 enablement inside conversion 1: `gen_evidence` passes host lines through; report profiles stop printing false CWE-327/409 "excluded" rows and gain a "no operand labelled secret" CWE-208 branch; one generic proofs target; LV-002 CI rule so editing a `.warrant`, a pinned fixture, `warrant-grammar`, `warrant-ir` or `ca-assurance-report` re-selects every adopter | wasm-leb128 | P1, P7 | none |
| 4 | Python lane (first WD-1xx): runner, sabotage, manifest, report for ct-compare then gf256; `cc.python.version-pinned` lands with it | ct-compare, gf256 | P3 | step 2 |
| 5 | Conversions sha1, md5, lzw, des (des carries G2, `Bytes<N>` parameters) | the four packages themselves | P7 | step 3 |
| 6 | RS01 spec and milestone 1, then LP-001a: gf256 proofs live; and convert gf256 to a v1 document so WX-B/C get their first real adopter and the v0 `U8`-as-`Nat` path (WD-013) can retire | gf256 | P8, P7 | none for the spec; T3 for LP-001a |
| 7 | Remaining WD-1xx lanes in VER04i §13.2 order (typescript, go, ruby, c, cpp, java, kotlin, csharp, fsharp, elixir, lua, perl, haskell, swift, dart, ocaml) | ct-compare, gf256 | P3 | step 4's runner template |
| 8 | AS-002 (Miri on the `unsafe` region), AS-003a (R1 taint checks) | ct-compare | P5 | AS-001a (done) |
| 9 | SH-001 gap analysis, then SH-002 with an independent oracle per tool | test-bootstrap, drat-check, sha256 | P6 | LH audits (done) |
| 10 | LV-001 Rust umbrella; LV-003 first conformance files for Python and Rust integer semantics | the lanes of steps 4 and 6 | P4 | step 4 |
| 11 | Generator gaps in adopter-yield order: G3 text host (four classical ciphers, base64), G4 integer-result fault (hash-functions), G6 qualified exports (lz77, lz78, lzss after their error channel) | the named packages | P7 | step 5 |
| 12 | Language features only with their adopters: WX-F with WX-D2 (streaming `Digest` for sha1/md5, rng, range-coder, the data structures); WX-J floats (numeric libraries); WX-I recursive types (resp-protocol); cross-package claims (reed-solomon, hmac) | each named | P7 | step 11 |
| 13 | VER14 spec, then the first non-Rust proof lane (C, as WD-4xx says) with its LV-001 semantics entries | ct-compare (C port) | P3, P4 | steps 6 and 7 |
| 14 | R2 `wir-to-riscv` and the symbolic RV32I executor; then AS-004 R3 on `riscv-gatelevel`; then R4 | ct-compare on `owned-rv32i` | P5 | step 8; VER18 for R3 |
| 15 | LV-005 specification (layer contract linking), then the 4004 chain bottom-up (LV-006): gates, combinational, arithmetic, sequential, CPU step, programs | the 4004 chain itself (this is the adopter that justifies LV-005) | P9, P7, P2 | none for the spec; the gate-integrity audits and a `logic-gates` Warrant file for the chain |
| 16 | LV-007 publish gate in `publish.yml`, built before the first release | every package that will ever be published | P13, P10 | CAP-RT-002 and CAP-RT-003 live (jail evidence), and PR #19227 merged |

Steps 2, 3, 6, 8, 9 and 15 are independent and can run in parallel; the table
order is priority, not sequence. WX-D1 is already merged; its remaining
slices (D2, D3) and WX-G wait for steps 11 and 12 to name an adopter.

### 8.1 Release standard (owner, 2026-11-12; backlog LV-007)

Nothing from the repository has been published yet. The owner holds all
publishing until packages are proven, and any package that is published must
meet a very high standard (P13). Today `.github/workflows/publish.yml` checks
only the tag format and that the commit is on `main`. That is not a standard.

Before the first release, a publish gate is built into `publish.yml`. For the
package being published it must require all three of:

1. the package's Warrant spec;
2. a generated assurance report that meets the bar (generated from evidence,
   never typed; P1);
3. live runtime-jail evidence (section 6.1), not a NOT-LIVE result.

What counts as "meets the bar" is part of LV-007's own specification. The
VER00 §12.1 approval block on exemptions still stands, so a package with an
unapproved exemption cannot meet it until the owner resolves that.

---

## 9. Honest current state

As of `main` at the time of writing. Each row says where it can be checked.

| Fact | Source |
|---|---|
| Two packages have Warrant-derived evidence: `rust/ct-compare` and `rust/gf256`. Both reports have `verdict: report-only`. | `code/packages/rust/{ct-compare,gf256}/assurance_report.json` |
| `ct-compare`: 4 claims; 1 `proved` (`ct_eq_u64`) and 3 `bounded(128)`; E5 `pass`, E2 `not-live`; every DRAT certificate is the collapsed trivial-miter refutation; the report lists 33 assumption ids. | ct-compare report; `wir.proof.trivial-miter` |
| `gf256`: 6 claims with no proof label (`proof: null`) and E5 `not-live`, because the Rust subset front end cannot parse its source (`&'static` at `src/lib.rs:193`) or lower `log` method calls. It is still a v0 document, so the v0 evaluator's `U8`-as-`Nat` behaviour (WD-013) still applies to it. | gf256 report (`EXEMPT-CWE-400-NO-STEP-PROOF` rationale); LP-001a; WD-013 |
| Every exemption is unapproved: ct-compare's five (CWE-416, 415, 476, 190 unsafe exception; CWE-208 gate level) and gf256's two (CWE-400 no step proof; CWE-208 gate level). Approval is blocked by VER00 §12.1. | both reports; `gov.approval.blocked` |
| Only the Rust lane has evidence. `ct-compare` exists in all 18 lanes, but no lane other than Rust has a `warrant-lane-evidence-v1` manifest; VER04i is a draft and WD-003 is an open PR (#19177). | `git grep warrant-lane-evidence-v1` finds no manifest; VER04i status line |
| CI does not re-select adopters when their `.warrant`, a pinned fixture, the generator (`warrant-grammar`), the proof crate (`warrant-ir`) or the report (`ca-assurance-report`) changes; only `v0_compat` pins the two adopters' generated output byte for byte. | leaf-adoption analysis §4 item 5; LV-002 |
| Known tool gaps: CA-OG1 (a type alias's default parameter is never bound), CA-OG2 (the local-receiver waiver is bypassed through `Deref`), WX-RD1 (the ground evaluator deep-clones values without charging them, so its memory meter bounds a value, not an evaluation). | VER-BACKLOG rows CA-OG1, CA-OG2, WX-RD1 |
| On `main`, no Warrant v1 feature has a real adopter: both converted documents are v0, and v1 is exercised only by test fixtures (`wx_fixed`, `wxa`, `wxh`, `wxv`). `wasm-leb128` is the first v1 adopter, in PR #19227, which is still open at the time of writing; it is not on `main` and counts only once it merges. | `code/specs/*.warrant` headers; `warrant-grammar/tests/fixtures/`; PR #19227 |
| PR #19227 (open) also carries the first finding of the P12 protocol, SEC-006 (bounded LEB128 decoders accepted `max_bits > 64`, found by the `wasm-leb128` conversion), and a Linux-only strict `BUILD_linux` line with `strace` installed on Linux CI for the runtime jail. | PR #19227; VER-BACKLOG row SEC-006 |
| The runtime jail (VER08b) is used by `ct-compare` and, in PR #19227, `wasm-leb128` only. No CI log had ever shown it running LIVE; CI installed no `strace` and set no `RCG_STRICT`; a NOT-LIVE gate exited 0 and the build tool hides the output of passing packages, so the gate could pass silently. Linux is live; macOS and Windows report NOT-LIVE. | VER08b; section 6.1; CAP-RT-004 |
| Nothing from the repository has been published. `publish.yml` checks only the tag format and that the commit is on `main`; it does not require a Warrant spec, a generated assurance report or runtime-jail evidence. The owner holds all publishing until packages are proven. | `.github/workflows/publish.yml`; section 8.1; LV-007 |
| No layer contract exists. There is no Warrant file for `logic-gates`, `arithmetic` or `intel4004-gatelevel`; no package publishes `contract.*` ids; and `assume platform:` resolves only to the VER13 catalogue (VER04 §2 and §3, no imports). The 4004 simulators exist in Rust, C, C++, Go, Python, Elixir, Lua and Perl, with no shared `.warrant` ISA model. | VER04 §2 and §3; sections 4.14 and 4.15; LV-005, LV-006 |
| The `gen_evidence` driver discards host binding lines, so WX-A composites and `errors=` mappings have never run through a BUILD-driven path. | leaf-adoption analysis §0 |
| Below WIR nothing is validated: no traced netlist, no `wir-to-riscv`, no R1 taint checker, no VER18 document. Of the 84 catalogue entries none is `discharged`: 65 are `assumed`, 9 `attested`, 3 `evidenced` and 7 `floor`. | `code/assumptions/catalogue-v1.json` statuses |

---

## 10. Conflicts and tensions with existing specifications and backlog

Each item names both sides and the resolution this roadmap recommends. None is
changed by this document except where section 12 says so.

| Id | Conflict | Recommended resolution |
|---|---|---|
| C1 | **Which CPU for gate-level constant time.** The owner's example and VER13 §4.4 choose RV32I on `riscv-gatelevel`. Backlog row AS-004 says "compile ct-compare's kernel to the repo's gate/CPU simulators (Busicom/Intel 4004 track, VER18)", and VER26 §8.1 calls the Busicom-to-gates pilot "the only route to meeting the CWE-208 floor". The id AS-004 is also used for two rows (AS-006 records that). | RV32I is the constant-time ladder for ct-compare; the Busicom/Nib pilot proves a different program and stays a separate owned-language pilot. Fix AS-004's text and split the duplicate id under AS-006. **Resolved 2026-11-12:** RV32I first (see 11.1). **Follow-up the owner asked for (2026-11-12), done:** VER26 §8.1 and the matching sentence in VER26 §9 now say RV32I on `riscv-gatelevel` is the first route and the Busicom/4004 track a second, independent stack (section 4.15); backlog row AS-004 says the same; the duplicate id is split, with the spec-inconsistency row renumbered AS-007 under AS-006. |
| C2 | **No runtime/VM layer in VER13.** The vision's "one layer down (runtime/VM)" has no layer key; `cc` is described as "the compiler and its IR". | Widen `cc` to "the language implementation" (section 5.2); decide whether a label such as `proved-to-vm` is needed (section 11, Q2). |
| C3 | **No system/integration layer.** Every catalogue entry is per package; Warrant v0 has no imports. | Leave the catalogue as is until a named cross-package adopter (reed-solomon or hmac) needs it (P7). |
| C4 | **Id style for per-language assumptions.** VER00 §7.5.2 uses examples such as `i64.wraps` and `sort.stable`; VER13 uses `src.rust.int-semantics`. | Follow VER13 (`src.<lane>.<construct>`); VER00's are illustrative. |
| C5 | **Features ahead of adopters.** WX-B/C/A/E/H and WX-D1 landed with no real adopter; the backlog orders extensions "cheapest unlock-per-cost first (from survey)"; VER04k orders D1 first, while the adoption analysis finds D1 alone unblocks none of the ranked candidates. | Under P7, freeze new WX slices until an adopter PR names them; convert gf256 to v1 so the landed features gain a real user; WX-RD1 (a security fix to landed code) proceeds regardless. |
| C6 | **LANG84 versus the Rust proof front end.** LANG84 forbids hand-written parsers and typed lifts on the IIR path; VER25a/b are a hand-written parser and an owned typed AST. LANG84 also orders "earliest release first", while the Rust plan orders value first (modern subset, then back-fill). | The Rust plan's differential migration keeps VER25a as the parity oracle until the grammar path replaces it; whether LANG84 binds the WIR path, and the order, are owner decisions (section 11, Q4 and Q5). |
| C7 | **One compiler or two.** VER04i makes the Rust `warrant-grammar` the owner of every lane's generators; VER04g ships a second, TypeScript Warrant compiler; WD-901 records duplicated TypeScript engines. | Keep one canonical compiler (P1); keep VER04g as an *independent oracle* for self-hosting (P6), not as a second generator. |
| C8 | **Stale facts in VER04i.** Its header says VER08b is not on `main` (it merged in #19079), and §13.2 marks lua, perl, haskell, swift, dart and ocaml as "needs port" (ports merged in #19082). The backlog's polyglot preamble has the same stale port list. | Update VER04i when WD-003 or WD-004 next edits it (not here: WD-003 is another session's open PR). |
| C9 | **Equal rigour versus "not-live" proofs.** VER04i §10 accepts `unchecked` constant-time and proof rows in 17 lanes, and VER14 (the plan to change that) does not exist. | P3 makes VER14 a required document; section 8 step 13 schedules it after the first non-Rust lanes have tests. |
| C10 | **Every version versus CI cost.** LANG84 covers every release in front ends, but evidence runs on one toolchain per lane. | Record the toolchain pin per manifest now; the owner decides which releases evidence must cover (section 11, Q3). |
| C11 | **Ownership of other lanes.** VER26 §8.3 says other lanes are owned by the package-parity loop; VER04i assigns their evidence to WD-1xx. | Already coordinated by OPS-004; ports belong to parity, evidence to WD-1xx. |
| C12 | **Platform declared `none`.** `ct-compare.warrant` says `assume platform: none` while its proofs rely on platform assumptions (`spec.platform.declared-none`, the former second AS-004 row, now AS-007). | Resolve with LV-001: the Rust umbrella is exactly the platform assumption the file should cite. |
| C13 | **Backlog status drift.** Rows in the 2026-11-10 wave table still read "PR" for items now merged (for example LP-001 #19077, CAP-RT-001 #19079, LH-006 #19081). | Hygiene for the coordinator's next backlog pass; not changed here. |
| C14 | **Contract ids versus the closed catalogue.** VER04 §3 requires `assume platform:` ids to resolve to the shared VER13 catalogue, and VER13's id families are a closed set (`spec src wir cc mc uarch gate phys tcb os gov att`). Layer contract linking (section 4.14) publishes `contract.<package>.<claim>` ids from packages, which are in neither. | LV-005's specification decides: either a `contract` family is added to VER13 with a rule that an id is valid only if the named package's evidence proves it at the pinned revision, or a package-published registry sits beside the catalogue. This roadmap edits neither VER04 nor VER13 and adds nothing to `code/assumptions/`. |

---

## 11. Open questions for the owner

1. **Q1 Stack policy.** May a package retire `EXEMPT-CWE-208-GATE-LEVEL` on
   the `owned-rv32i` stack while its production build is the host build?
   (VER13 §9 Q1, repeated because the whole of T6 depends on it.)
2. **Q2 Runtime label.** When a lane's assumptions are discharged by an owned
   VM, is the label `proved-to-vm`, or does it stay `proved` with the stack
   named (`owned-python-vm`)?
3. **Q3 Release coverage of evidence.** For "every version is supported":
   which toolchain releases must a lane's evidence run on: the newest only,
   the oldest and newest the package declares, or all of them on a nightly
   schedule?
4. **Q4 LANG84 on the WIR path.** Do LANG84's bans on hand-written parsers and
   typed lifts apply to proof front ends (the Rust plan's question 5)?
5. **Q5 Release order for Rust.** Value first (modern proof subset, then 1.x
   newest to oldest, then 0.x oldest to newest), as the Rust plan proposes, or
   LANG84's earliest first?
6. **Q6 The up route.** Is an executable language semantics, validated
   against the language's own conformance suite, an accepted discharge route
   for lanes with no owned runtime, and in which form (Warrant models, or a
   WIR-level reference semantics)?
7. **Q7 Freeze.** Do you confirm the freeze on new WX slices until an adopter
   PR names them (C5)?
8. **Q8 Composition and the netlist miter.** With each layer proved
   independently and linked by contract (sections 4.13 and 4.14), is the
   top-level proof for the RV32I path only a proof of composition over the
   component contracts, so that the full-netlist SAT miter of VER13 §4.4 is
   no longer needed? The owner has said it can be; the decision waits until
   the 4004 chain has shown the method on a small stack.

### 11.1 Owner decisions (2026-11-12)

| Question | Decision | By |
|---|---|---|
| Q1 Stack policy | The CWE-208 gate-level exemption retires only for the stack actually proved (e.g. `owned-rv32i`). A host-build row stays open, and the report names the stack. | owner |
| Q2 Runtime label | Open. Default until decided: keep `proved` and name the stack (`owned-<lane>-vm`). | coordinator default |
| Q3 Release coverage | Each PR runs a lane's evidence on the newest release plus the oldest release the package declares. A nightly job sweeps every supported release. | owner |
| Q4 LANG84 on the WIR path | LANG84 C1/C2 apply to proof front ends. Lowering walks CST views (RS01). | coordinator default, owner override welcome |
| Q5 Release order for Rust | Value first: modern proof subset, then the full modern grammar, then 1.x newest to oldest, then 0.x oldest to newest. Recorded as a deviation from LANG84. | owner |
| Q6 The up route | Not yet. Lanes with no owned runtime keep the "semantics as specified" assumption until an owned runtime discharges it (the down route). | owner |
| Q7 Freeze | Confirmed. No new WX slice is implemented until an adopter PR names it. | owner |
| Q8 Composition and the netlist miter | Open. Recorded in section 11 for the owner. | coordinator default |
| Layered, independent verification (P9, section 4.13) | Each layer is verified independently. Once a layer's contract is proved, that is enough for it; every layer above takes the contract as an assumption and proves only how it composes the parts below, never re-proving the lower layers. Composition is sound under three rules: the contract covers everything the layer above relies on (a constant-time claim needs a timing contract at every layer), an upper layer uses a lower one only through its proved contract (helpers outside it need their own claims), and the assumption is exactly the claim the lower layer proved, linked by id. (owner, 2026-11-12) | owner |
| Layer contract linking (section 4.14, LV-005) | A package's proved claims are published as catalogue-style ids such as `contract.logic-gates.and`; an upper layer's `assume platform:` names them; the report counts the assumption as discharged only if the lower package's evidence proves that claim at the pinned revision, else it stays open. It is the first Warrant feature with a concrete adopter (the 4004 chain), so it complies with Q7, and it needs its own specification before code. (owner, 2026-11-12) | owner |
| The Intel 4004 chain (section 4.15, LV-006) | A second independent stack and a bottom-up adoption pilot, complementing C1 (RV32I first for `ct-compare`). One `.warrant` ISA model tests every language's 4004 simulator. Nibbles are `U8` refined below 16 today; a `U4` or fixed-width bits type is an adopter-driven request. It proves the simulators match the datasheet model, and nothing about real silicon. (owner, 2026-11-12) | owner |
| Runtime jail is mandatory (P10, section 6.1) | Every library's test program runs inside a jailed sandbox built from exactly its `required_capabilities.json`; anything else is a failure. Scope is the package's and its dependency closure's declared capabilities plus the test program's own. Linux is live; macOS and Windows are NOT-LIVE and never a pass. A conversion is not complete without it (CAP-RT-003); without live runtime evidence a report cannot rise above `report-only` (CAP-RT-002); per-language lanes are CAP-RT-1xx. (owner, 2026-11-12) | owner |
| Release standard (P13, section 8.1, LV-007) | Nothing is published until packages are proven; the owner holds all publishing. A publish gate in `publish.yml` must require the package's Warrant spec, a generated assurance report that meets the bar, and live runtime-jail evidence, before the first release. (owner, 2026-11-12) | owner |
| Proof refusals are design findings (P11, section 7.4) | When the proof front end refuses code, treat it as a design finding first and a tool gap second. Fix pattern: a small pure core with an explicit contract plus a thin I/O or presentation shell. (owner, 2026-11-12) | owner |
| Finding protocol (P12, section 7.4) | Every finding gets, in order: a backlog row, a test that fails before the fix, the fix, a Warrant rejection or claim that pins the behaviour, and a lesson. The owner expects hundreds of findings. (owner, 2026-11-12) | owner |

Conflict C1 is resolved: the first gate-level constant-time proof targets RV32I on `riscv-gatelevel` (owner, 2026-11-12). The Busicom/4004 track remains a second, independent stack (section 4.15). The follow-up the owner asked for is done in the same change as the decisions above: VER26 §8.1's "only route" wording and backlog row AS-004 are reworded, and the duplicate AS-004 id is split (row AS-007; see C1).

---

## 12. Relationship to other specifications

| Spec or document | Relationship |
|---|---|
| VER00 | Parent. This roadmap restates §7.5.6 as a layer ladder and adds the per-lane assumption tiers; VER00 §11 gains a VER27 row. |
| VER-BACKLOG | Gains a pointer section with LV-001..LV-007. Existing rows are edited only as the 2026-11-12 decisions require: AS-004 (C1 follow-up), AS-006 and the former second AS-004 row, now AS-007 (duplicate id split), and the "mandatory" notes on CAP-RT-002 and CAP-RT-003 (section 6.1). |
| VER04i | The lane mechanism (T1). Unchanged here; its stale facts are listed in C8. |
| VER13 | The catalogue. Unchanged here; section 5.2 proposes entries and a `cc` text change as LV-001, and section 4.14 raises the question of `contract.*` ids (C14). |
| VER04 | The language. Unchanged here; its rule that `assume platform:` ids resolve to the VER13 catalogue, and that modules cannot import, is what LV-005 must widen (section 4.14, C14). |
| VER14 | Not yet written; required by P3 (section 8 step 13). |
| VER18 | Not yet written; required by R3 (L2, L3). |
| VER20, VER25, VER25a, VER25b, VER19, VER10, VER11 | The L6 machinery and the Rust L7 front end. |
| VER26 | The pilot; C1 records its Busicom statement. §8.1 and the matching sentence in §9 are reworded by the C1 follow-up (RV32I first, the Busicom/4004 track a second stack). |
| VER04j, VER04k | Feature specifications; P7 governs when their slices are implemented. |
| LANG84, PY01, CS01, JV01, FS01, LANG82 | The versioned-grammar contract and the grammars that start each lane's front end. |
| CPU-SIMULATOR-ROADMAP, RUST-CPU-SIMULATOR-BACKLOG, RUST-CPU-EXPANSION-BACKLOG | Supply L2 and L3 blocks; their gate-integrity audits are a precondition for any netlist claim and enforce the no-reaching-around rule of section 4.13. |
| VER08a, VER08b | Capability analysis at L7 and the runtime gate beside the ladder. VER08b's gate is mandatory for every library (P10, section 6.1). |

---

## 13. What this roadmap does not promise

- **Not a schedule.** Section 8 is an order, not dates.
- **Not a proof of anything.** It changes no label and discharges no
  assumption.
- **Not the floor.** L0 stays assumed for ever, and `gate.sequential.ideal-dff`
  and `cc.caller-context` are declared parts of the claim. A finished ladder
  ends with a short printed list, not an empty one.
- **Not a claim about real silicon.** The 4004 chain proves simulators match
  a transcription of the datasheet. It assumes the transcription, the host
  language, the toolchain and Warrant itself (section 4.15).
- **Not a publication plan.** Nothing is published until the gate of section
  8.1 exists and the owner lifts the hold.
- **Not inheritance between lanes.** A Rust proof never covers a Python port;
  only an owned layer that both lanes run on can carry a result from one lane
  to another (section 5.3, route 3).
