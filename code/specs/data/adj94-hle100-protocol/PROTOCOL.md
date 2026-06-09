# ADJ94 — Experimental protocol: 100-case HLE, 7 arms, byte-provenance enforced at every level

Status: **PRE-REGISTERED PLAN — not yet executed.** Hypotheses, arms, metrics, and analysis
comparisons are fixed *before* the run so results aren't fit after the fact. Open decisions
(needing sign-off) are listed in §12.

## 1. Objective
At N=100 HLE items, measure whether the byte-provenance framework lifts a cheap model (Haiku) and
a frontier model (Opus) over their blind baselines, separate the **retrieval** contribution from
the **reasoning-discipline** contribution (closed-book vs open-book), and test the **CAS
division-of-labor** thesis — Opus builds the IR/CAS once, Haiku reasons over it cheaply — at scale.

## 2. Arms (7 per item)
| # | code | model(s) | retrieval | description |
|---|---|---|---|---|
| 1 | `blind-haiku` | Haiku | none | one-shot answer, no framework, closed-book |
| 2 | `blind-opus` | Opus | none | one-shot answer, no framework, closed-book |
| 3 | `fw-haiku-cb` | Haiku | none | full framework, closed-book |
| 4 | `fw-opus-cb` | Opus | none | full framework, closed-book |
| 5 | `fw-haiku-spider` | Haiku | web | framework + Haiku spider, open-book |
| 6 | `fw-opus-spider` | Opus | web | framework + Opus spider, open-book → **populates CAS** |
| 7 | `fw-haiku-cas` | Haiku | CAS only | framework, Haiku reasons over the **Opus-built CAS from arm 6** (no new search) |

Arm 6 writes a persisted, provenance-tagged CAS per item; arm 7 consumes it. **Arm 7 depends on
arm 6** (intra-item ordering, see §6).

## 3. Pre-registered hypotheses (falsifiable)
- **H1 — open-book lift:** `fw-*-spider` > `blind-*` on accuracy, for both models.
- **H2 — closed-book null:** `fw-*-cb` ≈ `blind-*` on accuracy (no closed-book accuracy lift), replicating ADJ92 at scale. *Directional prediction: closed-book framework does NOT beat blind, and may slightly trail.*
- **H3 — CAS division of labor:** `fw-haiku-cas` ≈ `fw-opus-spider` on accuracy AND defensibility (cheap reasoner over the Opus-built CAS matches the Opus framework). *This is the headline.*
- **H4 — cost-collapse (the dream):** `fw-haiku-cas` ≥ `blind-opus` — a cheap model over an Opus-built CAS beats the blind frontier.
- **H5 — defensibility dominates:** all framework arms ≫ blind arms on **defensibility**, in *both* book modes — the framework's true axis, expected to win even where accuracy doesn't.

Each hypothesis maps to a fixed comparison in §9. A result that contradicts a prediction is reported as-is (we have already published two thesis-correcting results this line of work; honesty is the point).

## 4. Byte-provenance enforcement matrix (the core requirement)
Every framework arm (3–7) enforces the full stack we built. Enforcement point → mechanism (origin):

| invariant | mechanism | arms | origin |
|---|---|---|---|
| **Input coverage** — every given datum is load-bearing or justified-discarded | use-audit (load-bearing, not mention) + adversarial discard check | 3–7 | ADJ88 |
| **Inference support** — every inferred fact follows from cited support | per-fact support audit, default UNSUPPORTED; entailment check | 3–7 | ADJ61/89 |
| **Grounded adversarial read** — every auditor objection cites verbatim bytes | objection carries `grounding_quote`; deterministic provenance filter drops ungrounded objections | 3–7 | ADJ91 |
| **Convergence control** — no oscillation; missing standard constant → explicit assumption | category split + stop-when-no-new-issue | 3–7 | ADJ90 |
| **Closed-book domain grounding** — a correct-but-not-in-problem domain fact is accepted, not rejected | auditor `domain-knowledge` category (REQUIRED FIX, §11) | 3, 4 | ADJ92 lesson |
| **Spider provenance** — every retrieved fact cites its source (URL/span) | spider IR entries `{statement, source}`; facts without a source are dropped | 5, 6 | ADJ87/88 |
| **CAS provenance** — every cached IR entry retains its source; reuse preserves it | CAS records `{statement, source, retrieved_by, item_id}`; arm 7 cites the same sources | 6→7 | this run |
| **Closed-book integrity** — no web leaked into a closed-book arm | deterministic URL-leak scan on every answer; closed-book arms must be leak=0 | 1–4 | ADJ92 |
| **Context neutralization** — HLE question is not mistaken for a codebase query | NEUTRAL preamble on every agent | all | ADJ87 |

A run is **provenance-valid** for an item×arm iff: coverage gate closed, no surviving ungrounded objection, (closed-book) URL-leak=0, (open-book) every used fact has a source. Provenance-invalid cells are flagged, not silently dropped.

## 5. Item set & sampling
- Source: `cais/hle` (already downloaded locally in ADJ88; `HF_HUB_DISABLE_XET=1`).
- Filter: **text-only** (no image questions), **exact-match-gradable** answer format.
- Sample: **100 items, fixed seed**, drawn *once* and frozen to `items_100.json` **before any arm runs** (pre-registration). Record each item's id, question, gold answer, and HLE category.
- Stratification (decision §12-A): either uniform-random or stratified by category (reasoning / lookup / recall) so we can read H1–H4 *per stratum* (the retrieval vs reasoning split is category-dependent — recall items need the spider, derivation items need reasoning).
- Contamination guard: none of the 100 may overlap the 10 ADJ88 items used to tune the framework.

## 6. Per-item pipeline (DAG)
```
            ┌─ blind-haiku ─────────────┐
            ├─ blind-opus ──────────────┤
  item ─────┼─ fw-haiku-cb ─────────────┤
            ├─ fw-opus-cb ──────────────┤
            ├─ fw-haiku-spider ─────────┤
            └─ fw-opus-spider → CAS ──┬─┤
                                      └─ fw-haiku-cas (reads CAS) ─┘
                                                                   │
                                                          grade all 7 (blind)
```
Arms 1–6 run concurrently; arm 7 starts when arm 6's CAS is written. All 7 answers go to a single
blind grading stage. The CAS artifact persists to `cas/<item_id>.json` for later adversarial reuse.

## 7. Grading & blinding
- **Blind judge = Opus** (a neutral model, not in the answer-producing loop), given `{question, gold, answer}` with **no arm label** (shuffled `blind_map` per item).
- Two scores per answer:
  - **Accuracy**: `correct / partial / incorrect` vs gold (exact-match-aware).
  - **Defensibility** (0–5 rubric): is the answer's reasoning *grounded, auditable, and free of unsupported leaps* — independent of whether the final answer is right? (This is the framework's target axis; ADJ-line standing principle: score defensibility, not just correctness.)
- Grader sees only the final answer + work, never the arm identity or the other arms' answers.
- Reliability: re-grade a 10% random subset with a second blind judge; report agreement.

## 8. Metrics (per arm, over 100 items)
1. **Accuracy**: P(correct); P(correct or partial).
2. **Defensibility**: mean 0–5; fraction ≥ 4.
3. **Provenance completeness**: fraction of cells provenance-valid (§4); for open-book arms, mean fraction of used facts with a source; for closed-book arms, URL-leak rate (must be 0).
4. **Cost**: output tokens per cell (the cost-delta is the whole point of the CAS arms).
5. **CAS reuse integrity** (arm 7): fraction of arm-7 cited facts traceable to arm-6 CAS entries.

## 9. Analysis plan (fixed comparisons)
| hypothesis | comparison | metric | decision rule |
|---|---|---|---|
| H1 | `fw-haiku-spider` vs `blind-haiku`; `fw-opus-spider` vs `blind-opus` | accuracy | McNemar paired test, α=0.05 |
| H2 | `fw-haiku-cb` vs `blind-haiku`; `fw-opus-cb` vs `blind-opus` | accuracy | McNemar; pre-registered as a *null/negative* expectation |
| H3 | `fw-haiku-cas` vs `fw-opus-spider` | accuracy & defensibility | equivalence (TOST) — success = *not worse* by >1 grade |
| H4 | `fw-haiku-cas` vs `blind-opus` | accuracy & defensibility | McNemar; H4 holds if cas ≥ blind-opus |
| H5 | each `fw-*` vs `blind-*` (same model) | defensibility | Wilcoxon signed-rank, both book modes |
All comparisons are **paired by item**. Report per-stratum (§5) as well as pooled. N=100 gives ~80% power to detect a 15-point accuracy difference (paired); defensibility deltas are expected larger.

## 10. Scheduling (rate-limit safe)
- **20 batches × 5 items**, **30-minute break** between batches (per request).
- Each batch = a self-contained workflow run over its 5 items (all 7 arms + grading), writing partial results + CAS to disk immediately (crash-safe / resumable).
- A `CronCreate` driver fires the next batch ~30 min after the previous batch *completes* (not a fixed wall-clock, to absorb variable batch duration).
- Resume: each batch keyed by item ids; a re-fired batch skips items whose results already exist on disk.
- Estimated wall-clock: 20 batches × (~10–20 min run + 30 min break) ≈ **13–17 hours**.

## 11. Pre-run readiness checklist (MUST pass before batch 1)
1. **Consolidate one parameterized framework module** (single source of truth) exposing `solve(item, {model, mode: closed|spider|cas, cas?})` with the full §4 stack. Arms are config combinations of this one module — no per-arm drift.
2. **Closed-book domain-grounding fix** (ADJ92 lesson): add the auditor `domain-knowledge` category so a correct-but-ungroundable domain fact (e.g. `Ω₈^Spin = Z²`) is accepted as a flagged knowledge-claim, NOT rejected. **Validate it stops the bordism derail** (closed-book `fw-opus-cb` must not underperform `blind-opus` on bordism) before scaling.
3. **CAS read/write contract** finalized (`cas/<item_id>.json`, provenance-tagged) + a 1-item round-trip test (arm 6 writes, arm 7 reads & cites).
4. **Blind grader rubric** frozen; smoke-test on 3 known items (incl. an exact-match and a partial).
5. **10-item pilot (2 batches)** end-to-end, all 7 arms + grading + provenance validation + URL-leak check, artifacts saved. Inspect for the ADJ87 context-contamination bug and the ADJ86 stall before committing to all 20 batches.

## 12. Decisions (RESOLVED — signed off)
- **A — Sampling**: **stratified by category** (reasoning / lookup / recall), so H1–H4 read per stratum. ✅
- **B — Samples per cell**: **N=1** per arm per item; re-sample only flagged close calls. ✅
- **C — Scope**: **50 items × 7 arms** (10 batches of 5). ~1,800–2,200 agents, ~45–65M output tokens, ~7–9h. ✅
- **D — Closed-book enforcement**: prompt-prefix + URL-leak verification; report leak rate. ✅
- **E — Pilot gate**: **YES** — run the 10-item pilot (batches 1–2), then STOP for review before the remaining 8 batches. ✅

Revised batch plan: **10 batches × 5 items**; pilot = batches 1–2 (items 1–10) → review gate → batches 3–10 (items 11–50).

## 13. Artifacts (all saved, for later adversarial configs)
`items_100.json`, per-batch `results_batch_NN.json` (full per-arm outputs + work), `cas/<item_id>.json`,
`grades.json` (blind, with maps revealed post-hoc), `provenance_audit.json`, `cost.json`, and a final
`FINDINGS.md`. Everything keyed by item id for re-analysis and adversarial reuse.
