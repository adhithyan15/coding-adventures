# Positioning memo — what's taken, what's open, how to position

Phase 0 deliverable (task #46). Based on a related-work scan, 2026-06-08. For each
contribution: the close prior art, the genuine gap, and the defensible framing.

---

## 1. Byte-stability (sampling self-consistency over verbatim source bytes)

**TAKEN — and heavily.** This is essentially **SelfCheckGPT** (Manakul et al., arXiv 2303.08896): "if an LLM has knowledge of a concept, sampled responses are consistent." Plus **semantic entropy** (Farquhar et al., *Nature* 2024) and **LLM-Check**. The core idea — resample, measure consistency, low consistency ⇒ hallucination — is established and crowded.

**Verdict: DEMOTE. Do not present byte-stability as a novel detector.** It will read as "SelfCheckGPT on verbatim passages."

**What's still open / how to position:**
- It's a **component**, not the contribution. Lead with the recursive stage-contract; byte-stability is *one gate* inside it.
- The genuine novelty is the **negative result**: byte-stability detects *invention* but NOT *relevance* or *stable-error* (the Watson–Crick "closing sentence"; the Palmyrene stable misparse). The consistency-detection literature frames consistency as a hallucination signal; we show its precise *failure boundary* and that you need a second (entailment) layer above it. That boundary result is publishable and is NOT in SelfCheckGPT.
- Requiring the model to emit **source bytes** (not just resample answers) is a stricter, verifiable variant — worth one sentence, not a section.

## 2. Provenance / citation / attribution

**TAKEN at the output level.** **ALCE** (Gao et al., arXiv 2305.14627) is the citation benchmark (recall/precision of citations). **VeriCite**, blueprint models (Fierro 2024), VTG (Sun 2024), and a 2025 survey ("Attribution, Citation, and Quotation") cover answer→source citation + verification. Documented citation-hallucination rates 11–57%.

**What's open / how to position:**
- Existing work cites at the **output level** (the answer cites sources). Nobody enforces **byte-accounting at *every stage*** with **no silent drops** and **justified discards**.
- **The novel move is provenance for the *negative space*:** existing work checks whether citations *support* what was *said* (inclusion). We additionally force justification of what was *ignored* (exclusion) — and argue (with the omission framing) that the ignored span is where the load-bearing error hides.
- Input-side + rulebook-side provenance, recursively, not just output-side.
- **Frame:** "Prior attribution work grounds what the model *says*; we grounds what it *ignores*, and enforce it recursively at every pipeline stage."

## 3. Hallucination-as-omission (the science / the mechanism)

**SUPPORTING, not scooping.** **Knowledge conflict** (EMNLP 2024 survey), **lost-in-the-middle**, parametric-vs-contextual interplay (ECIR 2025 keynote) all establish that models under-use context in favor of parametric priors. This literature is your **home** — it sets up the problem your contract attacks.

**What's open / how to position:**
- Nobody frames a **forced-justification-of-discards contract** as the *intervention* against context-under-utilization.
- **The isolating ablation is the novel empirical contribution**: bare / coverage-only / justified-discards × stratified (present-but-skimmed vs absent). Predict justified-discards ≫ coverage-only ≈ bare on the omission subclass. This is the experiment that converts the story into science.
- Caveat to respect (interpretability reviewers): frame as **information flow / context utilization**, not raw attention-as-importance ("Attention is not Explanation," Jain & Wallace 2019). The contract is an *output-level constraint that induces* context-faithful processing — not a weight intervention.

## 4. Process supervision / faithful CoT (the discipline)

**TAKEN.** "Let's Verify Step by Step" (Lightman et al., ICLR) + PRMs = supervise/score intermediate steps. Faithful-CoT work (Lanham; FaithCoT-Bench; "CoT Monitorability," 2025) shows free-text rationales often don't reflect the true computation.

**What's open / how to position:**
- Process supervision **scores** steps with a reward model; we make the process **byte-grounded and human-editable** — the attending edits a specific clause, not a scalar reward.
- **Faithful-CoT is your ammunition, not your competitor:** because free-text rationales are unreliable, *byte-grounded* provenance (cite the actual input bytes, checkable) is strictly stronger than "explain your reasoning." This is the one-line answer to "why not just prompt for an explanation."

## 5. Knowledge compilation / model editing (Paper 2)

**TAKEN (the foil).** **ROME** (NeurIPS'22), **MEMIT** (ICLR'23), **AlphaEdit** (ICLR'25) edit facts *in weights*. Critically: **"Understanding the Collapse of LLMs in Model Editing" (2406.11263)** documents that weight editing is brittle and collapse-prone. **IKE** and parameter-preserving editing store knowledge externally.

**What's open / how to position:**
- Model editing edits **weights** — opaque, collapse-prone, entangled, model-bound, hard to revert. **We edit an external, versioned, content-addressed, executable rulebook** — transparent, diffable, revertible, regression-gated, **model-decoupled**.
- **"Knowledge compilation"** (Darwiche & Marquis) is the precise established term: preprocess a KB into a form that makes queries tractable. We modernize it: compile a *byte-grounded* rulebook into a *CAS executable program*; the query is the input IR; reasoning runs on CPU.
- **Frame:** "We don't edit the model; we edit and compile the knowledge base the model reasons over — and cite the model-editing-collapse result as evidence that weights are the wrong place to put correctable knowledge."

## 6. LLM cascades / routing (the "small beats big" headline) — READ THIS

**TAKEN, and this is the most important finding for the headline.** **FrugalGPT** (Chen et al., **TMLR 2024**) already published: *"match the performance of the best individual LLM (GPT-4) with up to 98% cost reduction."* Plus AutoMix, C3PO, **Agreement-Based Cascading** (escalate on ensemble agreement — close to your byte-stability gate!), MixLLM, and a 2026 routing/cascading survey.

**Verdict: "small model matches big model at a fraction of cost" is NOT a novel headline. FrugalGPT owns it.** A bare "Haiku reaches Opus cheaply" claim will be desk-rejected as derivative.

**What's open / how to position — a DIFFERENT AXIS from cascades entirely:**
- Cascades / FrugalGPT measure **answer-correctness parity** at lower cost. We do **not** claim Haiku matches Opus on capability/accuracy — it doesn't, and we don't need it to.
- **The claim is defensibility-parity, not capability-parity:** *defensibility is a property of the verification discipline, not of model scale.* Under the framework, Haiku produces work whose **audit trail is as defensible/verifiable as Opus's** — the intelligence that makes work defensible lives in the discipline, so a small model inherits it.
- **Honest boundary (this is what makes it bulletproof):** Haiku and Opus differ on **coverage**, not on **defensibility-of-completed-work**. Where Haiku hits a capability ceiling it **abstains / kicks back** rather than fabricating; Opus reaches further. So the *accuracy/coverage* gap persists (honest); the *defensibility* gap closes to ~0; neither produces un-auditable work.
- **Why this isn't FrugalGPT's territory:** nobody measures **defensibility-parity across model scale** (the ADJ68 axis). FrugalGPT owns correctness-parity; this lane is uncontested.
- **The headline:** *"Defensibility is model-independent under the discipline: a small **local** model produces work as defensible and auditable as a frontier model — abstaining where it cannot ground rather than fabricating — enabling privacy-compliant (PHI-local) deployment."*
- **The experiment (2×2):** {Haiku, Opus} × {bare, +framework}, same items, blind adversarial auditor scoring **defensibility-fraction** + accuracy + abstention/kickback rate. Prediction: the framework collapses the Haiku↔Opus **defensibility** gap to ~0 while the **accuracy/coverage** gap persists and is absorbed by honest abstention.

---

## Net effect on strategy

1. **Lead Paper 1 with the recursive stage-contract + justified-discards (provenance for the negative space) + the omission ablation.** Byte-stability is a demoted component with a publishable *boundary* result.
2. **Lead Paper 2 with knowledge compilation to an editable, model-decoupled CAS library**, model-editing as the foil, and the **privacy-local deployment** as the deployment contribution.
3. **The "Haiku = Opus" number must be reframed**: not "cheap parity" (FrugalGPT) but **"cheap parity + auditable + PHI-local + knowledge-external."** Measure accuracy parity, cost, AND defensibility together; the headline is the conjunction, not the accuracy alone.
4. **Strong venue signal:** FrugalGPT is a **TMLR** paper. TMLR is the right primary target for both.
