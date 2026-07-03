# Paper 1 · E2 — the correctability study (cost-to-correct: framework vs prose)

> **Work item W2** (spec). The paper's **headline** experiment. Measures the thing no other line
> of work measures: when the model is wrong, **how cheaply can a reviewer locate the error, fix it,
> and have the fix persist** — for the byte-provenance framework vs. plain LLM prose.
> Skeleton: [`PAPER1-skeleton.md`](PAPER1-skeleton.md) · plan: [`PAPER1-WORKPLAN.md`](PAPER1-WORKPLAN.md).

## 1. Why this is the headline

The paper's thesis is a goal-shift: from **correctness** to **correctability** (resident → attending).
Accuracy benchmarks and hallucination *detectors* both stop at "is it right / is it suspicious." E2
measures what happens **after** a wrong answer: the **cost to correct it**. That axis is (a) the one
the framework is actually built to win, and (b) un-scoopable by accuracy or detection work. It also
operationalizes the governance claim — *human as auditor, not author* — as a number.

A correction has three stages; the framework should beat prose at each, and only the framework
supports the third:

1. **Localize** — find the exact load-bearing error (a premise/fact/inference).
2. **Fix** — change that one thing (override a fact / edit a clause), not rewrite the answer.
3. **Persist** — re-derive: the fix sticks for this case **and propagates** to every other case that
   cited the same fact (the *derive-once, reuse-indefinitely* payoff).

## 2. What already exists (this spec assembles it into one study)

| stage | prior ADJ result | what it showed |
|---|---|---|
| Localize | **ADJ96** | blind auditor pinpoints the error in the framework trail **5/5**, plain prose **2/5** — and on 2 items the *plain* prose **fooled the auditor into affirming the false premise** |
| Localize (scale) | **ADJ99 audit-trail** | cross-model Opus localizes the flaw in **90/100**; **52/100** trace to a specific fixable CAS fact |
| Fix + Persist | **ADJ-CAS edit-override-propagate** (`adj52/cas/`, PR #5233) | override one CAS fact (`overrides/…json`) → re-derive → answer corrects; *fix the fact, not the weight* |

E2 is the controlled, scored, n-powered study that turns these three islands into one
framework-vs-prose comparison with cost metrics and blind controls.

## 3. Research questions & hypotheses

- **RQ1 (localize).** Does the byte-provenance trail let a blind reviewer find the true error locus
  more often, and with less material inspected, than plain prose?
  **H1:** framework localization rate > prose; framework converts *buried fatal premises* into
  checkable lines (so prose's failure mode — auditor **affirms** the false premise — is rarer under
  the framework).
- **RQ2 (fix).** Given the locus, can the reviewer correct it with a **single, local** intervention?
  **H2:** framework correction = one CAS-fact override / one clause edit; prose correction has **no
  localized handle** — you must re-prompt or rewrite, which is not a *fix* of the artifact.
- **RQ3 (persist + propagate).** Does the fix survive re-derivation and **generalize** to held-out
  cases that depend on the same fact?
  **H3:** overriding a CAS fact corrects this case **and** every sibling case citing it, with **zero**
  new model calls at answer time; prose has nothing to propagate.

The honest null we must be able to report: if prose corrections are *just as cheap and persistent*,
the correctability thesis is weakened. The design must be able to show that.

## 4. Design

### 4.1 Arms (same items, same models)
- **A. plain prose** — model answers in free prose (the baseline; the artifact is the text).
- **B. framework** — byte-provenance pipeline: sourced CAS facts + cited reasoning chain; the
  artifact is the trail **plus the editable CAS store**.

Solver model held fixed across arms (run at both a cheap scale, Haiku, and a frontier scale, Opus,
to test whether correctability is model-independent — cf. the ADJ99 rescore mechanism).

### 4.2 Items
Reasoning-bound items where the error lives in an identifiable premise/fact (so a locus exists to
localize and fix):
- the **ADJ96 6-item** reasoning set (errors in the chain),
- the subset of **ADJ99** items whose flaw traced to a CAS fact (the 52% — natural fix targets),
- the **meningitis CAS case** (`adj52/cas/`) as the worked propagation example (one fact override
  fixes a family of cases).
Target n ≥ 30 wrong-answer cases (powering left to W-run; pilot may be smaller and labeled as such).
Stratify by domain and by *error class* (omitted fact vs bad inference vs mis-weighted premise).

### 4.3 The correction protocol (per item)
1. **Oracle locus** (scoring only): Opus + gold answer marks the true error locus. Never shown to
   the auditor.
2. **Blind localize**: a **blind auditor** (Opus as domain-competent reviewer, *no answer key*, told
   to audit not re-derive — the ADJ96 protocol; **same auditor on both arms** so the delta is the
   *artifact's* auditability) names the step/premise it believes is wrong. Score: hit / partial /
   miss vs oracle. Record **material inspected** (claims/bytes the auditor had to read to reach the
   locus) as the localize-cost proxy.
3. **Fix**:
   - framework: apply the minimal CAS override / clause edit at the located locus.
   - prose: apply the minimal textual correction a reviewer could make to the prose.
   Record **intervention size** (framework: # facts/clauses changed; prose: # claims rewritten — or
   "not localizable" if the fix requires rewriting the derivation).
4. **Persist / re-derive**: re-run the decision from the corrected artifact with **no new solver
   call** where the framework allows it (engine/program re-execution). Score: does the answer now
   match gold, and is it **stable** on a second re-derivation?
5. **Propagate**: run K held-out sibling cases that depend on the same corrected fact. Score:
   fraction corrected by the single override (framework) vs not-applicable (prose).

### 4.4 Metrics (formal)
- `localize_rate` = hits / n (per arm). Primary for RQ1.
- `auditor_fooled_rate` = cases where auditor affirmed a false load-bearing premise (per arm). The
  ADJ96 *qualitative* failure made quantitative.
- `inspect_cost` = mean material inspected to localize (per arm).
- `fix_locality` = fraction of fixes that are a **single** local override/edit (framework) vs
  fraction requiring derivation rewrite (prose).
- `persist_rate` = fraction where the fix yields gold **and** is stable on re-derive.
- `propagate_yield` = mean fraction of sibling cases corrected per single override; **answer-time
  model calls = 0** is the headline number for the derive-once claim.
- Report all with error bars (bootstrap CIs); pre-register the comparisons.

### 4.5 Blind controls & validity guards
- Same auditor model on both arms; auditor never sees the arm label or gold.
- **Format-confound guard** (from the ADJ99 rescore / `lessons.md`): before trusting any
  framework-vs-prose auditor delta, run the deterministic leak check (can a regex tell the arm from
  the artifact?) and, where the comparison is judge-scored, **normalize presentation** so the
  auditor scores substance, not the citation-shaped format. *This is mandatory here* — E2 is exactly
  an arm-vs-arm judged comparison, the configuration that bit ADJ99.
- Oracle/auditor separation; an item whose gold is itself wrong (cf. ADJ96 `integral` 5482) is
  dropped and logged, not scored.
- Single-judge caveat: confirm localize/persist deltas with a second, non-Opus auditor on a subset
  (shares infra with W5).

## 5. What counts as the headline result

A figure with three panels — **localize**, **fix-locality**, **propagate** — showing the framework
strictly dominates prose, with the propagate panel showing **N sibling cases corrected by one fact
override at zero answer-time model cost** while prose sits at zero (nothing to propagate). Plus the
qualitative money quote: the ADJ96 *divisors* case, where the framework chain exposed
"`f` is multiplicative" as a checkable line the auditor refuted, while the prose buried it and the
auditor **affirmed the false premise**. That single contrast is the paper in one example.

## 6. Threats specific to E2
- **Prose strawman.** The prose baseline must be a *strong* one (best-effort prose, same model,
  allowed to show work) — not a deliberately terse answer. Otherwise the localize delta is unfair.
- **Auditor capability ceiling.** A weak auditor can't localize in either arm; an oracle-strong
  auditor may localize even in prose. Report across auditor strengths; the *interesting* regime is a
  competent-but-not-omniscient reviewer (the real attending).
- **Fix cherry-picking.** "Minimal fix" must be defined mechanically (smallest CAS/clause delta that
  flips the verdict), not chosen post-hoc by us.
- **Propagation realism.** Sibling cases must genuinely depend on the overridden fact; document the
  dependency, don't assert it.

## 7. Build order (the W-run that follows this spec)
1. Harness: load wrong-answer cases (ADJ96 set + ADJ99 CAS-fix subset + meningitis family).
2. Localize pass (blind auditor, both arms, same as ADJ96; add `inspect_cost`).
3. Fix + re-derive (framework: CAS override via `adj52/cas/override.py` pattern; prose: minimal text edit).
4. Propagate (sibling cases through the corrected CAS; count answer-time model calls = 0).
5. Aggregate with bootstrap CIs; format-confound guard; second-auditor subset.
Output: `code/specs/data/e2-correctability/` (harness + raw + FINDINGS), mirroring the ADJ run layout.
