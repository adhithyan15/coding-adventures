# Paper 1 skeleton — Byte-Recursive Provenance for Auditable, Correctable LLM Adjudication

**Target:** TMLR (primary), workshop for early feedback. **Status:** skeleton.
**Remaining work + live experiment status:** see [`PAPER1-WORKPLAN.md`](PAPER1-WORKPLAN.md)
(plan-of-record; reconciles this skeleton with the ADJ73–100 corpus and the ADJ99 rescore).

## One-sentence claim
A recursive byte-accounting contract — every byte of every artifact, at every pipeline stage, is accounted-for or discarded-with-a-justification; every extracted fact is anchored to source bytes; every rule is verified to be entailed by its cited bytes — turns LLM adjudication into work that is **auditable** (every link traces to bytes) and **correctable** (an error reduces to an *editable fact or clause* whose fix **persists under deterministic re-derivation and propagates to every dependent case**), and its central mechanism is that **forced justification of discards attacks omission-class hallucination.**

> **Correctability, sharpened by E2.** Correctability is *not* "a reviewer reads the trail and spots the bug faster" — E2 found that a strong reviewer localizes errors in plain prose just as well (a clean null, after the format-confound guard). It is **non-recurring cost-to-correct**: the fix lives in an editable artifact, so it is paid **once** and propagates, where plain prose (stateless, no persistence layer) re-incurs the same error on every future case. That is the axis the framework wins, and it is what E2 measures.

## Framing arc (intro → discussion)
- **Motivation:** resident vs. attending. The fix for a wrong resident is not a smarter resident; it's showing work so the attending can find the omitted fact / bad inference and correct *that*, cheaply. Current LLMs emit untraceable prose — un-correctable. Shift the goal from **correctness** to **correctability**.
- **History:** MYCIN worked but died on build/maintain/trust, not accuracy. LLMs dissolve the knowledge-acquisition and natural-language bottlenecks — but introduce a new one, unreliable knowledge. Byte-recursive provenance is how you pay that new cost.
- **Scope:** adjudicative knowledge work (apply criteria to evidence for a defensible judgment). Inclusion test stated; out-of-scope named.

## Contributions (C1–C4)
- **C1.** The recursive byte-accounting **stage contract** (formal): input→IR, rulebook→IR, decision, each gated; no silent drops; discards justified; facts byte-anchored; rules entailment-checked. *(vs. output-only citation — §2 of memo.)*
- **C2.** **Provenance for the negative space:** forced justification of discards, and the finding that the ignored span is where omission-hallucination hides. *(novel vs. attribution literature.)*
- **C3.** **Hallucination-as-omission** mechanism + the isolating ablation (bare / coverage-only / justified-discards × present-but-skimmed / absent). *(the science.)*
- **C4.** **Byte-stability boundary result** (demoted): sampling-consistency over verbatim source bytes detects *invention* but not *relevance/stable-error*; a second entailment layer is required. *(honest extension of SelfCheckGPT/semantic-entropy.)*

## Experiments
Status reconciled 2026-06-09 against the ADJ73–100 corpus (the original `#47–#50` pointers
were stale). Full detail + remaining work in [`PAPER1-WORKPLAN.md`](PAPER1-WORKPLAN.md).

Status reconciled again 2026-06-10 after the E2 run, the 200-item benchmark (run100 + run100b), and
the E4 cross-judge. All four experiments now have their load-bearing data; the only deferred item is
E1's frontier ablation tier (H4), placed in Limitations.

| # | experiment | status | evidence / next |
|---|---|---|---|
| (E0) | Two worked HLE runs (Palmyrene stable-error; hummingbird unstable-gap) | **DONE** | ADJ72 |
| E1 | Mechanistic ablation (justified-discards is the lever) + abstention gate | **PILOT + at-scale corroboration** | `adj73-omission-ablation/` (lever, ≤3B); abstention corroborated by E3's 200-item benchmark; capability-floor by E2; **H4 frontier tier deferred to Limitations** (E1 §8) |
| E2 | Cost-to-correct, framework vs prose — *correctability headline* | **DONE** | `e2-correctability/`: **find = null** (strong reviewer reads prose equally well), **fix/propagate + recurrence = the win** (1.5B raw 0/7 → +framework 7/7, capability-graded; meningitis/TAX derive-once) |
| E3 | Cross-domain matrix (same machinery, N domains, only rulebook changes) | **DONE** | 200-item pre-registered benchmark across 20 domains (run100 + run100b): byte-accounting 100/98, no hallucinated rules 100/100, reflexive gold audit; + ADJ49/56/59/70/71 ballast (W3) |
| E4 | HLE/defensibility screening (blind controls; corrected locus-exposure metric; cross-judge) | **DONE** | ADJ99 rescore (PR #5261) + Sonnet cross-judge (within-1 96.7%, r=0.79, opus gap +0.45 under both judges) |

## Related work (positions per memo)
SelfCheckGPT / semantic entropy (C4 demotion); ALCE / VeriCite / attribution survey (C1–C2 contrast); knowledge-conflict / lost-in-the-middle / parametric-vs-contextual (C3 home); process supervision / PRMs + faithful-CoT (the discipline; byte-grounding as answer to CoT-unfaithfulness); Attention-is-not-Explanation (frame C3 as information-flow, not attention weights).

## Threats to validity
Contamination (public benchmarks in training data → use less-contaminated/held-out items, state it); single model family (add cross-model arm); the stable-error-with-no-flagged-neighbor hole; n / error bars; correctness scoring protocol.

## Reproducibility
Clean repo + fixed prompts + one-command harness; every quantitative claim traces to a repo artifact (byte-provenance applied to the paper itself). Public data only — no real PHI; no IRB needed.

## Headline candidate
**E2 (correctability)** remains the most persuasive — it measures the thing nobody else measures
(**non-recurring cost-to-correct**), and it can't be scooped by accuracy or hallucination-detection
work. Its strength is now *sharpened by honesty*: E2 concedes the localize-by-reading null (closing
the obvious reviewer attack) and rests the headline on the fix→persist→propagate machinery and the
capability-graded recurrence result (1.5B raw 0/7 → +framework 7/7) — *intelligence accumulates in
the framework, not the weights*.
