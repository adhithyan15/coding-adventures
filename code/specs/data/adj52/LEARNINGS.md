# ADJ52 — running learnings log

Per-batch learnings from building and running the autonomous blind
cross-arm experiment loop. Newest first.

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
