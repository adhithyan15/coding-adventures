# VER04h — Warrant-Derived Evidence (VER-P030)

**Parent:** [VER00 §7–§9](VER00-verified-shipping-program.md), [VER01](VER01-assurance-report.md), [VER04 §5–§8](VER04-warrant-language.md), [VER04e](VER04e-warrant-codegen.md), [VER04f](VER04f-claims-to-properties.md), [VER05](VER05-property-engine.md), [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06a](VER06a-ct-compare-fuzz-targets.md), [VER15](VER15-weakness-registry.md), [VER19](VER19-symbolic-execution.md), [VER20](VER20-resource-timing-proofs.md), [VER26](VER26-ct-compare-pilot.md).

## 0. Why this exists

VER04 promises that a Warrant document is *the* specification of a package and
that tests, properties, fuzz targets and proofs are projections of it. An audit
of the pilot (see the lesson "A report that hardcodes its own evidence passes an
assurance gate that only checks shape") found the promise was not kept:

* `generate_ct_compare_report()` contained literal case counts, iteration
  counts, certificate hashes and `PackageAuditContext` flags;
* `--check` validated shape only, so any literal passed;
* the claim "property test" compared `a == b` with `a == b` and never called the
  crate; the emitted PBT harness was only string-matched, never compiled;
* random 0..32-byte pairs almost never take the equal branch;
* `ct_eq` was proved only through its equal-length helper at 4 bytes while the
  report said `proved`; `ct_select_bytes` at 4 bytes while the report said
  `bounded(4096)`;
* the Warrant document had one claim for four operations.

This specification states the rule that replaces all of that:

> **Evidence is never written by hand. Every case count, seed, iteration count,
> proof label and certificate hash in an assurance report is read from a
> manifest produced by the run that generated it, and the checker rejects any
> report that disagrees with the manifests or with the Warrant document.**

Nothing here proves more than before; the aim is that what is *claimed* equals
what was *run*. Where the tooling cannot reach a claim the label says so.

## 1. Pipeline

```text
code/specs/<pkg>.warrant ──► parser ► checker ► codegen (this spec)
        │                                           │
        │                    ┌──────────────────────┼─────────────────────────┐
        │                    ▼                      ▼                         ▼
        │        tests/generated/*.rs      proof obligations (driver)   (TFV: VER04e)
        │        examples+exhaustive+PBT   labels ► 2-safety            examples ► cases.tfv
        │        +fuzz (harness=false)     bound  ► BoundProver
        │                    │                      │
        │                    ▼                      ▼
        │       evidence/test-manifest.json   evidence/proof-manifest.json
        │                                     evidence/proofs/<sha256>.{cnf,drat}
        └─────────────────────┬───────────────────────┘
                              ▼
              ca-assurance-report: generate (reads manifests) / --check (cross-checks)
```

The generator is a pure function of `(checked Warrant module, validated
bindings, EvidenceOptions)`; identical inputs give byte-identical output. A
regeneration check (`warrant-grammar --example gen_evidence -- --check`) fails
when a checked-in generated file differs from what the current Warrant document
and generator produce. The same command is in the package `BUILD` and
`BUILD_windows`.

## 2. Construct-to-evidence mapping

| Warrant construct | Evidence generated | Executed against | Recorded as |
|---|---|---|---|
| `example` | L0 case: operation called with the literal arguments, result deep-compared with the evaluated expected value | real public export | `example/<name>` |
| `claim forall …` | (a) bounded-exhaustive suite; (b) L2 property suite; (c) fuzz suite | real public export, oracle = the claim body evaluated by generated model code | `exhaustive/<claim>`, `property/<claim>`, `fuzz/<claim>` |
| every suite above | negative control: the suite is re-run with a deliberate fault injected at the adapter and must fail with a registered failure id | same adapter | `negctl/<suite>/<claim>/<fault>` |
| `labels <op>` with `secret` rows | 2-safety (constant-time) proof obligation on the real lowered source | `src/lib.rs` lowered by `warrant-ir` | `proof/<op>/constant-time` |
| `bound <op>` `steps <= f(n)` | `BoundProver` obligation per concrete size | same lowered source | `proof/<op>/steps-bound` |
| `bound <op>` `heap`/`depth` | not discharged by the pilot prover; recorded `unchecked` | n/a | `proof/<op>/heap-bound`, `…/depth-bound` |
| `input … rejects … with E` | covered by claims of the form `op(...) == Err(E)`; a `rejects` row with no such claim is reported as uncovered | real export | via claim |
| `fresh` op | adapter checks the output buffer does not alias an input and that inputs are unchanged after the caller mutates the output | real export | folded into every claim on the op |

### 2.1 What the generated model code is

Claim bodies are not interpreted at run time. The generator emits Rust for the
checked expression over a small dynamic value type (`V`: `Bool`, `Nat`, `U64`,
`Bytes`, `Ok`, `Err`, `Name`). `model` functions become Rust functions over `V`;
`op` calls become calls into per-operation *adapters* that call the real export
and normalize its result (a panic from `ct_select_bytes` becomes
`Err(CT_SELECT_LENGTH_MISMATCH)` through `catch_unwind`, as declared in the
binding). Supported expression forms: literals, binder and model references,
operation calls (with literal widths), `Ok`/`Err`/`Name.Variant`, `len`,
`== != < > <= >=`, `+ - *`, `and or not ==>`, `if … then … else`. Anything else
fails generation with `W0701` rather than being silently skipped.

Binder refinements (`b: Bytes | len(a) == len(b)`) are admission predicates
generated the same way; inadmissible tuples are discarded by the exhaustive
suite and rejected by generators, and the suites record how many were
discarded so a refinement that excludes everything cannot pass vacuously.

## 3. Bounded-exhaustive suites

"Exhaustive" is only ever *relative to a stated finite domain*. The domain is
generated per binder type (all rules deterministic) and the suite enumerates the
Cartesian product of the binder domains, filtered by refinements.

| Binder type | Domain `D(T)` |
|---|---|
| `Bool` | `{false, true}` |
| `U8` | all 256 values |
| `U64` | all of `0..=255`; every single-bit value `1<<i` for `i >= 8`; `MAX`; every `MAX ^ (1<<i)` (377 values) |
| `Nat` | `0..=64` |
| `Bytes` | (i) every string of length 0 or 1 over all 256 bytes; (ii) every string of length 2..=3 over the alphabet `A = {00, 01, 80, ff}`; (iii) for each length 4..=8 the fills `00…` and `a5…` (347 strings) |
| `Bytes<N>`, `N <= 3` | the members of `D(Bytes)` of length `N` (for `N = 1` this is all 256 bytes) |
| `Bytes<N>`, `N >= 4` | uniform fills over `A`, plus every string that is `00…` with one position set to a symbol of `A` (`4 + 3N` strings) |

Case-count budget: the product over binders (before refinement) must not exceed
`EXHAUSTIVE_BUDGET = 400_000` per claim; the generator fails with `W0706` if a
claim would exceed it, forcing a smaller domain rather than a silent sample.

The budget is computed with **checked** arithmetic and also bounds memory:

* a product (or sum) that overflows `u64` is over budget, never zero -- eight
  `U8` binders are `2^64` cases, which wrapped to `0` and used to pass;
* `DOMAIN_BYTES_BUDGET = 64 MiB` bounds the total bytes the binder domains
  materialise (each element costs `VALUE_OVERHEAD = 32` bytes plus its payload),
  because the element count alone lets `Bytes<100000>` (about 300k elements of
  100 kB each) through;
* a fixed width above `MAX_FIXED_WIDTH = 4096`, as a `Bytes<N>` binder or as an
  `op<N>` call width (each becomes a const-generic match arm), fails
  generation with `W0707`.

The generated prelude repeats the same checks (`within_budget`) and runs them
*before* materialising any domain.
Both-sides length coverage is deliberate: every pair of lengths in `0..=8`
occurs for `Bytes` pairs, so each mismatch path is exercised.

The honest statement an exhaustive suite supports is
`exhaustive over <domain description>, <n> cases, <m> discarded`, never
"all inputs". The manifest records `domain` (text) and the counts.

## 4. Property suites (L2) and the equal-branch problem

Uniform random pairs make the equal branch (and the single-position-difference
branch) vanishingly rare, so generators are *relation biased*. For a claim with
two binders of the same type the generator draws the first binder, then chooses
the second by relation:

| Share | Relation of 2nd to 1st |
|---|---|
| 1/4 | independent |
| 1/4 | identical |
| 1/4 | one bit flipped (`Bytes`/`U64`) |
| 1/8 | one byte changed (`Bytes`) / one bit set (`U64`) |
| 1/8 | one byte appended or removed (`Bytes`) |

`Bytes<N>` binders draw exactly `N` bytes. A refinement is applied as a filter
with `max_retries = 100`; exhaustion fails with `property-exhausted`. Lengths
for `Bytes` are `0..=64`. Execution uses `pbt::check_property` with a per-claim
seed `SplitMix64(evidence_seed ^ fnv1a64("claim/" ‖ module ‖ "/" ‖ claim))`.
`PROPERTY_CASES = 4096` per claim. The manifest records the seed and the case
count actually run.

## 5. Fuzz suites

One `fuzz::fuzz_target` campaign per claim. The oracle is the claim body (same
generated model code as §3–§4). The fuzzer's byte stream is decoded
deterministically into binders: a mode byte selects the relation (independent,
identical, bit flip, length change) as in §4, so the coverage-guided mutator can
reach the equal branch from byte one; `U64` binders take 8 bytes each, `Bool`
one bit, `Bytes<N>` exactly `N` bytes (zero padded). A `Crash{kind:
"claim-violated"}` outcome (or a panic outside a declared error path) is
`fuzz-crash`. Coverage edges are the claim's observable branch classes (length
relation, matching-prefix length bucket, result). The manifest records
`iterations` actually executed (from `FuzzReport.iterations`, not the
configured maximum) and the seed.

## 6. Negative controls (sabotage)

A suite that cannot fail is not evidence. Every suite is registered together
with one `negctl` test per applicable *fault*. The fault is injected in the
adapter (the crate is never modified):

| Fault | Applies to | Behaviour |
|---|---|---|
| `invert` | any `Bool`-returning op | result negated |
| `ignore-last-byte` | `eq`, `eq_fixed` | the last byte of both inputs is zeroed before the call (models a scan that stops one byte early) |
| `ignore-high-bit` | `eq_u64` | bit 63 masked from both operands |
| `swap-choice` | `select_bytes` | `choice` negated |
| `missing-error` | `select_bytes` | length mismatch returns `Ok(a)` instead of an error |

Negative controls are per (suite kind, operation, fault): the test passes iff
at least one suite of that kind for that operation, run with the fault, *fails*
with a registered failure id (`property-counterexample`, `fuzz-crash`,
`not-deep-equal`), and the manifest records which claims caught it
(`caught_by`). A fault that no claim catches fails the control. The same
faults are selectable from the command line for BUILD-level checking:
`WARRANT_EVIDENCE_FAULT=<fault> cargo test … --test warrant_evidence` must print
`not ok` lines (and exit non-zero) for the affected suites. The BUILD files run
every fault in that mode.

`WARRANT_EVIDENCE_FILTER=<substring>` restricts a sabotage run to matching test
ids. It is only meaningful together with `WARRANT_EVIDENCE_FAULT`: set alone it
makes the target exit `2` with a message (it would otherwise run fewer tests
and exit `0`).  The fault must be a *real* one: `WARRANT_EVIDENCE_FAULT=none`
with a filter is refused the same way.  A filter that matches no test also
exits `2`, and a filtered or faulted run never writes a manifest.

## 7. Proof obligations from `labels` and `bound`

Driver: a test target in `warrant-ir` (the crate that owns the lowering and the
provers) that reads the Warrant document, lowers the *real* `src/lib.rs`, and
for each op:

1. **`labels … secret`** → `ConstantTimeProver` 2-safety obligation. Parameters
   labelled `public` (buffer *lengths*) are tied between the two runs; `secret`
   parameters (buffer contents, `choice`, `U64` operands) are independent
   symbolic inputs. Calls are inlined by a WIR inliner (the symbolic executor
   does not execute `CallFunction`) so the public export itself, not an internal
   helper, is the proved object.
2. **`bound … steps <= f(len(a))`** → `BoundProver` obligation evaluated at each
   concrete length with `f` evaluated by the Warrant model evaluator.

Obligations are discharged for every length *shape* up to `K`: all
`(len_a, len_b)` with `0 <= len_a, len_b <= K` for two-buffer operations
(equal shapes only for `eq_fixed`, whose widths are equal by type). The pilot
uses `K = 128` (about 22 s in CI; 16,641 shapes for `ct_eq`), recorded as
`max_len`. The honest label is therefore `bounded(K)` — **never `proved`** for
byte-slice operations — and `proved` only for operations with no loop and no
buffer, where the claim is over the full input domain (`eq_u64`).

`bounded(K)` means: every *non-trapping* shape with lengths `<= K`. A shape on
which the export's own `assert_eq!` on the public lengths fails
(`ct_select_bytes` with `len_a != len_b`) panics before reading any byte; the
symbolic executor reports a trap as an error, not a path outcome, so these
shapes are *counted, not proved* (`trap_shapes` in the manifest: 16,512 of
16,641 for `ct_select_bytes`) and are covered by the
`select_rejects_mismatch` claim's exhaustive/property/fuzz suites instead.

**Lowering fidelity fixes made for this.** `assert!`/`assert_eq!` lowered to
no-ops, which turned the mismatch path into an out-of-bounds scan; they now
lower to a branch into a trap (`debug_assert*` stays a no-op, as in release).
`vec![0; n]` stored a first element, which wrote past a zero-length buffer;
allocation is zero-filled so a zero fill emits no store.

### 7.1 What a certificate is

Each successful obligation records the CNF and DRAT proof text that the
`drat-check` kernel accepted, content-addressed as
`evidence/proofs/<id>.cnf` / `.drat` where `<id>` is the SHA-256 of
`cnf-bytes ++ 0x00 ++ drat-bytes`, so a file's name can be re-derived from its
contents. The *kind* is recorded:

* `trivial-miter-collapsed`: the divergence miter simplified structurally to
  `FALSE` (both runs produce identical observable traces by construction, which
  is the expected case for data-independent code), so the certificate is the
  fixed 2-clause refutation `x ∧ ¬x`. This is a valid refutation, but it
  certifies the *structural* equality of traces computed by the symbolic
  executor, not a non-trivial SAT refutation. Reports must not present it as
  more.
* `miter`: a Tseitin encoding of a non-trivial miter that CDCL refuted.

### 7.2 Negative controls for proofs

For each op the driver also lowers a *mutated* copy of the source with a
secret-dependent early exit in the scan loop (or a secret-indexed table read)
and requires the prover to return `Falsified`. This is recorded as
`proofneg/<op>/<mutation>`. `WARRANT_PROOF_FAULT=secret-branch` makes the main
obligations use the mutated source, which must fail; a too-small bound
(`bound-too-small`) must be `Exceeded`. Mutated runs never write a manifest.
**Observed:** every certificate in the pilot is `trivial-miter-collapsed`.

### 7.3 What stays out of scope

* `heap` and `depth` bound rows are recorded `unchecked`.
* A gate-level (hardware) timing proof is not live; CWE-208 stays an unapproved
  exemption.
* The WIR proof covers the lowered Rust subset; it does not cover LLVM output.
  The optimized-IR test remains separate, hand-written evidence.
* `ct_eq_fixed` proofs are over the const-generic function lowered once per
  tested width, not over all `N`.

## 8. Evidence manifests

Both manifests are deterministic (no timestamps, sorted keys and entries) and
written by the runs that produce the evidence.

`evidence/test-manifest.json` (`schema: "warrant-test-evidence-v1"`), written by
the generated test target when `WARRANT_EVIDENCE_OUT=<path>` is set:

```json
{ "schema": "warrant-test-evidence-v1", "module": "ct_compare",
  "spec": "code/specs/ct-compare.warrant", "spec_sha256": "<hex>",
  "evidence_seed": "<u64>",
  "tests": [ { "id": "warrant/ct_compare/exhaustive/eq_agrees",
               "kind": "exhaustive", "op": "eq", "claim": "eq_agrees",
               "cases": 119716, "discarded": 0, "domain": "…", "verdict": "pass" } ] }
```

`kind` ∈ `example | exhaustive | property | fuzz | negctl`. Property entries
add `seed` and `equal_cases` (generated cases that took the equal branch, so the
relation bias is measurable); fuzz entries add `iterations` and `seed`; negctl entries add `fault`
and `target` (the suite id attacked).

`evidence/proof-manifest.json` (`schema: "warrant-proof-evidence-v1"`) lists
obligations with `id`, `op`, `function`, `kind`
(`constant-time | steps-bound | negative-control`), `label`
(`proved | bounded(K) | unknown | falsified-as-expected | unchecked`),
`verdict`, `max_len`, per-shape `runs`, `certificates`
(`{id, kind, count}`; `kind` is `miter` or `trivial-miter-collapsed`) and, for
the obligations a run discharges, `obligation_sha256`: the SHA-256 of the
canonical text of the *lowered, inlined* WIR function (its deterministic derived
`Debug` rendering) followed by the textual obligation (kind, operation, secret
mask, length shapes, bounds).  The test and bench manifests also carry
`source_sha256`, the digest of **every** `src/**/*.rs` file -- the same
definition in all three manifests and in the checker.  The digested bytes are
the `warrant-src-v1` framing, implemented once in `test-bootstrap`
(`manifest::frame_sources` / `source_tree_framing`) so producers and checker
cannot drift:

```text
"warrant-src-v1\0"
u64-LE file count
per file, sorted by path bytes:
    u64-LE path length,    path bytes   (relative to src/, "/" separators)
    u64-LE content length, content bytes
```

Every field is length-prefixed, so different file sets never frame to the same
bytes.  A symlink, a non-UTF-8 name, an oversized or too deeply nested file
under `src/` is an error (producer and checker), never skipped.

`evidence/bench-manifest.json` is written by the (hand-written) microbenchmark
target in the same format as the test manifest (`kind: "bench"`), so the report
can read the zero-allocation and equal-step results instead of asserting them.

Manifests are checked in. Regenerating them is part of the package `BUILD`;
because they are deterministic, `--check` on a stale or hand-edited report fails.

## 9. Report derivation and fail-closed checking

`generate_ct_compare_report` takes the manifests as input and computes:

* per claim symbol: `property.cases` = sum of `cases` over that symbol's
  `property` entries; `fuzz.iterations` = sum of `iterations`; `example` =
  `pass` iff every example entry for the symbol passed;
* per claim: `proof.label` = the weakest label among the symbol's obligations;
  `proof.certificate` = `drat:sha256:<digest>` where `<digest>` is the SHA-256
  of the sorted, newline-joined certificate hashes backing that claim;
* every `PackageAuditContext` flag from a manifest entry (never `true` by
  default);
* every CWE and exemption evidence string must be an id present in a manifest.

`--check <report>` additionally fails closed (new `CheckError` variants) when:

1. an evidence id in a claim, weakness row or exemption is absent from the
   manifests (`UnknownEvidence`);
2. a count, label or verdict disagrees with the manifests (`EvidenceMismatch`);
3. a certificate hash does not match a file on disk, or the file's SHA-256 does
   not equal the name/hash recorded (`BadCertificate`);
4. the Warrant document declares a claim (or an op with `secret` labels or
   `bound` rows) that has no evidence entry (`UncoveredClaim`);
5. the manifest's `spec_sha256` differs from the Warrant document on disk
   (`StaleEvidence`);
6. any manifest entry covering a claim has verdict other than `pass`.

7. the report claims "no unsafe" (an `analyzer::no-unsafe-or-ffi` id, or an
   `excluded` memory-safety row) while the effect certificate declares an
   `unsafe_code` exception (`UnsafeContradiction`, AF-22). With such an
   exception CWE-125/787 are `discharged` from the bounds-checked proofs
   (bounded, not a Miri run) and CWE-416/415/476/190 are unapproved exemptions.

Further checks added by the security review (each has a falsification test in
`tests/test_evidence.rs`):

8. **Claims are matched by exact id, once.** A duplicate claim id, a claim with
   no derived counterpart, or a different claim count fails (`DuplicateId`,
   `UnknownEvidence`, `EvidenceMismatch`); a second copy can no longer hide
   behind a genuine first one.
9. **The generic checker validates what the report states**: a claim's
   `exhaustive.status` must be `pass`, its `proof.label` must have a rank
   (`proved`, `bounded(K)`, or the registry's `proved-to-gate-level`), and under
   a `pass` verdict no claim may list a `*:unchecked` assumption.  Rungs E3/E4 are
   judged *per operation* that has claims (each needs passing, non-empty
   evidence), and E5 requires every operation to have obligations that all passed
   with a ranked label.
10. **The effect certificate is parsed strictly** (typed `label`,
    `declared_effects`, `banned_construct_exceptions`; any wrong type or absent
    field is an error, never "clean").  A construct whose name contains `unsafe`
    or `ffi` (any case) is an unsafe-bearing exception.  As a cross-check
    `src/**/*.rs` is scanned for the keyword `unsafe` outside comments and
    literals; `unsafe` in the code with no declared exception fails at load and in
    `verify_evidence` (`UnsafeContradiction`), and also withdraws the
    `analyzer::no-unsafe-or-ffi` attestation.
11. **Certificates are bound to obligations (partial).** Every `constant-time` /
    `steps-bound` obligation with verdict `pass` must cite at least one
    certificate and carry a well-formed `obligation_sha256`.  A claim backed by
    a collapsed-miter certificate carries `proof.certificate_kind:
    "trivial-miter"` in the report (`"miter"` otherwise); `--check` fails a report
    that omits the mark (`TrivialCertificateUnmarked`).  Recomputing
    `obligation_sha256` from the source is *not* done: see WD-009 in section 10.
12. **Test and bench manifests are checked for staleness** against every
    `src/**/*.rs` (`source_sha256`, framing in section 8), like the proof
    manifest: editing `src/capabilities.rs` alone makes `--check` fail.
13. **Spec and ids.** The spec path in both manifests must be exactly
    `code/specs/<package>.warrant` (relative, no `..`), all three manifests must
    name the spec's module, evidence ids may not repeat within or across
    manifests, may not equal an attestation id, and may not start with
    `unverified::`; JSON objects with duplicate keys are rejected.
14. **The "no unsafe" contradiction uses structured fields** (status, CWE class,
    evidence ids), not the free-text `reason`: with an unsafe exception no row may
    cite the no-unsafe attestation, no memory-safety row may be `excluded`,
    CWE-416/415/476/190 must stay exemptions, and CWE-125/787 may be discharged
    only on a proof-manifest id.
15. **`unverified::` ids fail `--check`** unless `--allow-unverified` is given
    (every accepted id is then printed, one per line, not only a count).
    `ct-compare`'s `BUILD` passes the flag because its report still cites nine
    `unverified::` AST-audit attestations.  **The flag is temporary** and is
    tracked by backlog item WD-008; removing it is the incentive to replace the
    attestations with checked evidence.
    Counts in manifests must be integers in `0..=2^53`, and the per-operation
    totals are summed with checked arithmetic; anything else fails closed.
    The `unsafe` scanner fails closed too: a file it cannot tokenise counts as
    containing `unsafe`.
16. **Inputs are size-capped**: manifests, the effect certificate, the Warrant
    document, sources and the report at 16 MiB, certificate files at 64 MiB.

17. **Weakness rows are re-derived against every obligation of their kind.**
    CWE-400 rests on the steps-bound obligations and CWE-208 on the
    constant-time obligations *of every operation*, not only the live ones.  If
    any obligation of the kind is not a ranked `pass` (`not-live`, `unchecked`,
    failed, unranked), the generator emits no discharge and cites no proof id
    for the row (`not-live` becomes the unapproved computed exemption, `unchecked`
    alone leaves the row open), and `--check` rejects a report whose CWE-400 row
    is `discharged`, or whose CWE-400 / CWE-208 row cites a proof id of that kind
    (`EvidenceMismatch`).  A mix of proven and unproven operations therefore
    cannot be laundered into a discharge by the proven ones plus the benchmarks.
18. **`not-live` is declared, not asserted.**  A `not-live` proof entry must have
    no certificates, the label `not-live` and a non-empty `note`, and its
    operation must carry `proofs_not_live=<reason>` in the package's
    `evidence.bindings` (reviewed input, parsed by the same
    `parse_evidence_bindings` the generator uses).  `Evidence::load` refuses it
    otherwise, so weakening a proof to `not-live` needs a visible edit to a
    second file.
19. **Bindings are validated before use** (`warrant-grammar`): `package` and
    `export` are Rust identifiers, `exhaustive_budget` is at most 2^26, `spec`
    and `output` are plain relative paths pinned by `gen_evidence` to
    `code/specs/<package>.warrant` and `code/packages/rust/<package>/tests/generated/*.rs`,
    and the `<package>` argument is a plain directory name.  `--generate` reads the
    package version from the `[package]` table of `Cargo.toml` only.

20. **CWE-208 on a document with no secret operand** (first adopter
    `wasm-leb128`).  The attestation `warrant-spec::all-operands-public` exists
    only while the spec on disk declares at least one operation and every
    parameter of every operation has a `labels` row and no row is `secret` (an
    unlabelled parameter is unclassified, not public).  With that attestation
    and *no* constant-time obligation in the proof manifest, the CWE-208 row is
    an unapproved exemption `EXEMPT-CWE-208-NO-SECRET-OPERANDS` (kind
    `not-applicable`, reason "not applicable: no secret operands ...", evidence
    the attestation, derived assumption list empty): not `excluded`, because the
    registry gives CWE-208 no safe subset (VER15 section 4).  It is a separate
    branch, not a relaxation: any constant-time entry, proven or not, keeps the
    earlier outcomes (a gate-level exemption, "applicable and unaddressed", or an
    open row), and without the attestation the row stays open.  `--check`
    re-derives it: citing the attestation where it does not hold is
    `UnknownEvidence`, and the exemption id without both facts is
    `EvidenceMismatch`.  The id is reserved, so no manifest entry may use it.
    The audit profile that picks the static CWE-326/327/407 sentences gains
    `varint-codec`.
21. **CWE-409 (decompression bomb) is derived, and never excluded for a package
    that decompresses** (first adopter `lzw`).  The matrix used to print one row
    for every package, *excluded*, "implements no decompression or archive
    decoding routines": true for the earlier adopters, false for a decompressor.
    The row is now `excluded` only when the audit profile does not decompress
    *and* the document declares no `output` bound row (WX-H, VER04j section 6).
    Otherwise it is derived from the document and the test manifest: every
    operation with an `output` row must also declare a `heap` row, the manifest
    must hold passing exhaustive, property and fuzz suites named
    `<op>/output-bound` for it (the generated property: an `Ok` result is no
    larger than the row), and no observed manifest entry of the operation may have
    `max_observed_heap > bound_at_max` (the allocation observer's measurement
    against the `heap` row).  Complete evidence is `discharged`, labelled bounded
    (VER15 `bounded(k)`: the bound is checked on enumerated and sampled inputs and
    on the document's declared bomb examples, not proved for every input), and
    cites exactly those suites.  Any missing, failing or over-bound fact is an
    open row, so no report is generated.  The profile `lossless-compression`
    applies the class even to a document with no `output` row (that is an open
    row), and an `output` row applies it under any profile.  `--check` re-derives
    the facts in both directions: a report that excludes the class for a document
    with an `output` row, discharges it on evidence the manifest does not support,
    cites a different id set, or records an exemption, is `EvidenceMismatch`.
    The audit profile gains `lossless-compression`.

22. **Audit profiles are data, not code** (VER-P030 follow-up; closes backlog
    AR-001, narrows LP-010b).  Items 20 and 21 each added an enum variant, a
    `FromStr` arm, usage text and wording `match` arms to this crate, so every
    adopter edited shared source, re-emitted this crate's effect certificate and
    forced a rebuild of everything that depends on it.  The audit profile is now a
    declarative file, `assurance-profile.json`, beside the package's `Cargo.toml`.
    A new adopter adds that file and edits no Rust in `ca-assurance-report`.

    *Where the file lives, and why not in the Warrant document.*  The Warrant
    document is bound into every test and proof manifest by `spec_sha256`: a
    wording change there would invalidate the manifests and force the exhaustive
    suites and the DRAT certificates to be regenerated for a sentence.  It is also a
    language shared with the type checker, the code generators and the polyglot
    lanes (VER04i), so a new section would be a grammar change in shared crates, the
    same coupling this item removes.  The profile says nothing about behaviour; it is
    audit wording about the report's static rows.  It is therefore a sibling file,
    bound separately (below).

    *Shape* (`schema` `warrant-assurance-profile-v1`; every object has exactly the
    keys shown, any other key is refused, and the file is read with the manifest size
    cap):

    ```json
    { "schema": "warrant-assurance-profile-v1",
      "name": "varint-codec",
      "kind": "transform",
      "rows": { "326": { "applicability": "excluded", "reason": "..." },
                "327": { "applicability": "excluded", "reason": "..." },
                "407": { "applicability": "excluded", "reason": "..." } } }
    ```

    `name` is a label (`[a-z0-9-]{1,64}`) the report records; `--profile NAME`
    remains an optional pin that must equal it.  `kind` is a closed vocabulary
    whose semantics live in code: `transform` (nothing expands attacker-controlled
    input) or `decompressor`.  `rows` must declare CWE-326, CWE-327 and CWE-407
    (an absent row refuses the file: nothing defaults to a sentence) and may
    declare CWE-409 for a `transform` only; no other CWE may be declared.
    `applicability` has exactly one value, `excluded`; a profile cannot declare a
    row discharged, an exemption or approved.  `reason` is one line of printable
    ASCII, 1 to 2048 bytes, without the reserved suffix the matrix appends itself.

    *What stays in code, and cannot be weakened by data.*  The profile supplies the
    wording of a row and the declaration that it is excluded; the matrix still
    refuses the declaration when the evidence contradicts it.

    | Row | A declared exclusion is refused when |
    |-----|--------------------------------------|
    | CWE-326, 327 | the effect certificate declares a `rand:` capability (key material needs entropy) |
    | CWE-407 | the document has no operation, or an operation with no `steps` bound row |
    | CWE-409 | the kind is `decompressor` (refused when the file is loaded), or the document declares an `output` row |

    Every other row (memory safety, CWE-208, CWE-400, the capability rows) is not
    declarable and is computed exactly as before.  CWE-409 is still derived from the
    document's `output`/`heap` rows and the test manifest (item 21), and the rule
    "a decompressor's CWE-409 is never `excluded`" is code keyed on
    `kind: "decompressor"`: that kind applies the class and is an open row without
    `output`/`heap` evidence.

    *Binding.*  The report records `profile_sha256`, the SHA-256 of the file's exact
    bytes, beside `profile` (the name).  `Evidence::load` reads and hashes the file;
    a missing or malformed file is a load error, so there is no default profile.
    `--check` refuses a report whose `profile_sha256` differs from the file on disk
    (an edited profile) or whose `profile` differs from the file's `name`.

    *Re-derivation (AR-001).*  `--check` rebuilds the weakness matrix from the
    evidence and the bound profile and compares it row by row with the report
    (`cwe`, `name`, `status`, `approved`, `reason`, `evidence`), and compares each
    exemption's rationale with its row.  Edited reason text, a report generated
    under another profile, and a profile that claims applicability the evidence
    contradicts (which makes the rebuild itself fail) are each `EvidenceMismatch`.
    `verify_evidence` runs the binding and the CWE-409 kind rule for every library
    caller; the re-derivation is `verify_rederivation`.

    *LP-010b stays open, narrowed.*  Deriving "decompresses" from the Warrant
    document alone is not possible: the only fact the document carries is the
    `output` row, which already applies the class, and a decompressor that forgot
    the row is exactly the case with nothing to derive from.  What changed is that
    the profile can no longer be omitted (no flag, no default) and is hash-bound and
    visible in review; a decompressor can still declare `kind: "transform"`.

    The wording the six adopters carried as code (comparison, field-arithmetic,
    varint-codec, boolean-logic, bit-arithmetic, lossless-compression) moved
    verbatim into their `assurance-profile.json`; each committed report changes only
    by the added `profile_sha256` line.

Evidence ids come from three places: manifest entries (`warrant/...`,
`bench/...`; must exist and have verdict `pass`), attestations derived from the
effect certificate (`effect-certificate::proved`, `capability-audit::*-empty`,
`analyzer::no-unsafe-or-ffi` only while no unsafe exception is declared) or from
the Warrant document (`warrant-spec::all-operands-public`, item 20), and
`unverified::<id>` static claims that no tool in the repository checks. The last
group is accepted but counted and printed by `--check`, and the row's reason says
so. Each failure mode has a falsification test (`tests/test_evidence.rs`).

## 10. Divergences and follow-ups

* VER04f §3 describes `Gen::bytes_gen(0, 64)` uniformly; this specification
  replaces it with relation-biased generators (§4). VER04f is updated.
* VER04e §4 negative controls remain; §6 above adds claim-level faults.
* A CT proof of `ct_eq` requires inlining the equal-length helper; the inliner
  is a pure WIR pass in `warrant-ir`, tested by differential execution against
  the unlined function on concrete inputs.
* **WD-009 (follow-up)**: `obligation_sha256` is recorded and shape-checked, not
  recomputed.  A checker that re-parses and re-lowers `src/lib.rs`, re-inlines the
  function, rebuilds the obligation text and recomputes the digest (and compares
  each certificate's CNF with the miter it claims to refute) would close the gap
  between "a valid refutation exists" and "this refutation is of that obligation".
  Until then a hand-written manifest with a valid but unrelated certificate and a
  plausible hash is not detected; the `trivial-miter` mark makes the weakest case
  (collapsed miters) visible.
* The pilot's memory-safety CWE rows assume "no unsafe"; `ct-compare` contains
  one audited `volatile_barrier` module with `unsafe`. The report derives
  `is_safe_rust` from the effect certificate and states the exception.
