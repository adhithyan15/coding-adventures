# VER31 — Tiered Release Standard: Publish What the Evidence Computes

**Status:** Draft for owner approval. It contains no code. The gate it specifies is
backlog row LV-007a, and the first release candidate (`ct-compare`) is LV-007b.

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
every computed tier. So the gate does not trust it by convention. Its SHA-256 must
equal the digest in a bucket record signed with the owner's hardware key
(`buckets-v1.json.sig`, section 9.6), and it is code-owned by the owner. The
meaning of each bucket is fixed in the gate program ((a) never blocks `proved`;
(b) and (c) block it). A bucket file whose `blocks_proved` values say otherwise is
refused, so the data cannot redefine a bucket.

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
- `warrant_sha256` equals that file's digest at the tagged commit **and** the
  Warrant digest that the package's evidence manifests bind;
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
| 5 | `proved, baseline discharged` | rung 4, and every (a) id that is not a floor (catalogue `status: floor` or `kind: declared`) is in `proof.discharged` | As rung 4, with the platform layers discharged too (VER29). Floors (physics, declared caller context) can never be discharged and are listed. |

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
| F4 | A recorded security review covering the released source | `security-review.json` in the package (schema `ver31-security-review-v1`): `reviewed_commit`, `scope` (paths), `reviewer`, `date`, `findings` (each `fixed`, `accepted` or `not-applicable`), `outcome`. The gate requires `outcome: pass` and no open finding of severity `high` or `critical`. **The gate computes the scope the review must cover; it does not trust the record's.** The required set is every file in the built artifact (`cargo package --list`; `npm pack --dry-run --json`), plus the package's manifest, build script and lockfile, plus repository-level build configuration (the workspace `Cargo.toml`, `.cargo/`, `rust-toolchain*`). `scope` must cover that set. Each entry must be a normalized repository-relative path with no `..`, no glob and no leading `:`, and must not be a symlink. `reviewed_commit` must be on `main` and an ancestor of the tagged commit. Freshness is `git diff --quiet --literal-pathspecs <reviewed_commit> <tagged commit> -- <scope…>`. The record is typed; its coverage and freshness are computed. |
| F5 | Fuzz and property tests on every untrusted-input surface | Every claim of the report has `fuzz.status` and `property.status` equal to `pass`. A lane package (section 6.2) needs its own property and fuzz suites over each public operation (VER05a, VER06b engines), run by F1. |
| F6 | Recorded benchmarks | `evidence/bench-manifest.json` exists, parses as `warrant-bench-evidence-v1`, has at least one `pass` entry, and its `source_sha256` matches the package source. |
| F7 | README and CHANGELOG | `README.md`; `CHANGELOG.md` or `CHANGELOG.d/_meta.md`; the changelog names the version being released. |
| F8 | Zero third-party runtime dependencies | Cargo `[dependencies]`, `[build-dependencies]` and every `[target.*.dependencies]` are empty; npm `dependencies`, `peerDependencies`, `optionalDependencies` and `bundleDependencies` are empty. Development dependencies are allowed because they do not ship. |
| F9 | Licence | Manifest licence `MIT` and a `LICENSE` file inside the package. |
| F10 | The generated report ships verbatim and is about this code | `ca-assurance-report --check` passes at the tagged commit (and `--check-lane` for a lane package). The report is fresh: its `commit` is on `main`, it is an ancestor of the tagged commit, and nothing changed between them in the package source, its `.warrant` or its evidence manifests (`git diff --quiet --literal-pathspecs`). Otherwise the gate refuses and asks for regeneration. The built artifact contains `assurance_report.json` byte-identical to the checked file, and a generated `ASSURANCE.md` (section 8), even when the verdict is `report-only`. |
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
| H3 | Live runtime-jail evidence | A strict, live VER08b run (`RCG_STRICT=1`, Linux) of the package at the tagged commit, which the gate re-runs. NOT-LIVE fails. Needs CAP-RT-002 and CAP-RT-003. |
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
floor check itself, and it ships the parent report verbatim.

---

## 7. The gate

The gate is a step of `.github/workflows/publish.yml` (LV-007a) that runs before
every publish job of every language, and in a dry-run mode on demand.

- **Inputs:** the tagged commit; the package directory from the tag; the
  package's report (or the parent report for a lane package); `buckets-v1.json`;
  `catalogue-v1.json`; an optional statement sign-off; `.github/release-signers`
  read from `main`.
- **Program:** `code/scripts/release_gate.py`, standard library only, with
  falsification tests under `code/scripts/tests/`. It has three subcommands:
  `tier` (rungs, claim tier, release tier, bucketed assumptions), `floor` (the
  static F checks) and `notes` (release notes and `ASSURANCE.md`). Its output
  `gate.json` is uploaded with the artifact.
- **The tool is in bucket (c).** The gate program is ours and unverified, like
  `ca-assurance-report` (`tcb.report.checker-correct`). Its tests are the
  evidence; the release notes say so.
- **Privilege split.** GitHub grants permissions per job, not per step, so the
  gate is two jobs.
  - `gate` has `permissions: contents: read` only. It runs every test, lint and
    coverage step, which is where third-party development dependencies execute.
    It builds the artifacts, writes their SHA-256 digests into `gate.json`, and
    uploads both.
  - `attest` needs `gate`. It checks out no package source and installs nothing.
    It downloads the artifacts, refuses unless their digests equal `gate.json`, and
    is the only job with `id-token: write` and `attestations: write` outside the
    `release` environment.

  No job that runs package, test or dependency code holds `id-token: write`
  outside the `release` environment.
- **Publish uses the gated bytes.** npm publishes the gate's tarball. crates.io
  cannot upload a prebuilt file, so its publish job runs `cargo package
  --no-verify`, refuses unless the `.crate` digest equals `gate.json`, runs `cargo
  publish --no-verify` (verification already ran in `gate`), then downloads the
  published crate from `static.crates.io` and compares the digest. A mismatch fails
  the job, and the owner yanks the version.
- **Every job checks out `${{ github.sha }}`**, never the tag name, so a tag moved
  mid-run cannot change what is built.
- **Dry run:** `workflow_dispatch` with a package path and a ref runs `gate`
  only: no `attest` job, no publish job, no tag-signature check, so a dry run of
  unmerged code never produces a genuine attestation. Its `package` and `ref`
  inputs reach the shell only through `env:`, and must match the same strict
  pattern as release tags. The owner never approves a `release` deployment for a
  `workflow_dispatch` run.
- **Every language:** the Python and Ruby publish jobs also depend on the gate.
  They refuse until those packages have reports, as P13's hold did; LV-007c moves
  RubyGems to trusted publishing.

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
`namespaces="git,coding-adventures-statement,coding-adventures-buckets"` and the
`verify-required` option), added by the owner in a code-owned PR (section 9.6).

The gate's `verify-tag` job:

1. reads `.github/release-signers` from `refs/remotes/origin/main`, never from the
   tagged tree, so a tag cannot bring its own signer;
2. refuses an empty file, any key line whose type is not a security-key type, and
   any key line without `verify-required`;
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

Auto-merge must not move them. The controls:

1. **Code owners:** `.github/CODEOWNERS` lists them with the owner. The `main`
   ruleset requires code-owner review, so a PR that touches one waits for the
   owner even when auto-merge is set. The repository has no CODEOWNERS file today;
   LV-007a adds it.
2. **Signed bucket record:** the gate refuses a `buckets-v1.json` whose SHA-256
   differs from the digest in `buckets-v1.json.sig`. That record is signed by the
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

## 12. Open items

- LV-007d: the report generator should emit `exhaustive.domain` so that rung 2's
  `exhaustive` becomes reachable, and could emit buckets directly once this spec
  settles.
- The statement sign-off schema (section 3.2) and the security-review schema
  (F4) are specified here and implemented with the gate (LV-007a).
- Whether `proved-per-run` deserves its own rung is left for when a package
  produces one.
