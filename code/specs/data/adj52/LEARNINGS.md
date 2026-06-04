# ADJ52 — running learnings log

Per-batch learnings from building and running the autonomous blind
cross-arm experiment loop. Newest first.

## 2026-06-03 — Batch 5: ADJ54 — calibration-regression harness + H2 (anti-entropy)

The lesson from the softmax regression (Batch 2) was methodological, not
tactical: a fix judged on the noisy n=3 blind-judge loop can regress
individual cases while the aggregate improves, invisibly. **Fix the
instrument, not the patch.** Built a deterministic, offline, per-case gate
(`calibration/score.py`): freeze a corpus of (rulebook, program, label),
score any engine change in ms/case, and FAIL on any per-case regression
regardless of the aggregate. Validated it both ways (passes identical; flags
a synthetic regression even with the aggregate left unchanged — the exact
softmax failure mode).

**Root-cause, not guess.** A failure-enriched 30-case diagnostic re-run
(persisting per-case artifacts, which run-3 discarded) gave 7 wrong cases;
each root-caused from its actual rulebook + deterministic trace + judge
rationale + ground truth. Universal finding: ALL 9 cases (incl. both correct
controls) assert ~99% while recommending the confirmatory test. The wrong
cases split into three levers: **H1** correlation de-stacking (true sibling
suppressed by over-stacking — case-5,7), **H2** open-question discounting
(saturated while a bearing uncertainty is open — 6/9), **H3** residual
hypothesis (true answer not in the differential at all — case-6,18,29).

**H2 shipped + gated.** Runner-level VOI-band tempering with the key
anti-entropy property: **rank on RAW posterior, calibrate on the tempered
REPORTED posterior** — so H2 cannot reorder the differential (zero
correctness regression by construction; the property softmax lacked). Gate:
baseline→H2 = **0 regressions**, accuracy unchanged (0.833), saturation
17→12, log-loss −13%. BUT confidently-wrong stayed at 5 — the instrument
proved H2 insufficient AND said why: bands are narrow because saturation is
H1-driven (over-stacked OBSERVED evidence), not the pending test.

**Takeaways:** (1) aggregate Brier lies at high accuracy (0.0006 on
correct-only vs 0.16 with the wrong cases) — the corpus MUST include the
failures. (2) Reporting must be decoupled from ranking; only
correctness-targeted levers (H1/H3) may reorder, and only under the gate.
(3) Do calibration fixes in the runner first (zero blast radius, like the
ADJ53 mechanism construct), not the Miri-checked logic-engine crate. Next:
H1 runner-level same-sign-contribution shrinkage (frozen-corpus testable),
then H3 deriver residual hypothesis (needs re-derivation). Spec:
`ADJ54-calibration-regression-harness.md`.

## 2026-06-03 — Batch 4: 100-case hands-off run (the scaled descent data point)

Workflow `wuja6iixk`: **100/100 completed, 0 skipped, 0 compile failures**; 500
agents, 17.7M tokens, ~60 min. Each seed self-found + perturbed its own case.
Full writeup + cross-tabs: `runs/run-3-100case.md`; data: `run-3-100case-summary.json`.

**Result:** framework correct **62**, plain correct **61** (parity); blind-judge
wins framework **39** / plain **60** / tie 1.

**The cross-tabs are the finding:**
- Correctness is essentially tied and symmetric (both 54; only-fw 8; only-plain 7;
  neither 31). The framework is NOT a worse diagnostician than frontier Claude.
- **28 of plain's 60 wins are cases the framework ALSO got right but lost on
  calibration/defensibility** (saturated posteriors + pseudo-precise logits +
  unverifiable citations). That is the entire competitive gap.
- The framework's genuine niche — won AND correct where plain was wrong — is **8/100**
  (real but small; should grow as the base model weakens, per the descent).
- **Saturation persists: 51/100 top posteriors >= 0.99, median 0.9907.** The
  `mechanism` construct was available but the deriver didn't lean on it enough to
  temper the headline — availability != use.

**Takeaways:** (1) the machinery scales hands-off; (2) correctness parity with
frontier is established; (3) the only thing between co-equal and ahead is
CALIBRATION — the most fixable place to lose; the addressable pool is the 28
right-but-overconfident losses. Next: make the deriver actually route correlated
findings through mechanisms, and cap/hold the posterior so it never reads ~99%
while a confirmatory `uncertain` marker is open.

## 2026-06-03 — Batch 3: first-principles redesign (ADJ53) — latent mechanism

Reverted the softmax patch (it normalized already-inflated scores — a symptom
fix). Root cause: the engine sums independent log-LRs (Naive Bayes), but findings
are correlated (manifestations of one cause), so it double-counts and saturates.
Spec: `ADJ53-latent-mechanism-and-recursive-source-trees.md`.

**Phase A shipped + verified — the `mechanism` construct.** A `% mechanism <M>
for <C> lr <L> : <m1>, <m2>, ...` directive groups correlated findings under one
latent cause. If >=1 manifestation is observed, it contributes its LR ONCE
(realized by generating a single `contributes` on a synthetic `mechanism_present(M)`
atom that the normal engine handles — no shared-crate change). Demo
(`fixtures/mechanism-demo/`): four correlated McArdle findings that would saturate
to ~0.98 encoded flat now fire once → **P = 0.64**. The over-counting is fixed.

**Phase B (next):** promote the surface syntax into the adj-lang CORE grammar
(`adj_lang.tokens` + `.grammar` + grammar-tools regen + adapter + ast + lower),
done as a dedicated change — NOT rushed, because adj-lang has no build.rs (manual
grammar regen) and a botched regen cascades across the workspace.

**Also reframed (ADJ53):** the goal is NOT to beat frontier Claude — it's the
TOP-DOWN descent. Establish the machinery works at the top, then step the model
down and find the breaking point; the framework's value is how far down it raises
the floor. "Framework loses to frontier" was the top of the descent, not a failure.

## 2026-06-03 — Batch 2: calibration fix did NOT win (framework 0/3)

Re-ran the 3 perturbed cases with the coherent-differential softmax + exclusivity
tags + rulebook/program separation. **Framework 0/3, plain Claude 3/3** (was
1/3). Framework still reached the correct answer/family 2/3 (1 partial). Full
writeup: `runs/run-2-calibration-fix.md`.

**The proof (why it lost every time):**
1. The softmax fix tempered MULTI-hypothesis incoherence but NOT
   single-hypothesis saturation — the dominant hypothesis still hits ~100% and
   now confidently excludes competitors at ~0%, which is WORSE when the top pick
   is a wrong sibling (case-2: 100% Killian-Jamieson vs 0% Zenker, the correct
   answer — actively argued against it).
2. **"100% while recommending the confirmatory test" is the core incoherence.**
   The framework asserts certainty AND says "get the smear/genetics to confirm."
   A calibrated reasoner holds residual probability until the test returns. The
   engine collapses to 100% because observed evidence dominates and nothing
   discounts for the UNRESOLVED confirmatory uncertainty. This is the real
   "uncertainty at the core" gap.
3. Pseudo-precise logits + unverifiable citations are read as a NEGATIVE (false
   rigor / hallucinated PMIDs), not a plus.
4. The "plain" arm is frontier Claude reasoning fully — well-calibrated, names
   exact mutations, includes the right answer in its lead. Out-diagnosing it on a
   blind comparison is a very high bar; shared correctness doesn't win.

**Implication:** against a strong base model, "win a blind diagnostic comparison"
is the wrong metric — the framework is correct + auditable and still loses on
calibration. Paths: (a) deep engine calibration — never ~100% while a confirmatory
`uncertain` marker is open; temper LR magnitudes; stop displaying raw logits as if
measured; (b) reposition — test the framework wrapping a SMALL answerer model (the
ADJ17 regime where structure helps) and measure auditability + error-catching +
thin-rulebook robustness, not out-diagnosing frontier Claude. The one framework
WIN (run 1, case-2) was where the base model actually erred — that is its niche.

## 2026-06-03 — Batch 1: first FULLY HANDS-OFF run (3 perturbed cases)

`pipeline.workflow.js` ran 3 published "masquerade" cases end-to-end with no
human in the loop (15 agents). Each case diagnosis-invariantly perturbed to
defeat training recall. Full writeup: `runs/run-1-perturbed-3case.md`.

**Scorecard:** framework won 1/3, plain Claude 2/3, tie 0. Framework correct
2/3 (1 partial); plain correct 2/3 (1 partial). **0 compile failures.**
**Perturbation preserved the diagnosis in 3/3** (recall defeated; both arms
reasoned). The automation works: perturb→ingest→derive→run→judge→aggregate,
hands-off, no extraction.

**Findings (now consistent across 5 cases total):**
1. **Calibration is THE blocker.** Saturated/incoherent posteriors (multiple
   hypotheses ~100%; case-3 had paucibacillary 50.9% AND multibacillary 100% —
   not a coherent distribution) lose to plain Claude's graded confidence EVEN
   WHEN the framework is correct (cost it case-1 and case-3). #1 fix: a coherent
   NORMALIZED differential (compete the mutually-exclusive diagnosis queries,
   softmax their log-odds, temper extremes) — not more citations.
2. **Framework wins where the base model fails** (case-2: plain hallucinated a
   fish-bone foreign body; framework got the pharyngo-oesophageal diverticulum
   family). That is the regime where it earns its keep.
3. **Disposition gap:** the framework answered diagnosis + next test but omitted
   treatment/management the judge rewarded (stop methotrexate, G6PD, etc.). Add
   disposition/management queries.
4. Leak fixes held: opaque case ids, no diagnosis in any agent-visible field.

## 2026-06-03 — Batch 0c: novel case end-to-end (McArdle mimicking PMR)

First fully-novel case run through the WHOLE pipeline (ingest → derive →
compile → counterfactuals → plain-Claude → blind judge). Source PMC11724029;
ground truth late-onset McArdle disease (GSD V), initially misdiagnosed as
polymyalgia rheumatica. Artifacts in `cases/mcardle-pmr/`.

**Result.** The framework reached the CORRECT answer: diagnosis(mcardle) 100%
(top), correctly rejected inflammatory (3.4%) and diabetic (0.5%), penalized
PMR via the CK (−1.61), and recommended PYGM genetic testing (83.3%) — the
correct disposition. Plain Claude *also* got it right (Moderate confidence).
The **blind judge preferred plain Claude**, even with the framework's full
audit trail provided.

**Findings (consistent with Batch 0b, confounder now removed):**

1. **Over-saturated, internally-incoherent posteriors — CONFIRMED.** McArdle
   100%, coexisting 99.5%, PMR 97.5% all near-certain at once. The LR engine
   scores each query as an independent binary, so candidates never compete and
   don't form a coherent differential; reads as false precision. **#1 blocker
   to beating a strong base model.** Fix: normalize across mutually-exclusive
   candidates (softmax-style differential summing to ~1) + temper extremes.
   Serves the "uncertainty at the core" goal.
2. **Inert VOI markers.** Deriver declared `uncertain { test(...) }` but wrote
   no `contributes` FROM the test results → VOI range 0.0; the confirmatory
   test can't move the posterior. The panel correctly surfaced the gap. Fix:
   deriver must emit `contributes` from test-result terms to the conclusion.
3. **Term normalization needed.** Deriver emitted numeric/unit args
   (`age(64_years)`, `creatine_kinase_later_peak(3473_IU_per_L)`) that violate
   the adj-lang IDENT grammar; 6 normalized by hand. Fix: automated
   normalization, or instruct the deriver to emit grammar-valid atoms.
4. **Process: a leaked case name in the Agent `description` contaminated the
   first ingester run** (it referenced "McArdle"). Discarded + re-ran clean;
   recorded in `lessons.md`. Every subagent-facing field must be scrubbed.

**Net:** pipeline works end-to-end and gets novel cases right, but to BEAT a
strong base model it must add calibration + a coherent differential, not just
citations.

## 2026-06-03 — Batch 0b: full blind A/B on experiment-2

Ran all four arms on the experiment-2 case (known ground truth: PMBCL).
Framework arm = `adj52` runner over the ADJ51 rulebook; plain-Claude
control + blind judge run as real subagents. Artifacts in
`validation/experiment2-{plain-claude,framework-arm,judge}.json` and
`validation/experiment2-blind-ab.md`.

**Result: blind judge picked the framework (narrow).** Both arms got the
load-bearing decisions right (malignancy not infection, Actinomyces
colonizer, biopsy). Plain Claude *led* with myeloid leukemia — falling
for the paraneoplastic leukemoid trap; the framework avoided the wrong
specific commitment, which won it the edge.

**Two findings (the point of running it):**

1. **Methodology bug — the judge must receive the AUDIT TRAIL, not a
   prose summary.** The framework arm was rendered as prose that *claimed*
   citations without showing them; the judge correctly flagged "false
   rigor … can't be traced" and docked the framework on *defensibility* —
   its single biggest advantage. The orchestrator must feed the judge the
   runner's actual fired-clauses-with-citations
   (`05-run-output.txt`-style), or the experiment measures the framework
   with its defining feature amputated.
2. **Real calibration issue — the engine over-collapses to ~100%.** A
   ~100%/~99.9% posterior on a case that took weeks + a biopsy to resolve
   is overconfident. LR-aggregation produces extreme posteriors; the
   framework should carry residual uncertainty (an `uncertain`/kickback
   signal) here rather than collapse to certainty. Direct tie-in to the
   "uncertainty at the core" goal.

Both findings are now the top of the next-batch work list.

## 2026-06-03 — Batch 0: harness foundation + ingester validation

**Shipped & verified**

- **Spec** `ADJ52-autonomous-blind-cross-arm-experiment-loop.md` — the
  four-arm design, sandboxing model, IR contract (every byte typed or
  discarded-with-reason), recursive rulebook accretion with regression
  suite + anti-overfit rule, three gates, metrics (silent-discard rate,
  kickback precision/recall, rule-acquisition curve), and the cron loop.
- **Counterfactual / VOI / kickback runner** (`adj52-experiment` 0.1.0)
  — surfaces the engine's `uncertainties` that the ADJ51 runner threw
  away. Verified: on the demo fixture it reports "if biopsy malignant →
  99.5%, if benign → 7.9% (flips decision)" and a kickback band
  [0.079, 0.995] straddling 30% → ESCALATE. Confirmed strict superset of
  the ADJ51 runner (reproduces experiment-2 posteriors, panel omitted
  when no `uncertain` markers).
- **Four subagent prompt templates** (`prompts/`) — generic
  (domain-blind) ingester, recursive byte-provenanced deriver,
  plain-Claude control, blind judge.
- **Domain-blind ingester validation** (`validation/`) — ran a real
  ingester subagent on the experiment-2 sanitised prose. It inferred
  the domain unaided, covered the bytes (51 facts vs 47 reference), and
  — the key result — converted the case's genuine ambiguities into
  typed uncertainties (Actinomyces colonizer-vs-pathogen, weight-loss
  intentionality, leukocytosis interpretation) instead of hallucinating
  the "infection" answer that the ground truth flags as the naive trap.

**Failure modes / open issues observed**

1. **Sandbox is prompt-level only** — the ingester made (and abandoned)
   an accidental WebFetch. Need true filesystem isolation (worktree with
   only the input) before scaled/unsupervised runs.
2. **Ingester queries are human-readable, not engine-ready**
   (`what_is_the_unifying_diagnosis` vs `diagnosis(...)`). Need a
   normalization step before the deriver/vignette stage.
3. **Subagents prepend prose despite "JSON only"** — the orchestrator
   must extract the JSON object, not assume the whole message is JSON.

**Not yet done (next batches)**

- Deriver, plain-Claude, and judge arms not yet run live; only the
  ingester arm is validated.
- The three gates (round-trip ADJ04, semantic verifier rationale↔clause,
  regression suite) are specified but not wired into an orchestrator.
- No end-to-end 4-arm run yet; the orchestrator (Workflow script) is the
  next deliverable, to be validated first on experiment-2 (known ground
  truth) before any new case.
- Accretion + held-out split not yet exercised.
- Case acquisition + sanitisation leak-validator not yet built.

**Honest scope note.** Per-case live derivation is ~10 min (web
search); 1000 cases overnight is not physically reachable until the
ADJ51 indexed-source corpus exists. This batch validated the harness
foundation and the hardest single hypothesis (domain-blind ingestion),
rather than faking a large case count.
