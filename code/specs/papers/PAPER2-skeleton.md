# Paper 2 skeleton — Knowledge Compilation for Auditable Adjudication: CAS-Cached, Human-Correctable, Privacy-Local Rulebooks

**Target:** TMLR (primary) or a systems venue. **Depends on:** Paper 1 preprint being out. **Status:** skeleton.
**Worked anchor for E1+E2:** [`PAPER2-MYCIN-derive-once-proof.md`](PAPER2-MYCIN-derive-once-proof.md) —
MYCIN-2026 as the concrete derive-once / reuse-indefinitely / CPU-offload proof.

## One-sentence claim
Byte-grounded rulebooks compile **once** into content-addressed (CAS) executable libraries that future cases query on **CPU with zero answer-time model calls**; because every compiled rule cites verified source bytes, the cache is trustworthy, **human-correctable** (edit a rule → new CAS version, regression-gated), and **model-decoupled**; and the derive-offline / execute-local split makes the system **privacy-compliant (PHI-local) by construction.**

## Framing arc
- **The cache is institutional memory of corrections, not a speed hack.** When the attending corrects the resident, the resident doesn't make that mistake again. The CAS does the same: a corrected, byte-grounded rule is versioned and reused; the system carries forward every correction, auditably.
- **Provenance is the precondition for trust-free reuse.** You can only run a cached rule on CPU without re-invoking the model because the rule traces to verified bytes.
- **The privacy boundary aligns with the architecture's natural split.**

## Contributions (C1–C4)
- **C1.** **Knowledge compilation for LLM reasoning:** compile a byte-grounded rulebook into a CAS executable library; input IR → deterministic program that imports the library; reasoning is CPU-bound. *(Darwiche/Marquis knowledge compilation, modernized.)*
- **C2.** **External, editable, versioned corrections vs. weight editing:** human override = a provenance-tagged assertion (`human_override(editor, ts, justification, authority, scope)`); committed only if conflict-checked + **regression-gated** (CI for the knowledge base). Contrast with ROME/MEMIT (opaque, collapse-prone, model-bound). *(belief revision / TMS for the conflict logic.)*
- **C3.** **Model-decoupling:** corrections persist across a model swap (knowledge in the artifact, not the weights). The experiment cascades/model-editing can't run cleanly.
- **C4.** **Privacy-local deployment:** frontier model derives+compiles offline on *public* sources (never sees PHI); small local model ingests the case → byte-accounted IR; deterministic compiler emits the program; CPU executes locally; coverage-gaps kick back to a human. The decision **replays locally and deterministically** from local artifacts.

## Experiments
| # | experiment | status | task |
|---|---|---|---|
| E1 | CAS knowledge-compilation benchmark: cold-vs-warm cost, accuracy parity (compiled-CPU vs LLM-answer-time), cache hit-rate / generalization, ≥2 domains | EXTEND ADJ71 (one slice done) | #52 |
| E2 | Correction-persistence + **model-swap durability** (edit persists; swap model → corrections survive) | TO BUILD | #52 |
| E3 | **Privacy-local deployment**: frontier-offline-derive + local-Gemma-ingest + deterministic-emit + CPU-exec; extraction fidelity, parity vs frontier, coverage-gap kickback | TO BUILD | #53 |
| (E0) | Narrow CAS program-cache slice (8 U.S.C. 1427 naturalization) | DONE (ADJ71) | — |

## Related work (positions per memo)
Knowledge compilation (Darwiche & Marquis); model editing ROME/MEMIT/AlphaEdit + **the editing-collapse result** (foil); IKE / parameter-preserving editing; belief revision / truth-maintenance (corrections + conflict); **FrugalGPT (TMLR 2024) + cascades/routing** — the critical foil for the cost headline.

## The headline — DEFENSIBILITY-parity, not capability-parity (see memo §6)
**Do NOT claim "Haiku matches Opus on accuracy" — that's FrugalGPT's axis (TMLR 2024), and it isn't even true.** The claim is on a different, uncontested axis:
> **Defensibility is a property of the verification discipline, not of model scale.** Under the framework, a small **local** model (Haiku) produces work whose audit trail is **as defensible and auditable as a frontier model's (Opus)** — abstaining/kicking-back where it cannot ground rather than fabricating — enabling PHI-local deployment.

**Honest boundary:** the accuracy/coverage gap persists (Haiku abstains where Opus completes); the **defensibility** gap closes to ~0; neither produces un-auditable work.

**The 2×2 experiment:** {Haiku, Opus} × {bare, +framework}, same items, blind adversarial auditor scoring **defensibility-fraction** + accuracy + abstention/kickback rate. Prediction: the framework collapses the Haiku↔Opus *defensibility* gap to ~0 while the *accuracy/coverage* gap persists and is absorbed by honest abstention. Reframe vs. cascades: they measure correctness-parity and *gamble* on the cheap model; we measure *defensibility*-parity and *verify* it.

## Threats to validity
Rule-interaction consistency at scale (belief-revision is hard; conflict-detection + regression-gate is partial); local-extraction fidelity on real inputs; library coverage gaps (must kick back, never silently proceed); liability/governance (deployment barrier the audit trail mitigates but doesn't remove).

## Reproducibility / ethics
Public sources only (statutes, literature, court opinions) — **no real PHI**; the HIPAA discussion concerns the deployment *architecture*, not the experiments. CAS hashes + generated libraries + programs committed as artifacts.
