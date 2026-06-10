# Paper 1 skeleton — Byte-Recursive Provenance for Auditable, Correctable LLM Adjudication

**Target:** TMLR (primary), workshop for early feedback. **Status:** skeleton.
**Remaining work + live experiment status:** see [`PAPER1-WORKPLAN.md`](PAPER1-WORKPLAN.md)
(plan-of-record; reconciles this skeleton with the ADJ73–100 corpus and the ADJ99 rescore).

## One-sentence claim
A recursive byte-accounting contract — every byte of every artifact, at every pipeline stage, is accounted-for or discarded-with-a-justification; every extracted fact is anchored to source bytes; every rule is verified to be entailed by its cited bytes — turns LLM adjudication into work that is **auditable** (every link traces to bytes) and **correctable** (errors localize to a clause), and its central mechanism is that **forced justification of discards attacks omission-class hallucination.**

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

| # | experiment | status | evidence / next |
|---|---|---|---|
| (E0) | Two worked HLE runs (Palmyrene stable-error; hummingbird unstable-gap) | **DONE** | ADJ72 |
| E1 | Mechanistic ablation (justified-discards is the lever), open-weights Ollama, stratified items | **PILOT (mixed)** | `adj73-omission-ablation/`; needs confirmatory run + abstention gate (W4) |
| E2 | Correction-loop study (localize + fix + persist; framework vs prose) — *correctability headline* | **PARTS ONLY** | ADJ96/97/98/99 audit-trail + ADJ-CAS edit-override; assemble cost-to-correct study (W2) |
| E3 | Cross-domain matrix (same machinery, N domains, only rulebook changes) | **CONSOLIDATE** | ADJ49/56/59/70/71 + clinical → one artifact (W3) |
| E4 | HLE/defensibility screening harness (blind controls; correctness + defensibility + error bars; cross-model byte-stability) | **DATA IN; metric corrected** | ADJ87–100; rescore PR #5261; needs cross-judge robustness (W5) |

## Related work (positions per memo)
SelfCheckGPT / semantic entropy (C4 demotion); ALCE / VeriCite / attribution survey (C1–C2 contrast); knowledge-conflict / lost-in-the-middle / parametric-vs-contextual (C3 home); process supervision / PRMs + faithful-CoT (the discipline; byte-grounding as answer to CoT-unfaithfulness); Attention-is-not-Explanation (frame C3 as information-flow, not attention weights).

## Threats to validity
Contamination (public benchmarks in training data → use less-contaminated/held-out items, state it); single model family (add cross-model arm); the stable-error-with-no-flagged-neighbor hole; n / error bars; correctness scoring protocol.

## Reproducibility
Clean repo + fixed prompts + one-command harness; every quantitative claim traces to a repo artifact (byte-provenance applied to the paper itself). Public data only — no real PHI; no IRB needed.

## Headline candidate
**E2 (correctability)** is likely the most persuasive — it measures the thing nobody else measures (cost-to-correct), and it can't be scooped by accuracy or hallucination-detection work.
