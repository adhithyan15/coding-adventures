# PAPER2 · MYCIN-2026 — build spec (LLM-derived → adversarially-gated → CAS-committed → adj-lang-compiled)

> Concrete build spec for the derive-once MYCIN proof. The high-level proof design and hypotheses are
> in [`PAPER2-MYCIN-derive-once-proof.md`](PAPER2-MYCIN-derive-once-proof.md) (W7); **this** doc is the
> *how to build it*, pinned to the in-repo **adj-lang** language as the compile target. Specs-first per
> repo standard — no implementation lands before this is reviewed.

## 0. The one-paragraph build

An LLM reads byte-grounded clinical literature and **derives candidate adj-lang clauses** (a `prior`,
`contributes`, `interacts`, or `uncertain` clause, each carrying a `source`/`locator`). Each candidate
passes through a **CAS-write gate** — *N independent adversarial readers (vote) × byte-stability ×
blind-judge concurrence* — which either **commits** it to a content-addressed store (assigning a
`trust` tier it has earned) or **kicks it back**. The committed clauses are emitted as an **adj-lang
program** and compiled (existing `adj-lang` Rust crate) into a `LoweredProgram { kb, queries }`; the
`logic-engine` then decides held-out cases **on CPU with zero answer-time model calls**, each decision
emitting a **proof DAG** that cites the source bytes behind every fired clause. Corrections are CAS
edits (new version, regression-gated) that **propagate** to every case citing the clause and **survive
a model swap**.

This composes primitives the repo already has; the risk is integration, not invention.

## 1. The four-step pipeline (your framing) → mechanism → existing primitive

| step | mechanism | reuse | new work |
|---|---|---|---|
| **1. Derive rules via LLM** | LLM: byte-grounded literature → candidate adj-lang clauses with verbatim `source` spans | `DeriveOnce` workflow phase (E2/recurring_cost); ADJ44 meningitis rulebook | clause-emission prompt + schema targeting adj-lang clause kinds |
| **2. Adversarial readings + vote** | N model-diverse readers try to **refute** the clause's grounding (inference read) AND its **discards**; majority vote; decision-sensitivity scoping | run100b `adversarial_entail.workflow.js` + `nreader_majority`; FORWARD-DESIGN §1 | the **discard read** (symmetric gate, unbuilt); per-clause adaptation |
| **3. Commit to CAS** | write only if **(N-reader-unrefuted ∧ byte-stable ∧ judge-concurring)**; assign earned `trust` tier; content-address + version | cas_exercise CAS-WRITE (byte-accounting); adj52/cas versioned overrides | **compose** the 3 gate legs into one `cas_write_gate`; the **byte-stability** leg (unbuilt) |
| **4. CAS → ADJ program** | emit committed clauses as `.adj`; compile → `LoweredProgram`; `logic-engine` decides cases on CPU + proof DAG | **adj-lang crate + logic-engine** (ADJ48 ran the ACS differential end-to-end); ADJ71 program-cache | the CAS→`.adj` serializer; warm-path harness asserting 0 model calls |

## 2. Why adj-lang is the right target (the reasoner gap is already closed)

adj-lang **is** the modern formalization of MYCIN's model — no extension needed for probabilistic
differential diagnosis:

- **Clauses** (`code/grammars/adj_lang.grammar`, crate `code/packages/rust/adj-lang/`):
  - `prior <p> for <hypothesis>` — disease base rate → `prior_logit = log(p/(1-p))`
  - `contributes <LR> from <finding> to <hypothesis>` — a likelihood ratio → `log(LR)`
  - `interacts <LR> when <f1> and <f2> for <hypothesis>` — synergy / explaining-away
  - `uncertain { h1, h2, ... } for <hypothesis>` — competing differentials (drives VOI)
  - `observe <finding>` / `? <hypothesis>` — the case facts and the query
  - every clause carries `source "..."`, optional `locator "..."`, and a `trust` tier
    (`consensus | authoritative | empirical | inferred | unattributed`).
- **Execution** is deterministic log-odds composition: `posterior_logit = prior_logit + Σ log(LR) + Σ
  log(joint_LR)`, `posterior = sigmoid(...)`; `logic-engine` `search(query, kb, LRAggregate)` returns
  `{ posterior, dag, uncertainties }`. The **proof DAG** enumerates every fired clause with its
  provenance — a machine-checked defensible chain that removes the LLM from the verification loop.
- **Uncertainty** (ADJ65): the engine reports VOI/decision-sensitivity — *does the unknown matter for
  this decision?* (Jane Doe: precipitator unknown ⇒ KICKBACK at P=0.369; with troponin ⇒ no kickback at
  P=0.824). This is the CPU-side "argmax + sensitivity" the proof design needs.

> Implication: the **CAS unit is an adj-lang clause**. "Derive a rule" = emit one clause; "commit to
> CAS" = the gate admits one clause and fixes its `trust` tier; "CAS → ADJ program" = concatenate the
> committed clauses for a hypothesis into a `.adj` rulebook and compile.

## 3. The CAS data model

A CAS entry is one **byte-grounded adj-lang clause**, content-addressed by a hash of
`(normalized clause text + source span + locator)`:

```
cas/objects/<hash>.json   {
  clause: "contributes 8.0 from biomarker(troponin_elevated) to acs",
  source: "Roffi M et al., 2015 ESC NSTE-ACS Guidelines",
  locator: "Table 5",
  source_bytes: "<verbatim quoted span from the literature>",
  trust: "consensus",                 # ASSIGNED BY THE GATE, not the writer
  gate: { nreader: {...}, byte_stability: {...}, blind_judge: {...}, verdict: "ACCEPT" },
  version: 1, supersedes: null
}
cas/rulebooks/<disease>@<v>.adj        # the compiled-from-CAS program for a hypothesis
```

Corrections create a **new version** that `supersedes` the old hash (the adj52/cas override pattern),
and re-derivation regenerates the `.adj`. The base store is immutable; edits are versioned + attributed.

## 4. The CAS-write gate (the new core component)

`cas_write_gate(candidate_clause, literature) → ACCEPT(trust_tier) | KICKBACK(reason)`.
A candidate is committed **iff all three legs pass**; the gate verdict is stored as provenance.

1. **N-reader adversarial vote** (≥3 readers, *independent of the writer*, model-diverse — Opus+Sonnet+
   Haiku, per W5: a different/stronger reader localizes better). Two link types, both refutation-framed:
   - **inference read** — *do the cited `source_bytes` entail this LR / prior?* (reuse run100b
     `adversarial_entail`). A LEAP = the weight is not grounded in the bytes.
   - **discard read** (NEW, FORWARD-DESIGN §1) — for each finding the derivation *dropped* as
     non-contributing, an adversary tries to show it is actually a load-bearing LR. A successful
     refutation = a dispositive finding was silently discarded (the dangerous direction in medicine).
   Commit requires majority concurrence; **decision-sensitivity (ADJ65) scopes** it — act on a LEAP or a
   load-bearing discard only if it is *outcome-pivotal* (the un-gated version over-abstains: run100 88→74).
2. **Byte-stability** (NEW as a commit gate) — resample the inference read K times; a low-consistency
   clause is not stable enough to cache (SelfCheckGPT/semantic-entropy used as a *write gate*, not a
   detector: byte-stability catches *invention*, the N readers catch *over-read/stable-error*).
3. **Blind-judge concurrence** — an independent blind judge, given only the `source_bytes`, must reach
   the *same* LR direction/magnitude band the clause encodes. Disagreement blocks the commit.

**Trust-tier assignment** is the gate's output, mapped to adj-lang's existing vocabulary: unanimous
N-reader + ≥2 independent sources → `consensus`; single authoritative source, judge-concurring →
`authoritative`; cohort/empirical source → `empirical`; passes structure but no external citation →
`inferred` (admitted but flagged); fails → KICKBACK (never silently `unattributed`). This is exactly
how the gate handles ADJ44's *flagged-for-verification* citations: they cannot reach `authoritative`
and are either re-grounded or carried as explicitly-flagged `inferred` (the correctability story).

**Role → model is configuration** (FORWARD-DESIGN §3): derive / adversary-read / byte-stability /
blind-judge are separable seats, each assigned the cheapest-capable model; the adversary must be
independent of (and may be stronger than) the writer.

## 5. End-to-end mechanics (cold path → warm path)

**Cold (derive once, with the model):**
1. `DeriveOnce`: LLM reads a byte-grounded literature passage → candidate clause(s) + `source_bytes`.
2. `cas_write_gate` on each candidate → ACCEPT(tier) | KICKBACK. Record gate provenance + cold cost
   (model calls, tokens, wall-clock).
3. Commit accepted clauses (content-addressed). Serialize the disease's accepted clauses → `<disease>@v.adj`.
4. `adj-lang::compile(.adj)` → `LoweredProgram { kb, queries }` (the compiled, cached library).

**Warm (reuse indefinitely, CPU only):**
5. Small/local model ingests a held-out case → `observe` facts only (byte-accounted; *no* new rules).
6. Emit the case program: the `observe`/`?` lines linking the cached rulebook; `logic-engine`
   `search(?, kb, LRAggregate)` decides on CPU. **Assert `answer_time_model_calls == 0`.**
7. Emit the proof DAG (every fired clause → its `source_bytes`) + VOI/sensitivity → kickback if a
   load-bearing finding is unknown.

## 6. First vertical slice (what to build first — minimal, decisive)

Prove the whole loop on **bacterial vs. viral meningitis**, ~6–8 clauses, before scaling to MYCIN's full
differential:

```
code/specs/data/mycin-derive-once/
  literature/                # small byte-grounded source corpus (public guidelines, no PHI)
  derive.workflow.js         # DeriveOnce: literature -> candidate adj-lang clauses
  cas_write_gate.py + .workflow.js   # N-reader (inference+discard) x byte-stability x blind-judge
  cas/objects/ , cas/rulebooks/meningitis@1.adj
  cases/                     # 3-5 held-out synthetic cases (e.g. classic bacterial CSF; viral; ambiguous)
  warm_run.{py,sh}           # compile .adj (adj-lang crate) + logic-engine search; assert 0 model calls
  FINDINGS.md
```

The slice must exercise: (a) the gate **kicking back** ≥1 deliberately weak/ungrounded candidate
(seed one), (b) ≥1 case decided at **0 answer-time model calls** with a complete proof DAG, (c) one
**rule-edit → new CAS version → propagation** to a sibling case, (d) **model-swap** of the case-ingest
model leaving the corrected library unchanged.

## 7. Gaps inventory (build vs reuse)

- **Reuse as-is:** adj-lang crate + logic-engine (the reasoner — gap C is *closed*), `adversarial_entail`
  + `nreader_majority` (inference read + vote), decision-sensitivity, adj52/cas versioning, ADJ71
  program-cache pattern, cas_exercise CAS-write byte-accounting.
- **Build:** (A) the composed `cas_write_gate` ; (B) the **discard adversarial read** ; (the byte-stability
  commit leg) ; (the CAS→`.adj` serializer + trust-tier assignment) ; (the warm-path harness asserting 0
  model calls + proof-DAG completeness check) ; (the byte-grounded meningitis literature corpus + cases).
- **Decision taken:** reasoner target = **adj-lang** (in-repo, byte-provenance native, MYCIN's own model).

## 8. Metrics (from W7, made concrete on adj-lang)

- `answer_time_model_calls` on warm cases — **0 is the headline** (assert in harness).
- `gate_precision/recall` — on a set of seeded good/bad candidate clauses, does the gate ACCEPT the
  grounded ones and KICK BACK the ungrounded/over-read/bad-discard ones? (the gate's own validity.)
- `parity` — compiled-CPU decision vs LLM-answer-time decision on the same cases (no accuracy tax).
- `proof_dag_completeness` — every fired clause cites `source_bytes`; reviewer audits the DAG, never
  re-runs the model.
- `correction_persistence` + `propagate` — edit a clause → new CAS version → corrects this case and all
  siblings citing it; `model_swap_durability` — survives swapping the ingest/derive model.
- `cold_vs_warm_cost` — amortization curve (derive-once pays off after k cases).

## 9. Honest risks

- **Citation verification.** ADJ44's clinical citations are *flagged-for-verification*; the gate is the
  mechanism that surfaces this (they can't earn `consensus`/`authoritative`), but the literature corpus
  must contain real verbatim source spans or the slice degrades to `inferred`-tier clauses — report tier
  distribution honestly.
- **LR sourcing.** Likelihood ratios must come from cited literature, not invented; the byte-stability +
  blind-judge legs target exactly LR-magnitude hallucination, but small-n means this is a mechanism demo,
  not a calibrated clinical tool. Say so.
- **No clinical claim.** This resurrects MYCIN's *method* on a new substrate; it is not a deployable
  diagnostic. PHI-local deployment is PAPER2 E3, separate.
- **adj-lang coverage.** Confirm the crate handles the clause kinds the meningitis differential needs
  (multi-hypothesis `uncertain`, joint `interacts`); if a needed construct is missing, that becomes a
  scoped adj-lang extension PR, not a workaround.

## 10. Build order (PR slices, specs-first)

1. **This spec** (PR; no code). ← current.
2. Byte-grounded meningitis literature corpus + 3–5 held-out cases (data PR).
3. `cas_write_gate` — compose N-reader (inference) × byte-stability × blind-judge + trust-tier mapping;
   validate on seeded good/bad clauses (`gate_precision/recall`).
4. The **discard adversarial read** added to the gate.
5. `DeriveOnce` clause-emission + CAS commit + CAS→`.adj` serializer.
6. Warm-path harness: compile via adj-lang crate, `logic-engine` search, **assert 0 model calls**, proof
   DAG completeness.
7. Correction-persistence + model-swap; amortization curve; FINDINGS.

Output root: `code/specs/data/mycin-derive-once/`.
