# VER31 — Tiered Release Standard: Publish What the Evidence Computes

**Status:** Approved by the owner, 2026-11-23, as written, including the new
`checked` rung (section 4). This document contains no code. The gate it specifies
is backlog row LV-007a, and the first release candidate (`ct-compare`) is LV-007b.
Whether a separate agent account is a precondition of the first release (section
9.2) is still the owner's open risk decision, asked again at the LV-007b hand-off.

**Amendments of 2026-11-25 (section 13): proposed, pending owner signature.** Section 13
drafts the VER31 text that VER32 (PIF-020) and VER33 (ADL-006b) asked for: the sign-off
file and goal-signing aggregate, the `pif:sha256:` certificate kind, the model-theorem
category with its link status, the cap on lane packages, the gate's computation of the
A-rung, and a citation of the `solver-attested` text that VER34 CVM-170b owns. **Nothing
in section 13 is in force until the owner signs it.** Sections 1 to 12 are unchanged by
it, no bucket moves, and the claim ladder of section 4 is used as is.

**Parent:** [VER00](VER00-verified-shipping-program.md) (the verified shipping
programme), [VER27](VER27-layered-validation-roadmap.md) section 8.1 and principle
P13, which this document **replaces**.

**Builds on:** [VER01](VER01-assurance-report.md) (report and verdict),
[VER13](VER13-assumption-catalogue.md) (assumption catalogue, layers, statuses),
[VER04h](VER04h-warrant-derived-evidence.md) (evidence manifests),
[VER04i](VER04i-polyglot-warrant-evidence.md) (lanes), [VER08b](VER08b-runtime-capability-conformance.md)
(runtime jail), [VER29](VER29-layer-contract-linking.md) (discharging lower layers).

**Backlog:** LV-007 (this specification), LV-007a (the gate), LV-007b (ct-compare
release candidate), LV-007c (Python and Ruby under the gate), LV-007d (report
fields the ladder needs) in [VER-BACKLOG](VER-BACKLOG.md).

**Decisions (owner, 2026-11-23):**

1. Publishing is reframed as honest tiers computed from evidence, replacing "hold
   everything until proven".
2. Lower-level proofs (compiler, machine code, CPU, gates, physics, operating system,
   source-language semantics) are assumptions a proof may simply use, as Lean trusts
   its kernel, its compiler and the hardware. The repository's own tool gaps must
   still be closed for a claim to count as `proved`.
3. Of the kernel-trust candidates argued in section 3.4, only the compiler that
   builds the verifiers (`tcb.rust-toolchain.trusted`) is trusted. "Everything else
   has to be verified."
4. Registry names use the `coding-adventures` namespace everywhere (npm
   `@coding-adventures/<name>`, crates.io the crate's existing name, PyPI
   `coding-adventures-<name>`), not a short `ca-` prefix: npm's `@ca` scope is taken,
   and in this repository `ca-` already means capability and assurance tooling.
5. A release is authorized by a hardware-key-signed tag plus the owner's approval
   of a protected `release` environment. The owner triggers every publish.
6. Packages are published under the MIT licence.

---

## 1. Why this replaces P13

P13 (VER27, 2026-11-12) held every release until a package had a Warrant spec, a
generated report that "meets the bar", live runtime-jail evidence and approved
exemptions. Each condition is reasonable alone. Together they were unreachable:
"meets the bar" was never defined, live jail evidence waits on CAP-RT-002 and
CAP-RT-003, and exemption approval is blocked by VER00 section 12.1. The result was
that nothing has ever been published, so nobody can see that the evidence exists.

The alternative is not a lower bar but an honest one. A release states exactly
what its evidence supports, computed by a program from the generated report,
with every assumption listed. A `Tested` release with fifty named assumptions is
more trustworthy than an unreleased package that is "almost proved".

### 1.1 The rule this standard exists to keep

**A tier is computed, never typed.** VER-P030 and WD-001 recorded the failure: a
hand-written report passed a gate that only checked that the report was
consistent with itself. Here, no person writes a tier, a label, a count or a
bucket into a release. The gate (section 7) reads the generated
`assurance_report.json`, re-checks it with `ca-assurance-report --check`, joins it
with the bucket file (section 3) and computes everything else. A human writes
three things only: the bucket decisions (reviewed in this spec), a statement
sign-off (section 3.2) and a security-review record (section 5, F4), and each is
bound to the exact bytes it covers.

---

## 2. Vocabulary

| Term | Meaning |
|---|---|
| Claim | One entry of a report's `claims` array: a Warrant claim with its evidence and its `proof` block. |
| Generator label | The `proof.label` that `ca-assurance-report` wrote (`bounded(128)`, `proved`, ...). Input to the ladder, never the output. |
| Rung | The level a claim reaches on the claim ladder (section 4), computed by the gate. |
| Claim tier | The weakest rung over a package's claims. |
| Release tier | The tier the package is published at: its claim tier if the higher-tier conditions hold (section 6), else `Tested`. |
| Open id | An assumption id in a claim's `proof.assumptions` that is not in its `proof.discharged`. |
| Bucket | One of (a), (b), (c) in section 3. Every catalogue id has exactly one. |

---

## 3. The three assumption buckets

Every VER13 catalogue id belongs to exactly one bucket. The assignment lives in
[`code/assumptions/buckets-v1.json`](../assumptions/buckets-v1.json) (schema
[`ver31-assumption-buckets-v1`](schemas/ver31-assumption-buckets-v1.schema.json)),
beside the catalogue rather than in it, so that the catalogue digest every report
pins does not change when a bucket decision does. Reports do not carry buckets;
the gate joins them at release time and records the bucket file's SHA-256 in the
release notes.

Coverage is closed in both directions: every catalogue id appears once, and no id
appears that the catalogue lacks. A new catalogue id therefore fails CI until
someone buckets it, and the default for an id nobody has argued is (c).

The bucket file is a trust root: moving one `wir.*` id from (c) to (a) would raise
every computed tier. So the gate does not trust it by convention. Its exact bytes
must verify against a detached signature made with the owner's hardware key
(`buckets-v1.json.sig`, checked with `ssh-keygen -Y verify -n
coding-adventures-buckets`; section 9.6), and it is code-owned by the owner. The
meaning of each bucket is fixed in the gate program ((a) never blocks `proved`;
(b) and (c) block it). A bucket file whose `blocks_proved` values say otherwise is
refused, so the data cannot redefine a bucket.

Two more fields keep the signed record self-sufficient:

- **`floor`** on each entry, true when the catalogue entry has `status: floor` or
  `kind: declared`. Rung 5 (section 4) reads floors from here, not from the
  catalogue. The gate refuses when the two disagree, so an unsigned catalogue edit
  that turns an (a) id into a floor cannot exempt it.
- **`revision`**, an integer that increases with every signed change. The gate
  refuses a revision lower than the highest that has ever appeared on `main` (it
  reads every version of the file in `main`'s first-parent history). Restoring an old file
  together with its old valid signature, which would undo a later tightening, is
  therefore refused.

### 3.1 Bucket (a): accepted platform baseline

The layers below the source program (`cc`, `mc`, `uarch`, `gate`, `phys`, `os`)
and the semantics of the source language itself (`src.rust.int-semantics`,
`src.rust.safe-bounds-checks`), plus the compiler that builds the verification
tools (`tcb.rust-toolchain.trusted`).

These are assumed and **do not block `proved`**. Lean takes the same position: a
Lean proof trusts the kernel, the compiler that built it and the machine that ran
it, and `#print axioms` lists only the axioms the proof used. This repository is
stricter in one respect: bucket (a) ids are still printed in every release
(section 8), because a reader deciding whether to rely on a constant-time claim
needs to see that speculation (`uarch.speculation.none`) is assumed.

Two honest caveats, recorded rather than hidden:

- Some (a) ids describe this repository's own models rather than a vendor's
  product: `gate.*` (the gate-level simulator is not yet a netlist) and
  `mc.isa.model-faithful` (the RISC-V simulator). They are baseline by the owner's
  layer rule. Discharging them through VER29's layer contracts is a bonus that
  earns the top rung (section 4), not a requirement for `proved`.
- Some `os` ids describe limits of the runtime jail (`os.counters.not-live`,
  `os.platform.linux-only`, `os.runtime-gate.exercised-paths-only`). They qualify
  jail evidence, not proofs, and appear only in the report-level list today.

### 3.2 Bucket (b): statement assumptions

The `spec.*` ids: the Warrant claim says what we mean (the secret labels are
complete, the observation model covers the attacker, the claims formalise the
prose spec). No proof can establish that a formal statement matches an intention.
A person must read it and agree.

Bucket (b) is closed by a **statement sign-off**:
`code/specs/signoffs/<warrant-stem>.json` with schema
`ver31-statement-signoff-v1`:

| Field | Meaning |
|---|---|
| `warrant` | Repository path of the `.warrant` file. |
| `warrant_sha256` | Digest of that file's bytes at sign-off. Any later edit makes the sign-off stale. |
| `ids` | The `spec.*` ids the signer accepts for that file. |
| `signer` | The principal in `.github/release-signers` (section 9.1). |
| `date` | ISO date. |

A detached signature `<warrant-stem>.json.sig` is made with the same hardware key
as release tags (`ssh-keygen -Y sign -n coding-adventures-statement`). The gate
accepts a sign-off only if all of these hold:

- `warrant` is the package's own Warrant path (the H1 file), so a sign-off copied
  from another package's stem does not verify;
- `warrant_sha256` equals that file's digest at the tagged commit. (The
  evidence manifests record no Warrant digest. The Warrant is tied to the
  evidence by the package BUILD's `gen_evidence --check` drift check, which the
  gate re-runs under F10, so the digest binds the sign-off to the evidence too.
  This replaced "the Warrant digest the manifests bind" in LV-007a.);
- the signature verifies with `ssh-keygen -Y verify -n coding-adventures-statement
  -I <signer>` against the signers file read from `main` (section 9.1), where
  `<signer>` is the record's own `signer` field;
- `ids` covers every (b) id the claims list.

An unsigned, stale, mis-bound or mis-signed record leaves every (b) id open. A sign-off is a recorded human judgement, so the release notes
name it as such and never call it a proof.

### 3.3 Bucket (c): the repository's own gaps

Everything the repository itself is responsible for: the WIR lowering, model,
inliner, engine and proof objects (`wir.*`, `memory.unaliased`), the verification
toolchain (`tcb.*` except the compiler), properties of our own source code
(`src.rust.unsafe-region.sound`, `src.rust.subset-conformance`,
`src.panic.paths-public`), the capability analyzer and effect tooling
(`src.analyzer.*`, `src.effects.*`, `src.registry.*`), unchecked attestations
(`att.*`) and our own process (`gov.*`).

Any open (c) id in a claim's assumption list **blocks `proved`**. It is closed only
by a discharge the checker accepts (VER13 section 6, rule AC-3), never by
argument. (c) ids in the report-level list qualify the weakness matrix and the
capability rows, not a claim, so they do not move a claim's rung; they are printed
with the release all the same.

### 3.4 Kernel-trust candidates (owner decision, 2026-11-23)

Lean trusts a small kernel so that it does not have to trust everything else. The
question was which of our tools play that role. Each candidate was argued; the
owner decided.

| Id | Case for trusting it | Case against | Decision |
|---|---|---|---|
| `tcb.rust-toolchain.trusted` | It is the compiler. Lean trusts its compiler; so does every proof assistant. Distrusting it means distrusting `cc.*`, which is already baseline. | — | **(a)**, trusted |
| `tcb.drat.checker-correct` | `drat-check` is about 1,200 lines, independent of the solver, with exhaustive and reference tests: the closest thing we have to a kernel. | It is ours and has no proof. A small kernel is trusted because it is small and reviewed by many; ours is small and reviewed by us. | **(c)**: verify it (the LCF proof-kernel work is the route) |
| `tcb.report.checker-correct` | Its job is bookkeeping, not establishing truth. | About 13,000 lines, and it grades the tier. Trusting the tool that grades us is the WD-001 failure again. | **(c)** |
| `wir.lowering.faithful` | seL4 trusts its C-to-Isabelle parser; with differential evidence (`wir.interp.differential-only`) the lowering is tested on every run. | It has been wrong twice (`assert_eq!` lowered to a no-op; `vec![0u8; n]` stored past a zero-length buffer). seL4's parser earned its trust over years; ours has not. | **(c)**: translation validation (VER25b) is the route |

A future decision to move an id between buckets is an amendment to this section
and to the bucket file in the same PR, with the owner's sign-off.

### 3.5 The assignment

Counts: (a) 35, (b) 5, (c) 44, total 84 (catalogue v1). The table is the
human rendering of `buckets-v1.json`; the LV-007a check fails when the two drift.

| Id | Layer | Bucket | Rationale |
|---|---|---|---|
| `src.rust.int-semantics` | `src` | a | Source-language semantics (Rust integer arithmetic), part of the platform baseline like the compiler (VER31 section 3.1). |
| `src.rust.safe-bounds-checks` | `src` | a | Source-language semantics (safe Rust indexing panics instead of reading memory), part of the platform baseline (VER31 section 3.1). |
| `tcb.rust-toolchain.trusted` | `wir` | a | The compiler that builds our verifiers. The owner's one kernel-trust decision (2026-11-23): we trust the compiler, as Lean does (VER31 section 3.4). |
| `compiler.preserves_timing` | `cc` | a | Compiler layer: the platform baseline, trusted as Lean trusts its compiler (VER31 section 3.1). |
| `cc.llvm-ir.no-secret-branch` | `cc` | a | Compiler layer: the platform baseline, trusted as Lean trusts its compiler (VER31 section 3.1). |
| `cc.llvm-ir.addresses-public` | `cc` | a | Compiler layer: the platform baseline, trusted as Lean trusts its compiler (VER31 section 3.1). |
| `cc.volatile-honoured` | `cc` | a | Compiler layer: the platform baseline, trusted as Lean trusts its compiler (VER31 section 3.1). |
| `cc.version-pinned` | `cc` | a | Compiler layer: the platform baseline, trusted as Lean trusts its compiler (VER31 section 3.1). |
| `cc.caller-context` | `cc` | a | Compiler layer: the platform baseline, trusted as Lean trusts its compiler (VER31 section 3.1). |
| `mc.backend.no-secret-branch` | `mc` | a | Machine code and ISA layer: the platform baseline (VER31 section 3.1). |
| `mc.backend.addresses-public` | `mc` | a | Machine code and ISA layer: the platform baseline (VER31 section 3.1). |
| `mc.isa.timing-data-independent` | `mc` | a | Machine code and ISA layer: the platform baseline (VER31 section 3.1). |
| `mc.isa.model-faithful` | `mc` | a | Machine code and ISA layer: the platform baseline (VER31 section 3.1). |
| `uarch.cache.address-trace-suffices` | `uarch` | a | Microarchitecture layer: the platform baseline (VER31 section 3.1). |
| `uarch.fetch.pc-trace` | `uarch` | a | Microarchitecture layer: the platform baseline (VER31 section 3.1). |
| `uarch.speculation.none` | `uarch` | a | Microarchitecture layer: the platform baseline (VER31 section 3.1). |
| `uarch.predictor.no-state-leak` | `uarch` | a | Microarchitecture layer: the platform baseline (VER31 section 3.1). |
| `uarch.shared-resources.none` | `uarch` | a | Microarchitecture layer: the platform baseline (VER31 section 3.1). |
| `gate.model.is-netlist` | `gate` | a | Gate and netlist layer: the platform baseline (VER31 section 3.1). Some gate ids describe this repo's own simulators; discharging them is VER29's bonus, not a release requirement. |
| `gate.primitives.correct` | `gate` | a | Gate and netlist layer: the platform baseline (VER31 section 3.1). Some gate ids describe this repo's own simulators; discharging them is VER29's bonus, not a release requirement. |
| `gate.sequential.ideal-dff` | `gate` | a | Gate and netlist layer: the platform baseline (VER31 section 3.1). Some gate ids describe this repo's own simulators; discharging them is VER29's bonus, not a release requirement. |
| `gate.clocking.one-instruction-per-cycle` | `gate` | a | Gate and netlist layer: the platform baseline (VER31 section 3.1). Some gate ids describe this repo's own simulators; discharging them is VER29's bonus, not a release requirement. |
| `gate.memory.abstraction` | `gate` | a | Gate and netlist layer: the platform baseline (VER31 section 3.1). Some gate ids describe this repo's own simulators; discharging them is VER29's bonus, not a release requirement. |
| `gate.isa-equivalence` | `gate` | a | Gate and netlist layer: the platform baseline (VER31 section 3.1). Some gate ids describe this repo's own simulators; discharging them is VER29's bonus, not a release requirement. |
| `phys.silicon.realization` | `phys` | a | Physics floor: no software can discharge it (VER31 section 3.1). |
| `phys.side-channels.out-of-scope` | `phys` | a | Physics floor: no software can discharge it (VER31 section 3.1). |
| `os.allocator.size-only` | `os` | a | Operating-system layer: the platform baseline (VER31 section 3.1). |
| `os.std.runtime-benign` | `os` | a | Operating-system layer: the platform baseline (VER31 section 3.1). |
| `os.time.vdso` | `os` | a | Operating-system layer: the platform baseline (VER31 section 3.1). |
| `os.env.memory-reads` | `os` | a | Operating-system layer: the platform baseline (VER31 section 3.1). |
| `os.syscall.baseline` | `os` | a | Operating-system layer: the platform baseline (VER31 section 3.1). |
| `os.runtime-gate.exercised-paths-only` | `os` | a | Operating-system layer: the platform baseline (VER31 section 3.1). |
| `os.kernel.trusted` | `os` | a | Operating-system layer: the platform baseline (VER31 section 3.1). |
| `os.counters.not-live` | `os` | a | Operating-system layer: the platform baseline (VER31 section 3.1). |
| `os.platform.linux-only` | `os` | a | Operating-system layer: the platform baseline (VER31 section 3.1). |
| `spec.secret-labels.complete` | `spec` | b | Statement assumption: the claim says what we mean. Needs the owner's recorded sign-off of the Warrant statement, not a proof (VER31 section 3.2). |
| `spec.obs-model.complete` | `spec` | b | Statement assumption: the claim says what we mean. Needs the owner's recorded sign-off of the Warrant statement, not a proof (VER31 section 3.2). |
| `spec.obs-model.public-length` | `spec` | b | Statement assumption: the claim says what we mean. Needs the owner's recorded sign-off of the Warrant statement, not a proof (VER31 section 3.2). |
| `spec.platform.declared-none` | `spec` | b | Statement assumption: the claim says what we mean. Needs the owner's recorded sign-off of the Warrant statement, not a proof (VER31 section 3.2). |
| `spec.corpus.ct01-faithful` | `spec` | b | Statement assumption: the claim says what we mean. Needs the owner's recorded sign-off of the Warrant statement, not a proof (VER31 section 3.2). |
| `src.rust.subset-conformance` | `src` | c | Our own front end must reject what it cannot lower; a tool gap (VER31 section 3.3). |
| `src.rust.unsafe-region.sound` | `src` | c | A property of our own unsafe code; Miri or a proof must establish it (VER31 section 3.3). |
| `src.panic.paths-public` | `src` | c | A property of our own source (panic paths branch only on public values), not of the language (VER31 section 3.3). |
| `src.analyzer.G-1` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `src.analyzer.G-2` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `src.analyzer.G-3` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `src.analyzer.G-4` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `src.analyzer.G-5` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `src.analyzer.G-6` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `src.analyzer.G-7` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `src.analyzer.G-8` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `src.analyzer.G-9` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `src.analyzer.correct` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `src.effects.witness-token` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `src.effects.certificate-binding` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `src.registry.excluded-rows` | `src` | c | A limit of our own capability analyzer or effect tooling (VER31 section 3.3). |
| `att.analyzer.name-allowlist-regex-free` | `src` | c | An attestation no tool re-checks; our own gap until a tool replaces it (VER31 section 3.3). |
| `att.ast-audit.no-compression-routines` | `src` | c | An attestation no tool re-checks; our own gap until a tool replaces it (VER31 section 3.3). |
| `att.ast-audit.no-deserialization` | `src` | c | An attestation no tool re-checks; our own gap until a tool replaces it (VER31 section 3.3). |
| `att.ast-audit.no-legacy-ciphers` | `src` | c | An attestation no tool re-checks; our own gap until a tool replaces it (VER31 section 3.3). |
| `att.ast-audit.no-protocol-sinks` | `src` | c | An attestation no tool re-checks; our own gap until a tool replaces it (VER31 section 3.3). |
| `att.ast-call-graph.acyclic` | `src` | c | An attestation no tool re-checks; our own gap until a tool replaces it (VER31 section 3.3). |
| `att.language-type-system.no-prototype-pollution` | `src` | c | An attestation no tool re-checks; our own gap until a tool replaces it (VER31 section 3.3). |
| `att.type-signatures.unconstrained-slices` | `src` | c | An attestation no tool re-checks; our own gap until a tool replaces it (VER31 section 3.3). |
| `att.warrant-ir.cost-semantics-linear` | `src` | c | An attestation no tool re-checks; our own gap until a tool replaces it (VER31 section 3.3). |
| `wir.lowering.faithful` | `wir` | c | Our own Rust-to-WIR lowering, wrong twice before. The owner declined model trust (2026-11-23) (VER31 section 3.4). |
| `wir.lowering.volatile-by-name` | `wir` | c | Our own WIR lowering, model, engine or proof object; a tool gap (VER31 section 3.3). |
| `wir.inline.sound` | `wir` | c | Our own WIR lowering, model, engine or proof object; a tool gap (VER31 section 3.3). |
| `wir.interp.differential-only` | `wir` | c | Our own WIR lowering, model, engine or proof object; a tool gap (VER31 section 3.3). |
| `wir.cost.step-model` | `wir` | c | Our own WIR lowering, model, engine or proof object; a tool gap (VER31 section 3.3). |
| `memory.unaliased` | `wir` | c | Our WIR memory model's fit to the caller's buffers; Rust's aliasing rules may justify it, but no tool checks that yet (VER31 section 3.3). |
| `wir.memory.provenance-model` | `wir` | c | Our own WIR lowering, model, engine or proof object; a tool gap (VER31 section 3.3). |
| `wir.loop.bounded-k` | `wir` | c | Our own WIR lowering, model, engine or proof object; a tool gap (VER31 section 3.3). |
| `wir.bound.heap-unchecked` | `wir` | c | Our own WIR lowering, model, engine or proof object; a tool gap (VER31 section 3.3). |
| `wir.bound.depth-unchecked` | `wir` | c | Our own WIR lowering, model, engine or proof object; a tool gap (VER31 section 3.3). |
| `wir.proof.trivial-miter` | `wir` | c | Our own WIR lowering, model, engine or proof object; a tool gap (VER31 section 3.3). |
| `wir.proof.obligation-binding` | `wir` | c | Our own WIR lowering, model, engine or proof object; a tool gap (VER31 section 3.3). |
| `tcb.drat.checker-correct` | `wir` | c | Our own proof checker. The owner declined kernel trust (2026-11-23): it must be verified, not trusted (VER31 section 3.4). |
| `tcb.bitblast.encoding-correct` | `wir` | c | Part of our own verification toolchain; must be verified, not trusted (VER31 section 3.3). |
| `tcb.symexec.engine-correct` | `wir` | c | Part of our own verification toolchain; must be verified, not trusted (VER31 section 3.3). |
| `tcb.report.checker-correct` | `wir` | c | Our own report checker, which also grades the tier. The owner declined kernel trust (2026-11-23) (VER31 section 3.4). |
| `tcb.evidence.manifest-honesty` | `wir` | c | Part of our own verification toolchain; must be verified, not trusted (VER31 section 3.3). |
| `gov.approval.blocked` | `gov` | c | Our own process gap: exemption approval does not exist yet (VER31 section 3.3). |
| `gov.catalogue.integrity` | `gov` | c | Our own catalogue's completeness; nothing checks it against the tools (VER31 section 3.3). |

---

## 4. The claim ladder

For each claim the gate computes one rung from the report's fields, the bucket
file, the catalogue and the statement sign-off. Earlier rungs are prerequisites of
later ones.

| # | Rung | Computed when | What it says |
|---|---|---|---|
| 1 | `tested` | `example` is `pass`, `property.status` is `pass`, and `fuzz.status` and `exhaustive.status` are `pass` where present | Behaviour checked on chosen, generated and fuzzed inputs. Nothing about inputs not run. |
| 2 | `bounded(N)` | rung 1, generator label `bounded(N)`, and a certificate in `proof.certificate` (`drat:sha256:<64 hex>`; `--check` has verified it) | A checked certificate for every input up to size N. Nothing above N. |
| 2 | `exhaustive` | rung 1, and the claim's `exhaustive` block states `"domain": "full"` | Every input of a finite domain was checked. **Unreachable today:** no report emits `domain` (LV-007d), and the gate never infers it from a case count. |
| 3 | `checked` | rung 1, a proved-family generator label (`proved`, `proved-to-isa`, `proved-to-gate-level`) and a certificate, but an open (c) id or an open (b) id remains | A certificate covers every input, but the result still rests on our own unverified tools or an unsigned statement. **New rung**, added because without it a `proved` label with open gaps would have nowhere honest to land. |
| 4 | `proved` | rung 3's conditions, no open (c) id, and every (b) id covered by a valid statement sign-off | Certificate-checked for all inputs; only baseline assumptions remain. |
| 5 | `proved, baseline discharged` | rung 4, and every (a) id not marked `floor` in the signed bucket file is in `proof.discharged` | As rung 4, with the platform layers discharged too (VER29). Floors (physics, declared caller context) can never be discharged and are listed. |

Rules:

- **Order.** `tested` < `bounded(N)` < `exhaustive` < `checked` < `proved` <
  `proved, baseline discharged`, and `bounded(N)` < `bounded(M)` when N < M.
  `exhaustive` ranks above any `bounded(N)` because it covers its whole domain.
- **A package's claim tier is its weakest claim.** A package with three
  `bounded(128)` claims and one `checked` claim has claim tier `bounded(128)`.
- **`proved-per-run`** (evidence for one pinned build) computes at most
  `checked` until this section is amended for it.
- **Fail closed.** The gate refuses, rather than computing a rung, on: an unknown
  generator label; an assumption id with no bucket or absent from the catalogue; a
  `proof.discharged` id that is not in `proof.assumptions`; a claim with no
  `proof` block when the generator label is not `null`; any report that fails
  `ca-assurance-report --check`.
- The generator label is printed beside the computed rung, so a reader sees both
  `proved` (what the tool wrote) and `checked` (what the evidence supports).
- **The report cannot vouch for itself** (added in LV-007a after its security
  review). Two rules keep a weakened generator or checker, or a hand-edited
  report, from reaching `proved`:
  - **Tool-trust ids are always assumed.** For every claim with a
    certificate, the gate itself adds the (c) ids of our own verification
    tools (`tcb.report.checker-correct`, `tcb.evidence.manifest-honesty`,
    `tcb.drat.checker-correct`, `tcb.bitblast.encoding-correct`,
    `tcb.symexec.engine-correct`), whatever the report lists. The bucket
    file cannot move them out of (c).
  - **A discharge counts only if the catalogue records it.** A
    `proof.discharged` id closes nothing unless the code-owned catalogue marks
    that id `discharged` too. Today the catalogue discharges nothing, so no
    report can raise a claim past `checked`.
- Report strings rendered into notes (claim ids, symbols, labels, exemption
  ids, report-level assumption ids) must be plain text
  (`[A-Za-z0-9_.:/(),<> -]`); anything else is a refusal, so a report cannot
  spoof the tier text the owner reads.

---

## 5. Minimum publishable tier: `Tested`

A package may be published at `Tested`, with major version `0`, when every floor
check passes. The gate computes each one on the tagged commit; none is read from a
file someone typed, except where the row says so.

| Id | Check | How the gate computes it |
|---|---|---|
| F1 | Tests pass | Runs the package's tests (`cargo test -p <crate>`; `npm test`). |
| F2 | Coverage at the repository bar | Line coverage of 80 % or more (CLAUDE.md), measured by `cargo tarpaulin --engine llvm` (pinned as in `ci.yml`) or `vitest --coverage` (v8). The figure is printed; the library target of 95 % is shown, not enforced. |
| F3 | Lint | `cargo clippy -p <crate> --all-targets -- -D warnings`; `tsc --noEmit` under `strict`. |
| F4 | A recorded security review covering the released source | `security-review.json` in the package (schema `ver31-security-review-v1`): `files` (repository path to the SHA-256 of the bytes reviewed), `reviewer`, `date`, `findings` (each `fixed`, `accepted` or `not-applicable`), `outcome`. The gate requires `outcome: pass` and no open finding of severity `high` or `critical`. **The gate computes the scope the review must cover; it does not trust the record's.** The required set is every git-tracked file under the package (source, tests, BUILD files, build and test configuration, manifests, lockfile, evidence), every file in the built artifact, and the build configuration on the package's ancestor path: `.cargo/**`, `rust-toolchain`/`rust-toolchain.toml` and `.npmrc` in the repository root and every directory down to the package, plus the workspace `Cargo.toml`. (LV-007a refinement: the workspace `Cargo.toml` is covered as `code/packages/rust/Cargo.toml#settings`, its bytes with the `members`/`exclude` arrays removed, because every new crate rewrites those arrays (about 100 commits a month) while `[patch]` and `[profile]` stay covered. The workspace `Cargo.lock` is not required: F8 guarantees no runtime dependency, so it cannot change the shipped bytes. A TypeScript package's own `package-lock.json` stays required, because it pins the compiler that builds `dist/`.) The required set excludes exactly `security-review.json`; the gate refuses any
other exclusion, so files regenerated after the review (CHANGELOG,
`assurance_report.json`, `ASSURANCE.md`) need a fresh review record. `scope` must
cover that set (git-tracked files only; build outputs such as `dist/` are covered through the tracked `src/` they are built from, which is always required). Each entry must be a normalized repository-relative path with no `..`, no glob and no leading `:`, and must not be a symlink. Freshness: every listed file's bytes at the tagged commit hash to the recorded digest. (LV-007a replaced "reviewed_commit on `main` plus `git diff`": the repository squash-merges, so a feature-branch commit never lands on `main`; binding the reviewed bytes is squash-proof and stricter.) The record is typed; its coverage and freshness are computed. |
| F5 | Fuzz and property tests on every untrusted-input surface | Every claim of the report has `fuzz.status` and `property.status` equal to `pass`. A lane package (section 6.2) needs its own property and fuzz suites over each public operation (VER05a, VER06b engines), declared as the npm scripts `test:property` and `test:fuzz` and run by F1. |
| F6 | Recorded benchmarks | `evidence/bench-manifest.json` exists, parses as `warrant-bench-evidence-v1`, has at least one `pass` entry, and is fresh: the package BUILD regenerates it and F10's reproduction check requires the committed bytes to match. |
| F7 | README and CHANGELOG | `README.md`; `CHANGELOG.md` or `CHANGELOG.d/_meta.md`; the changelog names the version being released. |
| F8 | Zero third-party runtime dependencies | Cargo `[dependencies]`, `[build-dependencies]` and every `[target.*.dependencies]` are empty (read with a real TOML parser); npm `dependencies`, `peerDependencies`, `optionalDependencies` and `bundleDependencies` are empty. Development dependencies are allowed because they do not ship. The check runs twice: on the tree's manifest, and (F8b) on the manifest **inside the artifact**, whose name, version, licence and dependencies must equal the tree's and the tag's, because a build step could rewrite `package.json` before packing. |
| F9 | Licence | Manifest licence `MIT` and a `LICENSE` file inside the package. |
| F10 | The generated report ships verbatim and is about this code | `ca-assurance-report --check` passes at the tagged commit (and `--check-lane` for a lane package). The report is fresh: the gate re-runs the package BUILD, which re-derives every evidence manifest from real runs (and checks the generated harness against the `.warrant`), and then requires `git diff --exit-code` on the package directory, so the committed evidence must reproduce byte for byte. (LV-007a replaced "the report's `commit` is an ancestor of the tag": the `commit` field is a generator argument, and squash merges keep feature-branch commits off `main`.) The committed `ASSURANCE.md` must equal a fresh rendering. The built artifact contains `assurance_report.json` byte-identical to the checked file, and a generated `ASSURANCE.md` (section 8), even when the verdict is `report-only`. |
| F11 | Version | The tag version equals the manifest version; the major version is `0` unless the release tier is `proved` or higher; the version is not already on the registry. |
| F12 | Supply chain | Published by OIDC trusted publishing with no long-lived registry token in the workflow, with a build-provenance attestation for the exact artifact. The one exception is a package's first version (section 9.3). |

A package that fails any floor check is not published at any tier.

---

## 6. Higher release tiers

### 6.1 Conditions

The release tier is the package's claim tier (section 4) only when all of these
hold as well; otherwise it is capped at `Tested`, and the release notes say which
condition capped it. These are today's LV-007 requirements, kept, with "meets the
bar" defined.

| Id | Condition | Computed from |
|---|---|---|
| H1 | The package has a Warrant spec, and every claim's evidence derives from it | The report's evidence ids are `warrant/...` and the `.warrant` file exists at the tagged commit. |
| H2 | The report meets the bar | **"Meets the bar" means verdict `pass`** (VER01 section 3.4): every rung E0–E5 `pass` (so E2 is live), every weakness row `excluded` or `discharged` or an approved exemption, and no public symbol without a claim. |
| H3 | Live runtime-jail evidence | A strict, live VER08b run (`RCG_STRICT=1`, Linux) of the package at the tagged commit, which the gate re-runs (the package's `BUILD_linux` strict line, with `strace` installed). NOT-LIVE fails. Needs CAP-RT-002 and CAP-RT-003. |
| H4 | No unapproved exemption | Every `exemptions[].approved` is `true`. Implied by H2, listed because it is the usual blocker. |

H2 and H4 cannot hold while VER00 section 12.1 blocks exemption approval, so no
package can be released above `Tested` until that is resolved. This is stated in
every capped release's notes, so the cap is visible instead of silent.

A release tier of `proved` or higher may use major version `1` or above.

### 6.2 Lane packages

A package in a non-Rust lane (VER04i), such as `typescript/ct-compare`, has no
report of its own. The gate finds the single report whose `lanes` array names the
package (two reports naming one lane is a refusal). A lane row with rung `live`
gives the package rung `tested` for every claim; `planned` or `absent` refuses.
A lane package is at most `tested` until lanes carry proofs. It still meets every
floor check itself, and it ships the parent report verbatim. (Proposed amendment,
pending owner signature, section 13.4: the cap lifts per claim and per lane only
through the claim's own signed goal, and never past what the open ids allow.)

---

## 7. The gate

The gate is a step of `.github/workflows/publish.yml` (LV-007a) that runs before
every publish job of every language, and in a dry-run mode on demand.

- **Inputs:** the tagged commit; the package directory from the tag; the
  package's report (or the parent report for a lane package); `buckets-v1.json`;
  `catalogue-v1.json`; an optional statement sign-off; `.github/release-signers`
  read from `main`.
- **Program:** `code/scripts/release_gate.py`, standard library only, with
  falsification tests under `code/scripts/tests/`. Subcommands: `check` (rungs,
  claim tier, release tier, bucketed assumptions, floor and authorization
  checks; writes `gate.json`), `notes` (release notes), `assurance` (write or
  `--check` the committed `ASSURANCE.md`), `lane-report` and `verify-tag`. The
  workflow runs the copy of the script **on `main`**, not the tagged copy.
  `gate.json` is uploaded with the artifact.
- **The tool is in bucket (c).** The gate program is ours and unverified, like
  `ca-assurance-report` (`tcb.report.checker-correct`). Its tests are the
  evidence; the release notes say so.
- **Privilege split.** GitHub grants permissions per job, not per step, so the
  gate is three jobs.
  - `gate` has `permissions: contents: read` only. It runs the package BUILD,
    every test, lint and coverage step, which is where third-party development
    dependencies execute, and builds the artifact. It requires the BUILD to
    reproduce the committed evidence and leave no untracked file. It uploads the
    raw artifact, `lcov.info` and a small `gate-facts.json`. It does **not**
    decide anything: code running in that job could rewrite any file the job
    later reads, including a gate script or git itself (LV-007a security
    review).
  - `judge` (`contents: read`, a fresh runner) runs no package code. It checks
    out the tagged tree; builds `ca-assurance-report` from `main` in a separate
    worktree outside that tree and runs it against the report; re-packages a
    crate itself (`cargo package --no-verify`, toolchain pinned) and requires
    the gate's `.crate` to match byte for byte; asks the registry about the
    version; and runs `main`'s `release_gate.py`. Its `gate.json` is the only one
    the later jobs trust.
  - `attest` needs `judge` and `verify-tag`, so no attestation is minted for an
    unverified tag. It checks out no package source and installs nothing.
    It downloads the judged artifact, refuses unless its name is a plain
    `*.crate`/`*.tgz`, its digest equals the judged `gate.json` and the decision
    is `publishable`. It is the only job with `id-token: write` and
    `attestations: write` outside the `release` environment.

  No job that runs package, test or dependency code holds `id-token: write`.

  **Residual risk, stated in every release's notes:** F1-F3 (tests, coverage,
  lint), the evidence-reproduction result and, for npm, the built `dist/` are
  produced in `gate` next to the package's own code. The judge re-derives what
  it can without running that code (the report check, the crate bytes, the
  shipped manifest, every tier); it cannot rebuild `dist/` without running the
  compiler the lockfile pins, so an npm tarball's compiled JavaScript is only as
  trustworthy as the reviewed `src/`, lockfile and build configuration (F4).
- **Publish uses the judged bytes.** npm publishes the judged tarball, after
  checking that the `package.json` inside it names the judged package and the
  tag's version (npm publishes whatever the tarball says), with
  `--ignore-scripts`. crates.io
  cannot upload a prebuilt file, so its publish job runs `cargo package
  --no-verify`, refuses unless the `.crate` digest equals `gate.json`, runs `cargo
  publish --no-verify` (verification already ran in `gate`), then downloads the
  published crate from `static.crates.io` and compares the digest. A mismatch fails
  the job, and the owner yanks the version. Python and Ruby artifacts are also
  built in `gate`, recorded in `gate.json` and attested. Their publish jobs only
  download, compare digests and upload, and run no build tool, because every
  publisher pins the same `release` environment and a build backend running in a
  publish job could otherwise publish any package. `cargo package` must be
  reproducible for the digest comparison to pass; if it is not, the job fails
  closed. Because `cargo publish` runs in the tagged tree while holding the
  token, the crates.io job refuses any `.cargo/config` on the package's
  ancestor path that sets a credential provider, proxy, CA, source,
  registry, alias or env table, and pins `RUSTUP_TOOLCHAIN`,
  `CARGO_REGISTRY_CREDENTIAL_PROVIDER`, `CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS`
  and `CARGO_HTTP_CAINFO` in the environment, which overrides every config
  file.
- **Every job checks out `${{ github.sha }}`**, never the tag name, so a tag moved
  mid-run cannot change what is built.
- **Dry run:** `workflow_dispatch` with a package path and a ref runs `gate`
  only: no `attest` job, no publish job, no tag-signature check, so a dry run of
  unmerged code never produces a genuine attestation. Its `package` and `ref`
  inputs reach the shell only through `env:`, and must match the same strict
  pattern as release tags. The owner never approves a `release` deployment for a
  `workflow_dispatch` run.
- **Every language:** the Python and Ruby publish jobs also depend on the gate
  and run in environment `release`. Until LV-007c gives those packages reports
  and moves their builds into `gate`, the gate refuses them outright, as P13's
  hold did; LV-007c also moves RubyGems to trusted publishing.

Tags keep the existing format `<language>/<name>/v<semver>`. For `rust/` and
`typescript/`, `<name>` is the package **directory** (`rust/ct-compare/v0.1.0`),
and the gate reads the registry name from `Cargo.toml` or `package.json`. Python
and Ruby keep their registry-name tags. The tag never enters a `run:` script; the
existing pattern check, environment-variable passing and on-`main` check stay.

---

## 8. Release notes and `ASSURANCE.md`

Both are generated by `release_gate.py notes` from `gate.json`. Nothing in them is
typed. Shape:

```
# <registry name> <version> — release tier: Tested
Computed from assurance_report.json (sha256 …), buckets-v1.json (sha256 …),
catalogue-v1.json (sha256 …) at commit <sha>.

## Claims
| claim | symbol | generator label | computed rung | open (c) | (b) |

## Why the release tier is Tested        (only when capped)
- H2: verdict is report-only (E2 not-live; 5 unapproved exemptions)
- H3: no live runtime-jail evidence for this commit

## Floor checks
| F1 … F12 | result | value (coverage %, test count, …) |

## Assumptions (like Lean's #print axioms)
### (a) accepted platform baseline — 28 ids, assumed
### (b) statement — 5 ids, not signed off
### (c) own gaps — 20 ids, open; each blocks `proved`
### Report-level assumptions — 33 ids, by bucket
## Exemptions
```

The full list is printed, not a count. `ASSURANCE.md` holds the same assumption
and claim sections and ships inside the package.

---

## 9. Authorization and supply chain

### 9.1 Hardware-key-signed tags

The owner signs every release tag with a FIDO2 security key: an
`sk-ssh-ed25519@openssh.com` SSH key (created with `ssh-keygen -t ed25519-sk -O
verify-required`), with git's `gpg.format=ssh`. The public key is listed in
`.github/release-signers` (OpenSSH `allowed_signers` format, with
`namespaces="git,coding-adventures-statement,coding-adventures-buckets,coding-adventures-signers"`),
added by the owner in a code-owned PR (section 9.6).

The signers file is a chain. Each version carries a `# revision: <n>` line and a
detached signature `.github/release-signers.sig` (namespace
`coding-adventures-signers`) made by a key listed in the **previous** version. The
gate walks `main`'s first-parent history of the file (`git log --first-parent
refs/remotes/origin/main -- .github/release-signers`) and verifies every
transition. A missing `.sig`, or a gap or repeat in the revision sequence, is a
refusal. This means a newly added key cannot authorize itself, and an older
revision cannot be restored. The
first version is the trust anchor: the owner adds it, and every release's notes
print its fingerprint. OpenSSH's `allowed_signers` format has no
`verify-required` option (its options are `cert-authority`, `namespaces`,
`valid-after` and `valid-before`; checked against OpenSSH 10.3 in LV-007a), so the
gate accepts only `namespaces=` and the flag check in step 4 below is the control
for user verification. Removing the flag check is never the fix.

The gate's `verify-tag` job:

1. reads `.github/release-signers` from `refs/remotes/origin/main`, never from the
   tagged tree, so a tag cannot bring its own signer;
2. refuses an empty file, any key line whose type is not a security-key type, and
   any option other than `namespaces=`;
3. fetches the tag object explicitly (`git fetch origin
   +refs/tags/$TAG:refs/tags/$TAG`, since `actions/checkout` may not keep the
   annotated tag object), and refuses a lightweight tag;
4. runs `git verify-tag` with that file as `gpg.ssh.allowedSignersFile`, and
   parses the SSHSIG blob to refuse a signature whose security-key flags lack user
   presence (`0x01`) or user verification (`0x04`). Key-creation options are
   honoured by the signing client; the verifier checks the flags itself;
5. requires `git rev-parse "refs/tags/$TAG^{commit}"` to equal `GITHUB_SHA`, and
   `GITHUB_SHA` to be on `refs/remotes/origin/main`.

### 9.2 The `release` environment

Every publish job runs in a protected GitHub environment named `release`, with
the owner as required reviewer, deployment restricted to tags matching
`rust/**`, `typescript/**`, `python/**` and `ruby/**` (in GitHub's patterns `*`
stops at `/`, so `rust/*` would not match `rust/ct-compare/v0.1.0`), and
administrator bypass off. The owner creates the release that triggers the
workflow.

GitHub runs `publish.yml` as it exists at the tagged commit, so the checks in that
YAML are only as trustworthy as the commit. A tag on an unmerged commit could carry
a `publish.yml` with `verify-tag` deleted. Three controls outside the YAML close
that:

1. **Tag protection** (section 9.6): only the owner can create release tags.
2. **Environment-pinned publishers:** every trusted-publisher registration (crates.io,
   npm, PyPI) pins repository, workflow `publish.yml` **and** environment
   `release`, so a run that skips the environment gets no registry token.
3. **The owner's approval is informed:** before approving a `release`
   deployment, the owner confirms that the run's event is `release`, that its
   commit is the tag they signed (`git verify-tag` locally), and that the
   `verify-tag` job passed.

A release therefore needs two deliberate acts by the owner (a key touch with PIN,
and an informed approval) and passes registry checks that the workflow cannot
rewrite.

**Agents must not hold the owner's GitHub identity.** Controls 1 and 3 trust the
owner's account. A required reviewer can approve a pending deployment through the
REST API, and an admin can bypass rulesets and merge past code-owner review. So
anyone holding the owner's token can get past them with no key touch and no human,
and today the agents in this repository use that token. The rule therefore is:

- Agents and automation use a separate machine account or fine-grained token that
  is not a repository admin, is not a bypass actor on any ruleset, is not a
  required reviewer of `release`, and has no Administration or Environments
  permission.
- The owner's own token is kept off agent hosts.
- The owner approves `release` deployments only in the web UI, never through the
  API.

Until the machine account exists, every release's notes state the residual risk
plainly: an attacker holding the owner's credential and a commit they control is
stopped only by the approval step, which the same token can perform through the
API. In practice nothing stops them until the machine account exists. The signers
chain does not help here, because that attacker supplies the YAML that runs it.
Whether "machine account in place, owner token off agent hosts" is a precondition
of the first release is the owner's risk decision, recorded in the LV-007b
hand-off. The named upgrade is a
custom deployment protection rule (a GitHub App) that verifies the signed tag
outside the tagged YAML, so the check no longer depends on the workflow file the
tag carries.

### 9.3 Trusted publishing and the first version

| Registry | Steady state | First version |
|---|---|---|
| crates.io | OIDC via `rust-lang/crates-io-auth-action` (SHA-pinned) in the `release` environment | crates.io has no pending publishers, so the owner publishes the first version by hand from the tagged commit. Steps: verify the gated `.crate` (below); confirm `cargo package --no-verify` reproduces its digest; run `cargo publish --no-verify`. The token has endpoint scope `publish-new` only, crate scope exactly that crate's name, and an expiry of one day or less. It is passed as `CARGO_REGISTRY_TOKEN` for that single command, never through `cargo login` or `~/.cargo/credentials.toml`, and revoked immediately afterwards. Then the owner registers the trusted publisher (repository, `publish.yml`, environment `release`). |
| npm | OIDC with `npm publish --provenance --access public`, no token | npm has no pending publishers either. The owner verifies the gate's tarball (below) and publishes it once (`npm publish <tarball>`, with security-key 2FA), then registers the trusted publisher (repository, `publish.yml`, environment `release`). The scope `@coding-adventures` requires the owner to create the npm organisation first. |
| PyPI | OIDC (existing), with PyPI's attestations | Pending publishers make OIDC possible from the first version. Existing registrations that do not pin environment `release` are re-registered before LV-007a ships. |
| RubyGems | API key (existing) | LV-007c: move to trusted publishing. |

**Verifying a gated artifact by hand:** `gh attestation verify <artifact> --repo
<owner>/coding-adventures --signer-workflow
<owner>/coding-adventures/.github/workflows/publish.yml --source-ref
refs/tags/<tag>`, plus a comparison of the artifact's SHA-256 with the `gate.json`
from the same run. Pinning the signer workflow and the source ref matters: a plain
`--repo` check would also accept an attestation minted by some other workflow or
ref.

The publish job checks whether the package already exists on the registry; if
not, it stops with these first-version instructions instead of failing obscurely.

### 9.4 Provenance

npm provenance comes from `--provenance` (Sigstore). The `.crate` file and every
other artifact get a GitHub build-provenance attestation
(`actions/attest-build-provenance`, SHA-pinned), verifiable with
`gh attestation verify`.

### 9.5 Deferred upgrade

VER00 sections 9.4 and 9.5 describe phone-plus-hardware-key approval of
exemptions and releases. It remains blocked by VER00 section 12.1 and is the named
upgrade path: when it lands, it replaces the `release` environment approval and
can approve exemptions (H4). Until then, signed tags plus environment approval are
the release authorization, and exemptions stay unapproved.

### 9.6 Trust roots and tag protection

These files decide what a release may claim or who may authorize it:

- `.github/release-signers`
- `.github/workflows/publish.yml`
- `code/scripts/release_gate.py`
- `code/assumptions/buckets-v1.json` and `code/assumptions/catalogue-v1.json`
- `code/specs/signoffs/`
- the tools a report's numbers come from: `ca-assurance-report`, `drat-check`,
  `warrant-grammar`, `warrant-ir` (the judge also runs `main`'s copies, not the
  tagged tree's)
- build configuration cargo reads while the crates.io token is held:
  `.cargo/` directories, the workspace `Cargo.toml`, `rust-toolchain*`

Auto-merge must not move them. The controls:

1. **Code owners:** `.github/CODEOWNERS` lists them with the owner. The `main`
   ruleset requires code-owner review, so a PR that touches one waits for the
   owner even when auto-merge is set. The repository has no CODEOWNERS file today;
   LV-007a adds it.
2. **Signed bucket file:** the gate refuses a `buckets-v1.json` whose exact bytes
   do not verify against the detached signature `buckets-v1.json.sig` (namespace
   `coding-adventures-buckets`, a key from the signers file on `main`). The catalogue itself is not
   signed, because it changes with every VER13 edit. What a release relies on from
   it (each id's bucket and floor flag) is copied into the signed bucket file and
   cross-checked. That record is signed by the
   owner's key (namespace `coding-adventures-buckets`) and verified like a
   statement sign-off. Until the owner signs one, every release is refused.
3. **Tag protection:** a repository ruleset on `refs/tags/rust/**`,
   `refs/tags/typescript/**`, `refs/tags/python/**` and `refs/tags/ruby/**`
   restricts creation, update and deletion to the owner, with no other bypass
   actors.

The owner configures items 1 and 3 in repository settings; the gate cannot.
LV-007b's hand-off checklist lists them, and the release notes state whether the
gate could confirm them (it reads the rulesets API where the workflow token allows,
and otherwise says "not confirmed" rather than guessing).

---

## 10. Current state, computed (main at the time of writing)

`rust/ct-compare` (report verdict `report-only`, 4 claims, 53 assumption ids per
claim, 33 report-level ids):

| Claim | Generator label | Computed rung | Open (c) | (b) |
|---|---|---|---|---|
| `ct_eq_soundness` | `bounded(128)` | `bounded(128)` | 20 | 5 unsigned |
| `ct_eq_fixed_soundness` | `bounded(128)` | `bounded(128)` | 20 | 5 unsigned |
| `ct_select_bytes_soundness` | `bounded(128)` | `bounded(128)` | 20 | 5 unsigned |
| `ct_eq_u64_soundness` | `proved` | **`checked`** | 20 | 5 unsigned |

Per claim: 28 (a), 5 (b), 20 (c). The 20 open (c) ids are `memory.unaliased`,
`src.panic.paths-public`, `src.rust.subset-conformance`,
`src.rust.unsafe-region.sound`, `tcb.bitblast.encoding-correct`,
`tcb.drat.checker-correct`, `tcb.evidence.manifest-honesty`,
`tcb.report.checker-correct`, `tcb.symexec.engine-correct`,
`wir.bound.depth-unchecked`, `wir.bound.heap-unchecked`, `wir.cost.step-model`,
`wir.inline.sound`, `wir.interp.differential-only`, `wir.loop.bounded-k`,
`wir.lowering.faithful`, `wir.lowering.volatile-by-name`,
`wir.memory.provenance-model`, `wir.proof.obligation-binding` and
`wir.proof.trivial-miter`. Report-level: 7 (a), 26 (c).

- Claim tier: `bounded(128)`.
- H1 holds (`code/specs/ct-compare.warrant`). H2 fails (verdict `report-only`: E2
  `not-live`; five unapproved exemptions: CWE-416, 415, 476, 190 unsafe exception,
  CWE-208 gate level). H3 fails (no live jail evidence bound to a commit). H4 fails.
- **Release tier: `Tested`**, once the floor gaps close (LV-007b): no
  `security-review.json` (F4), no licence field or `LICENSE` (F9), the report not
  in the crate (F10). For `typescript/ct-compare`: also no property or fuzz suite of
  its own (F5), no bench manifest (F6), and `main` points at TypeScript source
  instead of built output.

Nothing in the repository can reach `proved` today: every proof label rests on
the (c) ids above. That is the honest position, and the LCF proof-kernel work and
VER25b translation validation are what move it.

---

## 11. Amendments to other documents

| Document | Change |
|---|---|
| VER27 principle P13 | Reworded: publish only what the evidence supports, at the tier the gate computes (VER31), instead of holding every release. |
| VER27 section 8.1 | Replaced by a pointer to this document. |
| VER27 section 8 step 16, section 9 row "Nothing … has been published", decision table row "Release standard" | Point at VER31; the blockers (CAP-RT-002/003, VER00 §12.1) now cap tiers instead of blocking releases. |
| VER-BACKLOG LV-007 | Points at VER31. New rows LV-007a, LV-007b, LV-007c, LV-007d. |
| VER01 section 3.4 | Unchanged. Its `pass` verdict is H2 here; its statement that a `report-only` report "cannot be published" is superseded for the `Tested` tier, which ships the `report-only` report verbatim. |
| VER13 | Unchanged. The catalogue's digest is unchanged; buckets live beside it. |
| VER32, VER33, VER34 (follow-up 2026-11-24) | **Pointer only: the amendment text is not written here.** [VER34](VER34-constraint-vm-smt-solver.md) CVM-170b, a spec-only change, **owns** the text: the certificate scheme `pif:sha256:` is accepted, the evidence kind `solver-attested` is capped at `tested` and is never accepted as `proof.certificate`, and the DRAT certificate record `(VC digest, k, CNF digest, DRAT digest)` of VER34 6.7. VER32 PIF-020 owns the sign-off file, the goal-signing aggregate, and the model-theorem category with its link status; VER33 ADL-006b cites both. **2026-11-25: drafted in section 13, proposed, pending owner signature.** PIF-020 and ADL-006b are written there; the CVM-170b text is cited, not copied (13.6) |

## 12. Open items

- LV-007d: the report generator should emit `exhaustive.domain` so that rung 2's
  `exhaustive` becomes reachable, and could emit buckets directly once this spec
  settles.
- The statement sign-off schema (section 3.2) and the security-review schema
  (F4) are specified here and implemented with the gate (LV-007a).
- Whether `proved-per-run` deserves its own rung is left for when a package
  produces one.
- **Amendments owned elsewhere (follow-up 2026-11-24).** The `pif:sha256:` scheme, the
  `solver-attested` kind and the DRAT certificate record are written by VER34 CVM-170b; the
  sign-off file (VER32 3.9 gives the format `pif-signoff 1` that PIF-003 implemented), the
  goal-signing aggregate and the model-theorem category are written by VER32 PIF-020. The
  rung table of section 4 is not edited until those land. The ladder is used as is
  (VER34 13: "The claim ladder is used as is"). Until then `drat:sha256:` remains the only
  `proof.certificate` scheme this document names, with VER34 6.7's four rules limiting what
  the DRAT route may claim. **2026-11-25:** the PIF-020 and ADL-006b text is drafted in
  section 13 below, proposed and pending owner signature; the rung table of section 4 is
  still not edited, and `drat:sha256:` stays the only scheme in force until the owner signs.

### Follow-up 2026-11-24 (cross-spec follow-up; VER31 has no review-record section, so this is its equivalent)

| # | Change | Source | Where |
|---|---|---|---|
| V1 | A row in section 11 and a bullet in section 12 point at **CVM-170b**, which owns the amendment text (`pif:sha256:` scheme, `solver-attested` capped at `tested`, the DRAT certificate record). **No rung or rule is edited here** | VER34 13.1 (VER31 1) | 11, 12 |
| V2 | The same row and bullet record that **PIF-020** owns the sign-off file (format in VER32 3.9), the goal-signing aggregate and the model-theorem category with link status and domain quantified, which VER33 ADL-006b and ADL-006c cite | VER32 follow-up F5, F8, F13; VER33 13.1 | 11, 12 |
| V3 | Nothing in the owner decisions of 2026-11-24 changes a VER31 rule. `kernel.logic.consistent` stays (c) and open; classical claims reach `checked` at best | VER34 6.8, VER32 12.1 | none |

**Not applied, and why.** The amendment text itself is deliberately not written in this PR:
VER34 assigns it to CVM-170b so that it ships with, and is reviewed against, the
reconstructor's rules.

---

## 13. Amendments proposed 2026-11-25 (PIF-020, ADL-006b): pending owner signature

**Status: proposed, pending owner signature.** This section is dated text for the owner
to read and sign. Until the owner signs it, sections 1 to 12 are the rule, the gate
(`release_gate.py`) behaves as it does today, and the rows that would implement it
(PIF-020's gate half, ADL-006c1 to c3) stay unstarted. It follows VER31's own rule for
changing a trust root (section 3.4): an amendment to this specification, reviewed in
the same pull request as any bucket-file change, and signed by the owner. **No bucket
moves here.** Every id the amendments name is either already bucketed or a new id with
a proposed bucket (13.7); none changes bucket.

**Sources, one per change.** Each sub-section names the text it takes over:

| Sub-section | Change | Source |
|---|---|---|
| 13.1 | The sign-off file `pif-signoff 1` and the goal-signing aggregate | [VER32](VER32-proof-interchange-format.md) 3.7 (E1, E6), 3.9, 8.3; backlog PIF-020 |
| 13.2 | The certificate kind `pif:sha256:` | VER32 8.2 (V3), 8.3, owner question Q5 |
| 13.3 | The model-theorem category and its link status | VER32 9.2 ("VER31 status of a model theorem"), Q8; [VER33](VER33-warrant-algorithm-definition-language.md) 1.4, 1.5, 9.2 |
| 13.4 | The cap on lane packages (section 6.2) | VER33 review finding 12, 9.2, 13; backlog ADL-006b |
| 13.5 | The gate's computation of the A-rung | VER33 1, 1.5, Q7; backlog ADL-006b, ADL-006c1 to c3 |
| 13.6 | The `solver-attested` evidence kind | [VER34](VER34-constraint-vm-smt-solver.md) 6.6, 6.7, backlog CVM-170b (cited, not copied) |
| 13.7 | The ids G-M1 opens | VER32 9.2, 11 (PIF-016a); VER33 9.1 (ADL-006a); VER11a 11.2 |

### 13.1 The sign-off file and the goal-signing aggregate (PIF-020)

Section 3.2 defines a statement sign-off for a Warrant file: JSON, one `warrant_sha256`,
the `spec.*` ids. A proof-interchange claim needs a different record, because what a
person signs is a set of **goals** over a **base environment**, not one file's bytes. VER32
3.9 pins that record's bytes (`pif-signoff 1`); this section says what the gate does with it.

**The file.** `pif-signoff 1`, exactly as VER32 3.9 spells it (ASCII, single spaces, LF
after every line including the last, no blank lines or comments):

```
pif-signoff 1
base prelude sha256:<64 hex>
base adl     sha256:<64 hex>          (zero or more, in chain order)
base model   sha256:<64 hex>          (at most one)
base axioms  sha256:<64 hex>          (at most one, last; classical claims only)
base-environment <64 hex>
goal <claim id> <64 hex>              (one or more; ids unique)
goals-sha256 <64 hex>
```

The file is **complete or refused**. It lists the signed base in order with each file's role
(the roles are not part of the environment fold, VER32 8.3, so these lines are what fix
them), the base environment digest, one goal line per claim, and the aggregate.

**The goal-signing aggregate.** For each claim, form the line

```
<claim-id> <goal-digest> <base-environment-digest>
```

with both digests as **bare 64-digit lowercase hex** and the line ended by **LF**. Sort the
lines **bytewise** (as whole lines; the space after the id sorts below every character
a claim id may contain, so this equals sorting by id). The aggregate, written `goals-sha256`,
is the SHA-256 of the concatenation of the sorted lines. The goal digest is the SHA-256 of
the binding CTE of the theorem's type (VER32 3.6, G5, 8.3). Worked example, computed with a
standard SHA-256, so that a second implementer can check the bytes. Take the base
environment `830915798904748f31b45c2c2edcc174828cd8bae1eb3c2545efc934d28db7ca` and two claims
(the digests here are of the ASCII strings `example base environment`, `example goal one`
and `example goal two`, standing in for real ones):

```
demo.claim.one fd3ed41b408a3b1ecf70e3138977169e7c295e3b540f226a92d921bbb1a10e34 830915798904748f31b45c2c2edcc174828cd8bae1eb3c2545efc934d28db7ca
demo.claim.two de9e44d4ee43c0ecdb0559858f104ac9248651f9c862ce9955d4616478b22454 830915798904748f31b45c2c2edcc174828cd8bae1eb3c2545efc934d28db7ca
```

(two lines, in this order, each ending in LF) hash to
`59d761bf40f29f3b87d0631393c3af71d594758a82899b8366e66b1a70e5d8c5`, which is that sign-off's
`goals-sha256`. This supersedes the `goals_sha256` amendment once listed under KRN-011
(VER11a 7.4), which bound the printed goal without the base.

**What the signature covers.** The detached signature is made over the **exact bytes of the
whole sign-off file**, so it covers the base chain with roles, the base environment digest,
every goal line and the aggregate. It binds the **base** environment, **taken from the
sign-off, not from the proof file** (VER32 E1, E6): the claim file and any proof-side files
(VER32 E7) are the proof, they are not allow-listed and not signed, and a regenerated proof
neither invalidates the signature nor needs one. The aggregate carries the base environment
digest on every line so that a goal can never be moved to a different base without
changing the signed bytes. In `pif-signoff 1` all claims in one file share one base chain,
so the **sign-off records each claim's base chain** by having that single chain on its `base`
lines; a package whose claims need two chains uses two sign-off files.

**Where it lives and how the gate verifies it.** Like section 3.2's records, under
`code/specs/signoffs/` (already a trust root, section 9.6), as
`<stem>.pif-signoff` with the detached signature `<stem>.pif-signoff.sig`. The signature
namespace is `coding-adventures-statement`, the one the signers file already allows
(section 9.1), so no signers-chain revision is needed; the magic line `pif-signoff 1` and
the JSON of section 3.2 cannot be mistaken for each other. Because the format has no signer
field (VER32 3.9 fixes it as complete), the gate runs `ssh-keygen -Y find-principals` and
then `-Y verify -n coding-adventures-statement` against the signers file read from `main`
(section 9.1), and the release notes print the principal that verified. The gate accepts
the sign-off for a claim only if all of these hold:

- `pif-check` (re-run by the judge, section 7, never taken from the report) computes the
  same `base-environment` digest from the base files it resolved, the same goal digest for
  every claim the claim file proves, and the same aggregate; the report prints
  `goals_sha256_verified: true` only then;
- every `base` file is on the allow-list under the role the sign-off gives it (VER32 E4);
- the signature verifies; and
- the claim's goals are all `goal` lines of the file (a claim file proving a goal that is
  not signed is refused, VER32 G6).

**What a valid sign-off closes.** It closes (b) ids only, and only those the claim lists:
`spec.goal.faithful`; `pif.prelude.faithful` when a `base prelude` line is present;
`adl.prelude.faithful` when `base adl` lines are present; `adl.wir-abi.faithful` when the
encoding file is a `base` line; and the package's `<pkg>.model.faithful` (for the logic
gates, `gates.model.faithful`) when a `base model` line is present. It never closes a (c)
id. A stale, unsigned, mis-bound or mis-signed record leaves every one of them open, as
section 3.2 says of a statement sign-off, and the release notes name it as a recorded human
judgement, never a proof.

### 13.2 The certificate kind `pif:sha256:` (PIF-020, owner question Q5)

`proof.certificate` may carry `pif:sha256:<64 hex>` in addition to `drat:sha256:<64 hex>`
(section 4, rung 2). It **supersedes `lean-export:`**, which VER11a 11.3 named and no report
ever carried (VER32 8.3). The digest is the SHA-256 of the **claim file**. The certificate
record, which the report carries and `--check` recomputes, holds:

| Field | Meaning |
|---|---|
| `file_sha256` | The claim file's digest (the value after `pif:sha256:`). |
| `base_environment_sha256` | The fold over the signed base only (VER32 8.3). This is the digest the sign-off signs. |
| `environment_sha256` | The same fold continued over the proof side and the claim file. Recorded, **not signed**. |
| `kernel` | The kernel crate's name, **version and commit**, and the reader's. |
| `signoff_sha256` | The digest of the sign-off file the check used. |

The kernel version and commit are recorded because an implementation bug in the kernel
changes no `logic` number and so withdraws evidence by crate version and commit (VER32 V3):
a gate that holds a list of withdrawn kernels refuses a certificate whose `kernel` is on it
(where that list lives is an open item, 13.8).

**Tool-trust ids are per scheme.** Section 4's rule that the gate itself adds the (c) ids
of our own verification tools, whatever the report lists, applies to a claim with a
`pif:` certificate as follows. The gate adds `tcb.report.checker-correct` and
`tcb.evidence.manifest-honesty` (as for every certificate), and adds
`kernel.checker.correct`, `kernel.logic.consistent`, `pif.reader.correct`,
`tcb.pif.gate-binding` and `tcb.pif.printer`. It does **not** add the three solver ids
(`tcb.drat.checker-correct`, `tcb.bitblast.encoding-correct`, `tcb.symexec.engine-correct`)
for a `pif:` claim whose proof uses no solver (VER34 6.5: "none from the solver"); a claim
whose claim file is a reconstructed solver proof lists the ids VER34 6.7 names. The bucket
file cannot move any of these out of (c), and a `proof.discharged` id closes nothing unless
the catalogue marks it `discharged` too, both as section 4 already says. Bucket of
`kernel.logic.consistent`: (c) and open, as follow-up item V3 records; classical claims
reach `checked` at best until the owner decides VER11a Q5.

What a `pif:` certificate is worth on the ladder is VER34 6.5 and 6.7's table: a bounded
statement is `bounded(k)` with no solver id; a statement for every input is `checked`, or
`proved` when no other (c) id is open. This section changes no rung.

### 13.3 The model-theorem category and its link status (PIF-020, owner question Q8)

A claim file may check a theorem about a **model** (a definitions-only PIF file reviewed and
signed under `<pkg>.model.faithful`), not about code. Section 4's `checked` rung needs rung 1
(`tested`) as a floor, and a model theorem has no tests, so it cannot be a rung (VER32 review
finding 14). It is a **separate category**, computed by the gate and printed in the release
notes in its own table beside the claims table:

- **It is never a package rung.** It does not enter the ladder, never changes a claim tier
  or a release tier, and is never matched to a package claim by name (VER33 1.4). A package
  claim rises above `tested` only through its own signed goal.
- **Its word is `model-checked`.** The gate never prints `proved`, `checked` or `verified`
  for it. The word is followed by its link status.
- **It is `model-checked` only when** `pif-check` accepted the claim file under a valid
  sign-off (13.1), the theorem's type equals the signed goal, and its axiom footprint is empty
  or lists only registered axioms (VER32 3.9); otherwise it is absent from the table, not a
  weaker entry.

The **link status** of each model theorem is computed from the sign-off, the report and the
goal, never typed:

| Field | Values | How it is computed |
|---|---|---|
| `link` | `model-only` or `code-linked` | `code-linked` only if a link theorem over the code semantics is checked in the same signed environment and `<pkg>.model.code-link` is closed; otherwise `model-only`. At G-M1 always `model-only`. |
| `model_origin` | `hand-written` or `generated` | `generated` when the model file was elaborated from a Warrant document by the elaborator (the document's path and digest are printed); `hand-written` otherwise. |
| `domain` | the quantified type(s), for example `Int` | **The domain actually quantified**: the gate reads the binder types of the goal's display CTE. It is printed beside the lane parameter type of the package it models (`u8`, `number`), so a theorem over `Int` is never read as one about `U8` (VER33 1.3, 1.5). |
| `open` | the (c) and (b) ids it still carries | The tool-trust ids of 13.2, the (b) ids the sign-off does not close, and, while `model-only`, `<pkg>.model.code-link`. Printed in full, as in section 8. |

A model theorem is thus **relative to** the model-to-code ids and says so. For G-M1 (logic
gates) the entry reads: `model-checked, model-only, generated, domain Int, open:
kernel.checker.correct, kernel.logic.consistent, pif.reader.correct, tcb.pif.gate-binding,
tcb.pif.printer, tcb.report.checker-correct, gates.model.code-link` (VER32 9.5), with the
package claims' rungs unchanged.

### 13.4 The cap on lane packages, section 6.2 (ADL-006b, VER33 finding 12)

Section 6.2 caps a lane package at `tested`. That cap is **the default and stays**: a lane
package whose lane row is `live` has rung `tested` for every claim. VER33 review finding 12
showed that it also makes `proved` unreachable for every lane, with no honest route to
lift it, and that VER33's earlier "A3 complete, signed gives `proved`" row was not
reachable. The amendment lifts the cap **per claim, per lane, by computation**, never by
declaration:

1. A lane package's claim is eligible for a rung above `tested` only through **its own signed
   goal** for that lane's code (VER33 1.4, code shape: A3-WIR or A3-source), with the
   `Equiv` statement checked for every `op` the claim mentions. A model theorem (13.3) never
   lifts it. A claim with no such goal stays `tested`.
2. An eligible claim's rung is then computed by the **ordinary ladder** of section 4 from its
   open ids, so the open ids do the capping. With A3-WIR, `wir.lowering.faithful` and the other
   (c) ids stay open, so the result is `checked` at most. With A3-source it is `checked` at
   most until all of these hold: the owner has decided the bucket of
   `src.<lane>.lean-semantics.faithful` for that lane (VER11a Q3, decided once per lane),
   `kernel.checker.correct` is closed for the claim (independent agreement, KRN-019), and no
   other (c) id is open.
3. **`proved` is therefore not reachable for any lane until VER11a Q3 is decided for that
   lane.** This amendment decides no Q3 and moves no bucket.
4. A lane whose row is `planned` or `absent` still refuses, as in 6.2. The parent report
   still ships verbatim; how a lane report carries the lifted goal is ADL-006c's and
   ADL-033's work and is not specified here.

### 13.5 The gate computes the A-rung (ADL-006b, ADL-006c1 to c3)

VER33 1 defines an adoption ladder A0 to A4. The gate **computes** each claim's A-rung and
**never takes it as declared**: a report carrying an `a_rung` field is refused. The A-rung is
printed **beside** the VER31 rung (as the generator label already is, section 4), because the
two say different things.

| A-rung | The gate computes it when | Rendered as |
|---|---|---|
| A0 | the claim is at least `tested` and its evidence derives from a Warrant file (H1) | `A0` |
| A1 | A0, and the evidence includes solver-chosen vectors (executed tests), or a `bounded(N)` rung with a checked certificate on the lane whose WIR was symbolically executed | `A1` (the solver-attested kind never raises it, 13.6) |
| A2 | the claim's gate-transported goal (VER33 1.4, embedded-domain shape) has a `model-checked` model theorem (13.3) | `A2 model-checked (<link status>)` |
| A3-WIR, A3-source | the claim has its own signed goal over the code and `Equiv` for every `op` (13.4 point 1) | `A3-WIR` or `A3-source`, with the VER31 rung beside it |
| A4 | the claim's evidence is generated code that has not passed the reverse check of VER33 8.7 | `A4 (tested)` |

Rules:

- **A2 is never rendered as a package-level word containing "proved".** An A-rung cell for
  A0, A1, A2 and A4 never contains the substring `proved`, and an A2 cell does not read as a
  claim about shipped code: it is a statement about the model, with its link status. This is a
  gate self-test (ADL-006c3).
- The **link status** of 13.3, including the domain actually quantified, is printed with every
  A2 and above (ADL-006c2); the whole-unit marker is added for A3-source only.
- **Fail closed.** An A-rung the gate cannot compute from the report, the sign-off and the
  goals, or an unknown rung word, is a refusal, not a guess.
- A higher A-rung never invalidates a lower one, and a lifter refusal keeps the lower rung
  (VER33 1.1); it never blocks a release (section 5).

### 13.6 The `solver-attested` evidence kind: cited, not copied (ADL-006b)

The backlog row for ADL-006b says it **cites** VER34 CVM-170b, which **owns** the amendment
text; this section therefore does not copy that text, so two copies cannot drift. The summary
below is for the reader's convenience and is **not normative**: on any difference the text
CVM-170b lands controls.

- The evidence kind `solver-attested` (solver, version, query digest; VER34 6.6) records an
  unsatisfiable answer with no checked certificate. The gate **never** accepts it as
  `proof.certificate` and **never** lets it raise a claim above `tested`; it may lower
  confidence by refuting (a checked model).
- For the DRAT route, a certificate **record** `(VC digest, k, CNF digest, DRAT digest)`
  binds the verification condition and the bound (VER34 6.7).
- CVM-170b's clause "`pif:sha256:` accepted" is to read "as defined in 13.2 of this
  document", so that the scheme's fields have one owner. That is a request to CVM-170b, not an
  edit made here.

### 13.7 The ids G-M1 opens

G-M1 (VER32 9.2) opens ids that have no catalogue entry or bucket today, and an
unbucketed id fails the gate (section 3). The registration is **proposed as data** in
`code/assumptions/catalogue-v2.proposed.json` and `buckets-v2.proposed.json`, with the diff
summary and the signing instructions in `code/assumptions/PROPOSED-gm1-registration.md`
and the VER13 side in [VER13](VER13-assumption-catalogue.md) section 10. **The table below is
the human rendering of the proposal and is deliberately not in section 3.5's row shape**,
because section 3.5 is the rendering of the *signed, current* bucket file and the AB-2 check
compares the two. It moves into 3.5 when the owner signs revision 2.

| Id | Proposed bucket | Closed by |
|---|---|---|
| `kernel.checker.correct` | (c) | independent agreement, per claim (VER11a Q6; KRN-019, PIF-010) |
| `kernel.logic.consistent` | (c), open | owner decision on VER11a Q5 (proposed (a)); cannot be discharged by evidence |
| `spec.goal.faithful` | (b) | signature on `goals-sha256` (13.1) |
| `pif.reader.correct` | (c) | independent agreement of a second reader (VER32 9.2) |
| `pif.prelude.faithful` | (b) | signature on the PIF prelude digest (13.1) |
| `tcb.pif.gate-binding` | (c) | falsification tests and fuzzing of the binding (PIF-003) |
| `tcb.pif.printer` | (c) | both round trips checked by the gate (VER32 9.2) |
| `gates.model.faithful` | (b) | signature on the model file digest and goals list (13.1) |
| `gates.model.code-link` | (c) | G-M2: replaced by the `wir.*` ids (VER32 9.2) |
| `warrant.model.elaboration-faithful` | (c) | mechanical evidence only; never a signature (VER33 Q6) |
| `adl.prelude.faithful` | (b) | signature on each ADL layer digest (13.1) |
| `adl.wir-abi.faithful` | (b) | signature on the encoding file digest; opened at A3, registered now (PIF-016a) |
| `tcb.vcgen.claim-faithful` | (c) | retiring the DRAT route or a kernel-reconstructed proof; opened at A1, registered now (ADL-006a) |

With these, the bucket file has 97 ids: (a) 35 (unchanged), (b) 10 (five new), (c) 52
(eight new). `logic_gates.model.faithful`, which VER33's scheme `<pkg>.model.faithful` could
instantiate, is **not** registered: VER32 9.2 pins the spelling `gates.model.faithful`.
The ids VER32 and VER33 list for later milestones (`gates.nand.primitive-contract`, the
`wir.*` ids G-M2 relies on, `gates.compose.*`, `wir.semantics.lean-faithful`,
`src.<lane>.*`, `<lane>.backend.code-link`, `tcb.<lane>-toolchain.trusted`) are PIF-016b,
PIF-016c and the per-lane rows, not G-M1's, and are not registered here.

### 13.8 What the owner signs, and what is open

Signing is two separate acts on two different files, and neither is done by this change.

1. **This section** (13.1 to 13.7), by approving the pull request that carries it, as for
   any VER31 amendment.
2. **The bucket file, revision 2**, once the catalogue has the 13 entries: the exact bytes of
   `code/assumptions/buckets-v1.json` after the proposal is applied (revision 2, `decided`
   naming the owner), signed with `ssh-keygen -Y sign -n coding-adventures-buckets`, as
   `buckets-v1.json.sig` (section 3, section 9.6). The signature covers every id's bucket and
   floor flag, and the revision. The signature is over the *applied* bytes, not over the
   `.proposed.json` file, whose `decided.by` is a marker. See
   `code/assumptions/PROPOSED-gm1-registration.md` for the steps.

Open items this section does not close:

- The catalogue's family list (`spec src wir cc mc uarch gate phys tcb os gov att`) does not
  contain `kernel`, `pif`, `gates`, `warrant` or `adl`, so the 13 entries cannot be added
  until the checker (`assumption_catalogue.py`), its Rust twin
  (`ca-assurance-report/src/catalogue.rs`) and the JSON schema agree on them. That is a small
  data and code change owned by PIF-016a, listed in the backlog, and **not made here**.
- The location of the list of withdrawn kernel versions (13.2).
- Whether `pif-signoff 1` should carry a signer and a date (13.1 finds the principal by
  `find-principals`; VER32 3.9 fixes the format as complete).
- The gate code for 13.1 to 13.5 (PIF-020's gate half, ADL-006c1 to c3).

### Follow-up 2026-11-25 (G-M1 registration; VER31's review-record equivalent)

| # | Change | Source | Where |
|---|---|---|---|
| W1 | **PIF-020 written** as proposed text: the `pif-signoff 1` file's gate rules, the goal-signing aggregate with a worked example, what the signature covers (the whole file, the base environment, not the proof file), the `pif:sha256:` certificate kind with kernel version and commit and both environment digests, the model-theorem category with link status and domain quantified | VER32 3.9, 8.3, Q5, Q8; VER33 1.5 | 13.1 to 13.3 |
| W2 | **ADL-006b written** as proposed text: the section 6.2 cap lifted per claim by computation, the A-rung computation with "A2 is never rendered as proved", and `solver-attested` **cited** from CVM-170b | VER33 finding 12, 1.5, 9.2, 13; VER34 6.6, 6.7 | 13.4 to 13.6 |
| W3 | **Registration** of the 13 ids G-M1 opens, as proposed data, no bucket moved | VER32 9.2, 11; VER33 9.1 | 13.7 |
| W4 | **Resolved: cite, do not include** the CVM-170b text, as the ADL-006b row says; the one overlap (`pif:sha256:` accepted) has a single owner (13.2) | backlog ADL-006b, CVM-170b | 13.6 |
| W5 | **Not applied**: the rung table of section 4, section 3.5, the bucket file and the gate; all wait for the owner's signature | VER31 3.4, 9.6 | none |

**Questions for the owner.** (1) Approve 13.1 to 13.6. (2) Approve reusing the namespace
`coding-adventures-statement` for `pif-signoff 1` (the alternative is a new namespace and a
signers-chain revision). (3) Confirm `kernel.logic.consistent` is registered as (c), open, with
VER11a Q5 left for a separate decision. (4) Confirm `gates.model.faithful`, not
`logic_gates.model.faithful`. (5) Approve extending the catalogue's family list.
