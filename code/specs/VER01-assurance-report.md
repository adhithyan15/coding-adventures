# VER01 — Ship Gate, Assurance Report, and Verdict Evaluation

**Parent:** [VER00 §9](VER00-verified-shipping-program.md), [VER15](VER15-weakness-registry.md), [VER26 §3, §5 Step 28](VER26-ct-compare-pilot.md).

This specification formalizes **VER01**: the machine-readable assurance report format (`assurance_report.json`), the verification ladder evaluation rules, and the fail-closed publication verdict determination.

---

> **Update (VER04h):** claim numbers, labels, certificates and evidence ids in this document's examples are *computed from evidence manifests* and cross-checked by `--check`; see [VER04h](VER04h-warrant-derived-evidence.md) section 9. Memory-safety rows are `excluded` only when the effect certificate declares no `unsafe_code` exception and the package source contains no `unsafe`. Each claim's `proof` also carries `certificate_kind` (`"trivial-miter"` when a backing certificate is a collapsed miter, else `"miter"`), `--check` fails closed on duplicate claim ids, unranked proof labels, failing exhaustive status and (under a `pass` verdict) unchecked assumptions, and rejects `unverified::` evidence ids unless `--allow-unverified` is given.

## 1. Objectives and Ladder Integration

```text
  ┌─────────────────────────────────────────────────────────────┐
  │ Gate: VER01    Machine-Readable Assurance Report & Verdict  │ ◄── THIS SPEC
  ├─────────────────────────────────────────────────────────────┤
  │ Matrix: VER15  Weakness Registry v1 & Closure Rules         │
  ├─────────────────────────────────────────────────────────────┤
  │ L4: VER-P027b  Generated Effect Module & Proof Certificate  │
  ├─────────────────────────────────────────────────────────────┤
  │ L4: VER-P027a  Rust Capability Analyzer & Name Allowlist    │
  ├─────────────────────────────────────────────────────────────┤
  │ L3: VER20      Bounds & Constant-Time Trace Proofs (UNSAT)  │
  ├─────────────────────────────────────────────────────────────┤
  │ L2: VER06a     Coverage-Guided Fuzz Engine & Targets        │
  ├─────────────────────────────────────────────────────────────┤
  │ L5: VER07a     Equal-Instruction-Count Microbenchmarks       │
  ├─────────────────────────────────────────────────────────────┤
  │ L0: TF01       Conformance Corpus & Sabotage Runner         │
  └─────────────────────────────────────────────────────────────┘
```

The publication gate (VER00 §9) mandates that no software package may be published to an external or internal distribution registry without an audited, machine-readable **assurance report**. The report integrates all rungs of evidence into a cryptographic, tamper-evident manifest.

---

## 2. Assurance Report Schema

The canonical report is emitted as `<package_root>/assurance_report.json` with the following schema:

```json
{
  "package": "rust/ct-compare",
  "version": "0.1.0",
  "commit": "a1a722f32c500000000000000000000000000000",
  "closure": ["rust/ct-compare"],
  "capabilities": {
    "declared": [],
    "analyzer": "pass",
    "certificate": "proved",
    "new_since_last_release": []
  },
  "weaknesses": [
    {
      "cwe": 416,
      "name": "Use After Free",
      "status": "excluded",
      "reason": "safe Rust: analyzer confirmed no unsafe or ffi reachable"
    },
    {
      "cwe": 208,
      "name": "Observable Timing Discrepancy",
      "status": "exemption",
      "approved": false,
      "reason": "proof floor requires gate-level CPU netlist; backed by WIR trace proof and equal instruction counts"
    }
  ],
  "claims": [
    {
      "id": "ct_eq_soundness",
      "symbol": "ct_eq",
      "example": "pass",
      "property": { "cases": 4096, "status": "pass" },
      "exhaustive": { "cases": 120409, "status": "pass" },
      "fuzz": { "iterations": 5000, "status": "pass" },
      "proof": {
        "label": "bounded(128)",
        "assumptions": ["compiler.preserves_timing", "memory.unaliased"],
        "certificate": "drat:sha256:<digest of the sorted certificate ids>",
        "certificate_kind": "trivial-miter | miter"
      },
      "evidence": ["warrant/ct_compare/exhaustive/eq_agrees", "warrant/ct_compare/proof/eq/constant-time"]
    }
  ],
  "rungs": {
    "E0": "pass",
    "E1": "pass",
    "E2": "not-live",
    "E3": "pass",
    "E4": "pass",
    "E5": "pass"
  },
  "public_symbols_without_claims": [],
  "exemptions": [
    {
      "id": "EXEMPT-CWE-208-GATE-LEVEL",
      "kind": "weakness-floor",
      "cwe": 208,
      "approved": false,
      "rationale": "Hardware gate-level CPU timing proof (Busicom track) is not yet live. Backed by Level 3 WIR-level 2-safety self-composition trace proof and Level 5 equal-instruction-count microbenchmarks.",
      "evidence": [
        "warrant-ir::test_ct_proofs",
        "ct-compare::tests::bench_instruction_counts"
      ]
    }
  ],
  "verdict": "report-only"
}
```

---

## 3. Evaluation and Gate Invariants

### 3.1 Rung Assessment Rules
1. **E0 (Capabilities):**
   Requires capability analyzer pass and effect certificate verification (`effects(P) ⊆ declared(P)`).
2. **E1 (Unit / Conformance Corpus):**
   Requires all canonical corpus cases (`cases.json` / `cases.tfv`) to pass via the L0 bootstrap harness.
3. **E2 (Integration & Simulated Host):**
   If the simulated host or fault-injection harness is not yet live for this package family, E2 **must be reported as `"not-live"`**, never `"pass"`.
4. **E3 (Generative Property Testing):**
   Requires all `claim` properties to execute with isolated PRNG seeds and meet test budgets.
5. **E4 (Coverage-Guided Fuzzing):**
   Requires fuzz suites to execute across target mutations with crash deduplication.
6. **E5 (Formal Proofs):**
   Requires formal WIR-level or SAT/DRAT verified proofs for all target claims.

### 3.2 Symbol Claim Coverage
The set of public symbols exported by the crate is extracted by the analyzer. Every exported symbol must be covered by at least one formal claim.
$$\text{public\_symbols\_without\_claims} = \emptyset$$
Any uncovered exported symbol causes an immediate gate rejection.

### 3.3 Computed Exemptions
Exemptions are **never declared by package authors**:
- If a rung or weakness class cannot meet its strict floor, an exemption is mechanically derived.
- Every exemption is initialized with `approved: false`.
- Approvals require physical cryptographic token signatures per VER00 §9.1 and §9.5.
- Because VER00 §12.1 blocks the publishing/approval pipeline until trust-root findings are settled, **all exemptions remain unapproved**.

### 3.4 Verdict Determination
The final `verdict` is computed as follows:
- **`pass` (Publishable):**
  All rungs E0–E5 are `"pass"`, all weakness rows are `"excluded"` or `"discharged"`, all exemptions are `approved: true`, and `public_symbols_without_claims` is empty.
- **`report-only`:**
  The package is valid, honest, and completely audited for testing/research, but cannot be published to a distribution registry because:
  - At least one rung is `"not-live"` (e.g. E2 simulated host); OR
  - At least one computed exemption is unapproved (`approved: false`); OR
  - The package is explicitly evaluated in `--report-only` mode.
- **`fail`:**
  A verification failure occurred (e.g. capability violation CAP001/CAP002, failing test, tampered certificate, unmapped weakness class, or public symbol without claim).

---

## 4. Pilot Application: `rust/ct-compare`

For the `rust/ct-compare` pilot crate:
- **E0:** `pass` (`effects = ∅`, effect certificate proved).
- **E1:** `pass` (all 12 CT01 test cases pass L0 bootstrap).
- **E2:** `not-live` (simulated host integration not applicable for pure math / not yet live).
- **E3:** `pass` (PBT single-bit sweeps, agreement with `==`, fresh copy).
- **E4:** `pass` (coverage-guided fuzz suites pass with zero crashes).
- **E5:** `pass` (WIR trace proofs for `ct_eq`, `ct_eq_u64`, `ct_select_bytes`).
- **Weakness Matrix:**
  - 24 classes `excluded` (safe Rust with no unsafe, no FFI, no ambient I/O).
  - 1 class (CWE-400) `discharged` (zero allocation, bounded linear steps).
  - 1 class (CWE-208) as `exemption` (`approved: false`, backed by WIR trace proofs and equal instruction counts).
- **Verdict:** `"report-only"`.
