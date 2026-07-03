# ADJ52 — Autonomous Blind Cross-Arm Experiment Loop

> **Headline.** ADJ45 proved, with a blind judge, that the framework's
> resolution loop beats raw prompting on QA benchmarks (SimpleQA
> hallucination 47% → 6%). ADJ51 proved the byte-recursive provenance
> contracts work end-to-end on two real clinical cases. ADJ52 fuses
> the two and **industrialises** them: a single autonomous loop that,
> per case, runs a generic (domain-blind) ingester → IR, an IR-driven
> recursive rulebook deriver, a compiled engine query with
> **first-class counterfactuals**, a plain-Claude control arm, and a
> **blind judge** that scores framework-vs-plain against ground truth
> without knowing which is which. The rulebook **accretes** across
> cases under a regression suite, so the corpus compounds. The goal is
> not a better code generator — it is to make the reader (Claude, or
> any LLM) **constitutionally unable to glide past a byte**: every byte
> of the problem statement is either typed into the IR or discarded
> *with a recorded reason*, and every decision-relevant uncertainty is
> resolved with tools or escalated, never silently guessed.

## Why this experiment

Hallucination is selective attention: the model locks onto the parts
of an input it recognises and silently resolves whatever it skipped by
confident assumption. The framework denies the skip. This spec turns
that principle into a measurable, repeatable, cross-domain loop and
asks four questions:

1. **Does forced byte-disposition reduce silent assumptions?** Measured
   as *silent-discard rate* — the fraction of discarded/collapsed input
   spans that carry no reason or an un-checkable rubber-stamp reason
   that round-trip can't defend. Target: the ADJ45 ~6% analog.
2. **Is the escalation calibrated?** The load-bearing capability is
   knowing what to hand a human. Measured as *kickback precision/recall*
   against a held-out "should-escalate" label — not raw accuracy.
3. **Does the rulebook compound?** Measured as the *rule-acquisition
   learning curve* — new clauses added per case over the sequence. The
   working hypothesis is saturation: after N cases the rulebook holds
   ~80% of the rules a domain ever needs, with the residual fetched on
   demand (the resident-consults-UpToDate move).
4. **Does the framework beat plain Claude where it matters?** Measured
   by a blind judge over framework-vs-plain output against ground truth,
   exactly as ADJ45, now on a byte-provenanced pipeline.

The defining property under test is **diagnosable wrongness** (ADJ51):
when the framework is wrong, the failure traces to a specific clause,
rationale, and source byte-span. Correctness comes and goes with
rulebook quality; diagnosability is the architecture.

## Scope

**Adjudicative** knowledge work — work that produces a defensible
judgment on a well-posed query against criteria that, given the facts,
largely *determine* the verdict (clinical, legal, financial, audit,
compliance, scientific evaluation, eligibility). The litmus: *would two
competent experts, given the same facts and the same rulebook, reach
the same verdict?* Where they would legitimately diverge, the framework
produces a defensible **map of the disagreement** (source-disagreement
+ kickback), not a false verdict. Explicitly out of scope: generative
creative work and open-ended hypothesis generation — though the same
reader can *audit* such artifacts even where it cannot *produce* them.

## The per-case pipeline (four arms, two sandboxed)

```text
                        ┌──────────────────────────────────────┐
  published case  ──▶   │ MAIN AGENT (holds ground truth)      │
                        │  split: 00-ground-truth / 01-prose    │
                        │  sanitise: no answer-leak in prose    │
                        └───────────────┬──────────────────────┘
                                        │ 01-prose (inline)
            ┌───────────────────────────┼───────────────────────────┐
            ▼                                                         ▼
  ┌───────────────────────┐                              ┌──────────────────────┐
  │ ARM 1 — FRAMEWORK     │                              │ ARM 2 — PLAIN CLAUDE │
  │                       │                              │  case presented      │
  │ ingester (SANDBOXED)  │── 02-ingestion.json          │  plainly; answer +   │
  │  domain-blind, web-ok │                              │  reasoning recorded  │
  │       │               │                              │  → 06-plain.json     │
  │   [GATE: round-trip]  │  ADJ04 entailment            └──────────┬───────────┘
  │       ▼               │                                         │
  │ deriver (SANDBOXED)   │── 03-rulebook.adj (recursive,            │
  │  IR-driven, web-ok    │   byte-provenanced, accretes)           │
  │  [GATE: semantic ver] │  rationale↔clause                       │
  │  [GATE: regression]   │  re-run all prior cases                 │
  │       ▼               │                                         │
  │ compile + query       │── 05-framework.json                     │
  │  posteriors +         │   (posteriors, counterfactuals,         │
  │  COUNTERFACTUALS +    │    kickback, source-disagreement,       │
  │  kickback             │    coverage)                            │
  └───────────┬───────────┘                                         │
              └───────────────────────┬─────────────────────────────┘
                                      ▼
                        ┌──────────────────────────────────────┐
                        │ ARM 3 — BLIND JUDGE (SANDBOXED arm)   │
                        │  inputs A/B RANDOMISED + anonymised   │
                        │  + ground truth; scores both; keymap  │
                        │  held by main agent → 07-judge.json   │
                        └──────────────────────────────────────┘
```

### Sandboxing model

The ingester and deriver are **sandboxed by construction**: they
receive their input *inline from the main agent* (the sanitised prose,
or the ingestion JSON), never a path to ground truth. They MAY use
`WebSearch`/`WebFetch` for uncertainty disambiguation. They MUST NOT
read local repository files to discover the real answer. The main
agent holds `00-ground-truth.txt`; the judge receives ground truth but
the two arm outputs **anonymised and randomised** (labelled A/B) with
the keymap retained only by the main agent — so the judge cannot know
which output is the framework and which is plain Claude.

Stronger filesystem isolation (run each sandboxed subagent in a
worktree containing only its input) is a hardening follow-up; v1
enforces the sandbox through inline-input + explicit prohibition in the
subagent prompt.

## The IR contract — generic, domain-inferred

The ingester is **never told the domain**. It reads the prose, infers
the domain, and decomposes to a human-readable IR. Shape is flexible;
the only hard invariants are:

- **Every byte gets a disposition.** Not "every byte becomes content" —
  human communication is redundant and compressible. Every span is
  either typed into the IR *or* `discarded` with a reason. Silent
  omission is the one thing forbidden.
- **Discard reasons are typed by checkability:**
  - *Structural, self-checking* — `redundant_with(span)`,
    `restatement_of(span)`, `boilerplate`, `affective_framing`,
    `formatting`. Mechanically verifiable; round-trip is the safety net.
  - *Judgment* — `not_relevant_to_query`. **Not** mechanically
    checkable; these route to the escalation surface, not waved through.
- **Ambiguity becomes an `uncertainty`, never a guess.** An
  underspecified or ambiguous span is classified as an uncertainty with
  a domain of candidate readings — then resolved with tools or
  escalated. This is the anti-hallucination core.
- **Facts + queries drive everything downstream.** The IR's facts and
  the queries it raises determine which rulebook is needed.

A representative (non-binding) node taxonomy for adjudicative reading:
`Fact / Query / Uncertainty / Constraint / Reference-to-verify /
Discarded(reason)`. The SWE/spec-reading instantiation swaps in
`Requirement / Constraint / Example-or-TestCase / Reference-to-verify /
Ambiguity / OutOfScope`. The engine cares only about facts, queries,
and uncertainty markers; the rest is human-readable scaffolding.

## Recursive rulebook derivation

The deriver consumes the IR and produces an `adj-lang` rulebook:

- **IR-driven.** The facts present and the queries raised determine the
  rulebook. If no rulebook exists for the inferred domain, derive one.
  If one exists (accretion), decide what rules to *add*.
- **Recursive into subtypes.** A query over a category (e.g. a parent
  diagnosis) recurses into sub-queries with independent priors so a rule
  attaches to the correct scope. This is the fix for ADJ51 experiment
  1's troponin failure (a parent-scoped rule that should have been
  sub-type-scoped). The deriver may introduce sub-queries on its own.
- **Byte provenance on every clause.** Each `prior`/`contributes`/
  `interacts` clause carries `source`/`locator`/`trust` annotations and
  is immediately preceded by a `% rationale` block (ADJ51 contract).
- **Decision-relevant uncertainties become `uncertain { … } for …`
  markers** so the engine emits VOI reports and the runner can show
  what resolving them would shift.
- **Anti-overfit rule.** A clause is admitted only when a *citable
  source* justifies it — never because it made a case pass. The case
  *triggers* the lookup; the source *authorises* the rule.

## The three gates

1. **Round-trip entailment (ADJ04).** Render the IR back to text; check
   it entails the source prose. This is the automatic safety net on
   discards: if a load-bearing span was wrongly discarded as
   "boilerplate," the re-rendered IR fails to entail the source and the
   bad discard surfaces — no one has to read the discard reasons.
2. **Semantic verifier (rationale↔clause).** For each `(rationale,
   clause)` pair, confirm the clause does what the rationale claims.
   This would have caught the troponin bug. Structural precondition
   (every clause has a rationale) is `validate_rulebook.py`; the
   semantic check is a separate verifier subagent.
3. **Regression suite.** Every rulebook accretion is re-run against all
   prior cases' vignettes. An accretion that regresses a prior verdict
   is rejected or flagged. The corpus of past cases *is* the rulebook's
   test suite — the second compounding asset alongside the rulebook.

## Counterfactuals / VOI / kickback as first-class output

The engine already provides the machinery
(`logic-engine::lr_aggregate`): `LRAggregateResult.uncertainties`
(per-marker `UncertaintyReport` with per-value `if_observed_logit_delta`
and `voi_logit_range`), `suggest_kickback(threshold)` (a lo/hi
posterior band that, when it straddles the threshold, recommends
resolutions ranked by VOI), `counterfactual(query, kb, assumed_facts)`,
and `source_disagreements(kb, conclusion)`. ADJ51's runner destructured
`uncertainties` and **threw it away**. ADJ52's runner surfaces all of
it as a per-query panel:

```text
Counterfactual sensitivity (query Q):
  uncertainty: <conclusion> over { v1, v2, … }   VOI range = R logits
    if observe v1:  P 0.82 → 0.95
    if observe v2:  P 0.82 → 0.30   ← flips decision at threshold
  kickback: band [0.30, 0.95] straddles 30% → ESCALATE; resolve [marker …]
  source-disagreement: evidence E — AHA LR 2.5 vs ESC LR 4.0 (range 0.47)
```

This is the part of the thesis ("the output calls out what
uncertainties remain and what would shift") that was specified but not
shipped. It is the cheapest high-fidelity win and the *acquisition
trigger* for rulebook growth: the top-VOI gap is exactly the rule the
deriver should go look up next.

## Metrics

| Metric | What it measures | Target / use |
|---|---|---|
| **silent-discard rate** | discards with no reason / un-checkable reason that round-trip can't defend | drive toward ADJ45's ~6% |
| **observation/requirement coverage** | fraction of source constraints captured vs silently missed | recall on the reading |
| **hallucinated-reference rate** | IR references (symbols, citations, entities) that don't exist | ~0 (mechanically checkable) |
| **kickback precision / recall** | escalations vs a held-out should-escalate label | the calibration result |
| **blind-judge win rate** | framework vs plain Claude against ground truth | the ADJ45 analog |
| **rule-acquisition curve** | new clauses per case over the sequence | saturation point (~80% claim) |
| **held-out generalisation** | accrete on 1..k, freeze, score k+1..n | memorisation vs learning |

## The autonomous loop

Per `lessons.md`: autonomous loops use **`CronCreate`, not
`ScheduleWakeup`**, driven by a `.claude/adj52-state.json` work-queue
state file (`{ case_id, status: pending|in-progress|run|pr-open|merged
}`), a 3-minute recurring cron with directive language, babysitting
open PRs, and self-deletion when the queue drains. Cadence: **gather
data and open a PR every ~100 experiments** (or at milestone
boundaries when per-case cost makes 100 impractical in a window). Each
PR carries the batch's results + a clustered **failure taxonomy**
(ingestion drops, rationale↔clause mismatch, global coverage gaps,
sub-type-of-category errors, calibration misses) — the loop is a
failure-mode discovery engine, not just an accuracy run. No fabricated
counts: every number ships from a real run.

## Implementation sequence

1. **Counterfactual/VOI/kickback panel in the runner** (standalone
   crate; consumes existing engine APIs; no shared-crate edit). Unit-
   tested on a synthetic rulebook with `uncertain {}` markers; smoke-run
   on ADJ51 experiment 2. *First deliverable.*
2. **Generic ingester + deriver + plain-Claude + judge subagent
   prompts**, plus a Workflow/orchestrator that runs one case end-to-end
   through all four arms. Validated first on ADJ51 experiment 2 (known
   ground truth + already-sanitised prose).
3. **The three gates** wired into the orchestrator.
4. **Accretion + regression** harness (corpus-as-test-suite, cite-
   source-not-case, held-out split).
5. **Case acquisition + sanitisation** with a leak validator, then scale
   via the cron loop, PR per batch.

## What ADJ52 ships (incrementally)

- This spec.
- `code/specs/data/adj52/` — standalone runner crate extending ADJ51's
  with the counterfactual/VOI/kickback panel, subagent prompt
  templates, orchestrator, gates, and per-case artifact directories.
- Per-batch PRs with results + failure taxonomy + learnings.

## What ADJ52 does not yet do

- **Throughput at 1000 scale.** Per-case live deriver runtime is
  ~10 min (web search); 1000 cases overnight is not physically
  reachable until the ADJ51 indexed-source corpus is built. ADJ52
  validates the harness and scales as far as data + time allow.
- **Indexed-source corpus.** Still live WebSearch per case; the
  pre-indexed claim corpus (ADJ51 sketch) that makes derivation ~seconds
  and reproducible is a separate track.
- **Filesystem-level sandbox.** v1 sandboxes via inline-input +
  prohibition; worktree isolation is a hardening follow-up.
- **Semantic verifier as a standing subagent.** Structural validator
  ships first; the rationale↔clause semantic check follows.

## Status

- 2026-06-03: ADJ52 spec authored on branch
  `adj52-blind-cross-arm-experiment`. Implementation in progress,
  starting with the counterfactual/VOI/kickback runner panel.

## See also

- [ADJ45](ADJ45-three-way-blind-judge-experiment.md) — the blind-judge
  design ADJ52 industrialises and extends to a byte-provenanced pipeline.
- [ADJ51](ADJ51-byte-recursive-provenance.md) — the byte-accounting
  contracts and the generic ingester/deriver pipeline ADJ52 automates.
- [ADJ48](ADJ48-mycin-2026-in-adj-lang.md) — decision-relevant vs
  load-bearing uncertainty; the kickback semantics ADJ52 surfaces.
- [ADJ18](ADJ18-active-sensing-voi.md) — value-of-information, the
  engine-layer enabler of the counterfactual sensitivity panel.
