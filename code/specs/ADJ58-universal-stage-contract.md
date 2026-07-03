# ADJ58 — The Universal Stage Contract (byte provenance at every stage)

> **Status (2026-06-04):** Built and run end-to-end. The byte-provenance invariant
> is now enforced at EVERY pipeline arrow, not just case→IR. A generic `Stage`
> gate + a composed `Trail` make the framework prove it accounted for 100% of each
> stage's input — or show the hole. Implementation:
> [`code/specs/data/adj57/pipeline/`](data/adj57/pipeline/) (`stage.py`, `run.py`).
> Builds on [ADJ57](ADJ57-byte-provenance-pipeline.md) (which gated one arrow).

## 1. The principle, absolute

> There is no privileged stage. Every transform — normalize, decompose, derive,
> ground, aggregate — takes an input and must **prove it accounted for 100% of that
> input**: every used unit cites the output it produced, every discarded unit
> carries a reason, and the proof is appended to one composed, auditable trail.

ADJ57 gated *one* arrow (case→IR). That was the hole: the framework *claimed*
end-to-end auditability while `derive` dropped facts silently, `ground` read a page
without a discard ledger, and `aggregate` abstained ad hoc. ADJ58 closes it — the
gate runs at every arrow, and a stage that fails to cover its input is a visible
**hole in the trail**, not a silent gap.

## 2. The contract — one gate, two input shapes

A stage's coverage proof partitions its input completely:

- **TEXT input** (a case, a source page): the proof is a byte partition — the
  segments must concatenate back to the input exactly. Each segment is `used`
  (cites what it produced) or `discard` (carries a reason).
- **ELEMENT input** (a list of facts, a set of grounded LRs): the proof partitions
  the set of element ids. `used ∪ discard == all ids`, disjoint; every used cites a
  `produced`, every discard a `reason`.

`clean` = covered AND every used cites + every discard reasons.
`Trail.ok()` = every stage clean = the byte-trail is **unbroken** from raw input to
final output. ([`stage.py`](data/adj57/pipeline/stage.py); 9 unit tests in
`test_stage.py`.)

## 3. The arrows, now all gated ([`run.py`](data/adj57/pipeline/run.py))

| stage | input | the contract it now honors |
|---|---|---|
| **decompose** | case bytes | typed facts (used, cite term) + reasoned discards = 100% of bytes (ADJ57) |
| **derive** | the facts | EVERY fact is used (role) or discarded-with-reason — *no silent drop of comorbidities* |
| **ground:**\* | each source page | the cited span (used) + the surrounding page (discarded-as-context) = 100% of the excerpt |
| **aggregate** | the grounded LRs | each LR used in the posterior, or abstained-with-reason (`direction_only`/`fabricated`) |

The `derive` retrofit is the visible win: in the prior brucella run, `glaucoma`,
`dyslipidemia`, `hypertension` were silently ignored. Under the contract they must
appear as `discarded: "comorbidity, no bearing on the leading diagnosis"` — logged,
auditable, defensible.

## 4. The run — Kikuchi-Fujimoto disease (PMC11724740)

A fresh case: 32-year-old woman, FUO + non-tender cervical lymphadenopathy +
pancytopenia. Ground truth (held aside): **Kikuchi-Fujimoto disease** — a *rare*
histiocytic necrotizing lymphadenitis, diagnosed by node biopsy. The leading
diagnosis the framework derived from the facts: **`kikuchi_fujimoto_disease` —
correct.**

**The audit trail came back UNBROKEN** — every stage accounted for 100% of its input:

```
[OK] decompose   text  1788 bytes = 46 used (1088b) + 47 discard (700b)
[OK] derive      elem    46 facts = 24 used      + 22 discard-with-reason
[OK] ground:prior          + 4 source pages, each = cited span + discarded context
[OK] aggregate   elem     4 units = 0 used       + 4 abstained-with-reason
=> trail UNBROKEN (every stage byte-accounted)    trail_ok=True, holes=[]
```

**The `derive` hole is closed.** All 46 facts are accounted for; the 22 discards each
carry a reason — `fatigue(increased)`: *"nonspecific constitutional symptom, no
discriminating value"*; `past_history(post_streptococcal_glomerulonephritis)`:
*"remote comorbidity, no current findings"*; `iv_drug_use(absent)`: *"lowers
blood-borne-infection prior, not discriminating for KFD"*. Negatives and
comorbidities are no longer silently dropped.

**And the verdict is the honest part:** the framework **abstained from a posterior.**
The prior came back `direction_only` (KFD is rare — no grounded prevalence), and the
three finding LRs all came back `direction_only` (there is essentially no published
likelihood-ratio literature for clinical-finding→KFD, because the entity is too rare
and biopsy-defined). So `aggregate` had **0 grounded inputs**, and the framework
reported: *"prior not grounded — no defensible posterior."*

That is exactly right. A hallucinating system would emit a confident "KFD 90%" with
invented LRs. This framework instead: (1) correctly named the leading suspicion, (2)
accounted for every byte of every stage (unbroken trail), and (3) **refused to
produce a number the evidence base cannot support** — the same conclusion the real
workup reached (it went to biopsy). The framework tells you precisely what it could
and could not ground, and the trail proves it left nothing out.

## 5. Why this matters

The framework's promise is *auditability* — a verdict you can trace to source bytes.
A single ungated arrow breaks that promise everywhere downstream of it: you cannot
trust a number whose derivation dropped part of its input for reasons it never
recorded. Making the gate universal is what lets the framework *truthfully* claim
the trail is unbroken — and, where it isn't yet (a stage that can't cover its
input), say so out loud instead of papering over it.

## 6. Next

- **Normalization stages** (source-text + IR) plug into the same contract — each is
  a provenance-preserving transform with its own used/discarded ledger and a
  raw→normalized offset map (the recursive byte-provenance applied to normalization
  itself).
- **CAS-first grounding** (hash the normalized page before decomposing; reuse the
  decomposition on a hit) — so the `ground` stages are skipped when a source is
  already decomposed in the store.
- Promote the trail into the audit-trail schema (ADJ07) so replay covers every
  stage, not just LLM calls.
