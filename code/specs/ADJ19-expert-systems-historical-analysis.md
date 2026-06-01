# ADJ19 — What Killed Expert Systems in the 90s and Why This Framework Doesn't Inherit Those Bugs

> Not a specification — a *contextual analysis*. The adjudication
> framework's design choices look a lot like 80s/90s expert systems
> at first glance, and reviewers will spot the resemblance
> immediately. This document names the eight failure modes the
> classical literature documented, maps each to a specific
> architectural choice in this framework, and is honest about which
> ones remain open. Reads as the literature-review section for a
> publishable paper on probabilistic adjudication.

## Why this matters

Between roughly 1980 and 1995, expert systems were one of AI's
flagship technologies. Edward Feigenbaum coined "knowledge is
power"; DEC's R1/XCON saved $40M/year configuring VAX systems
([McDermott 1982](#citations)); MYCIN matched human experts on
blood-infection diagnosis ([Buchanan & Shortliffe 1984](#citations));
PROSPECTOR found a $100M molybdenum deposit at Mt. Tolman ([Duda et
al. 1976](#citations)). Then, somewhere between 1988 and 1995, the
whole field collapsed. The follow-up books were titled things like
*The Brittleness Bottleneck* and *Why Expert Systems Failed*. By
2000 "expert system" was a slur in AI research.

The collapse had specific, well-documented causes. **Every cause
maps to an architectural decision the failed systems made — and
every one is either repeated, fixed, or partially fixed by this
framework.** A paper that proposes a "Prolog-style logic engine
with LLM extraction" without addressing the eight failure modes
below will get reviewed as if it's a 1985 PhD thesis with a fancy
front-end. So the framework's structural choices need to be
defensible against each failure mode explicitly.

This document does that defense.

## The eight failure modes — at a glance

| # | Failure mode                                | Classical example | Framework's response | Status |
|---|---------------------------------------------|-------------------|----------------------|--------|
| 1 | Knowledge acquisition bottleneck            | MYCIN, CADUCEUS   | LLM extraction + ADJ02 total coverage forces every byte explicit | **Structurally fixed** |
| 2 | Knowledge maintenance crisis                | R1/XCON           | Rules cite source spans; audit trail makes provenance machine-readable | **Structurally fixed** |
| 3 | Closed-world brittleness                    | All rule-based ES | ADJ14 LR aggregation; `Uncertainty` nodes; active sensing (ADJ18) | **Partially fixed** |
| 4 | Combinatorial rule explosion                | R1/XCON at scale  | LR aggregation O(n) in observed evidence; indexed retrieval (deferred) | **Architecturally fixed; empirically untested** |
| 5 | Calibration absence (MYCIN's CFs)           | MYCIN, EMYCIN     | LP19e rigorous log-odds Bayesian inference; ADJ14 explicit independence assumption | **Structurally fixed** |
| 6 | Explanation inadequacy                      | MYCIN, GUIDON     | ADJ16 derivation rendering with span citations | **Architecturally fixed; rendering is mechanical not domain-intuitive** |
| 7 | Validation crisis                           | All ES literature | ADJ12 structured benchmarks; audit-trail replay | **Partially fixed; gold-label problem unchanged** |
| 8 | Knowledge engineer dependency               | All deployments   | LLM as the knowledge engineer; ADJ06 clarification closes the loop | **Structurally fixed; prompt engineering is the new specialized role** |

The rest of this document walks each row.

## 1. Knowledge acquisition bottleneck

### What went wrong

Edward Feigenbaum ([1977](#citations)) named the central problem:
"knowledge is power." Translating that knowledge from human experts
into machine-executable rules turned out to be the hard part, not
the inference engine. The standard pattern was a "knowledge
engineer" interviewing a domain expert for hundreds of hours,
producing rules iteratively, watching the resulting system make
errors, asking the expert to explain, refining. MYCIN's rule base
took roughly five years of knowledge engineering. CADUCEUS
([Pople 1982](#citations)) took longer and was never completed.

The failure modes documented at the time:

- Experts couldn't articulate what they actually did. The
  "compiled expertise" was opaque even to its possessor.
- Different experts disagreed; the knowledge engineer had to
  pick a winner.
- Rules that worked in interview failed in deployment because the
  expert's stated rule didn't match their actual practice.
- Updates required re-interviewing. As the domain evolved, the
  knowledge base ossified.

### How this framework addresses it

The framework inverts the knowledge-acquisition pipeline:

1. **The domain expert writes the rulebook in natural language**
   — the same prose they would write for a textbook, training
   manual, or regulatory document.
2. **The LLM extracts the typed IR** (ADJ01 grammar) from that
   prose.
3. **ADJ02 total coverage forces every byte of the source to map
   to some IR node** — typed claim, structural grouping, or
   explicit `Discarded(reason)`. The model cannot silently drop
   the parts it doesn't understand.
4. **ADJ06 closes the loop**: when extraction is incomplete or
   inconsistent, the framework re-prompts the model with the
   structured violation and re-extracts. The model improves; the
   expert doesn't have to re-state the rulebook.

The asymmetric labor is *fundamentally inverted*. The expert
writes their domain knowledge once, in their own language. The
framework handles formalization. The "knowledge engineer" role
collapses into prompt engineering — itself a real specialization,
but a much shallower one than 80s knowledge engineering, and one
that's domain-portable (the same `decompose_text` prompt works
across TSA, clinical, and contract domains in our demos).

### What's still open

- **Rulebook quality.** The framework can extract what's in the
  source document; it cannot fix poorly-written or contradictory
  source documents. Rulebook authorship remains a domain-expert
  skill.
- **Latent expertise.** If the domain expert's actual decision
  process is *not* writable in prose (the "tacit knowledge"
  problem), the framework can't extract what isn't there. Same
  problem as the 80s, just shifted.

## 2. Knowledge maintenance crisis

### What went wrong

R1/XCON ([McDermott 1982](#citations); [Bachant & McDermott
1984](#citations)) started with about 750 rules in 1980. By 1985
it was over 6,000. By 1989, over 10,000. The maintenance crisis was
well-documented:

- Adding a rule could silently break previously-working
  inferences because rules interacted through the shared
  working-memory state.
- No declarative semantics for "what a rule means" — the only way
  to know was to run it.
- No type system, no test coverage. Regression testing was
  ad hoc.
- The team that maintained XCON grew from 2 people to 30+ to keep
  up with VAX hardware changes.
- When DEC's product line simplified in the early 90s, the
  maintenance cost no longer justified the savings; XCON was
  retired.

The general pattern: rule-based knowledge bases have **superlinear
maintenance cost** in their size, with no declarative tools for
catching regressions.

### How this framework addresses it

Two structural choices:

1. **Rules cite source spans in the original document.** Every IR
   node carries `source_spans: Vec<Span>`; every clause produced by
   the connector cites the IR node it lowered from. Update the
   source document, regenerate the IR, and the audit trail surfaces
   every clause that changed. **Maintenance becomes source-document
   editing, not rule-graph surgery.**
2. **The audit trail is reproducible.** Every LLM call is
   prompt-hashed; same input → same output at `temperature: 0.0`.
   A new extraction can be diffed against the old extraction at the
   IR-node level. Regressions surface as new `Discarded` reasons,
   new coverage failures, or polarity flips — all caught by the
   existing checker passes.

Plus a structural property the 80s couldn't have: **the IR has a
type system**. ADJ01's grammar forces every claim into one of
`Fact | Query | Rule(subtype) | Uncertainty | Exception |
TextRun | Discarded`. ADJ02 and ADJ03 catch type-level
inconsistencies before the engine runs. R1/XCON had no equivalent.

### What's still open

- **Empirical scaling test.** The framework has been tested at
  rulebook scales of dozens of rules. Whether ADJ02 + ADJ06 + the
  audit trail keep the maintenance cost sublinear at 10,000+
  rules is an open empirical question. R1/XCON's failure mode at
  scale was *not* observed at small scale either. We should not
  claim immunity until we've measured.
- **Rule conflict surfacing.** ADJ11's `MixedShapeOnSameConclusion`
  is one example of an explicit conflict check, but there are
  others the framework doesn't yet detect (overlapping
  `contributes` clauses with inconsistent LR magnitudes, etc.).

## 3. Closed-world brittleness

### What went wrong

Classical Prolog and most rule-based systems used the **closed-world
assumption**: if a fact wasn't in the KB, it was treated as false
(negation-as-failure). Symbolically clean; practically catastrophic
when the system encountered a case slightly outside its training:

- MYCIN's recommendations on patients with unusual histories were
  often wildly off — but the system reported them with the same
  confidence as on standard cases.
- R1/XCON would silently produce wrong configurations when a VAX
  customer's order had a component the rulebook didn't know about.
- PROSPECTOR's geological inferences degraded gracefully *because*
  it was Bayesian — but most expert systems weren't.

Lenat's CYC ([Lenat 1989](#citations)) was the maximum-effort
response: hand-code 1M+ facts of common-sense knowledge so the
system would have *something* to say about every situation. CYC
ran for 30+ years and never broke through. The brittleness problem
was bigger than any feasible amount of hand-coded common sense.

### How this framework addresses it

Three layers:

1. **The IR has an explicit `Uncertainty` node kind.** A model that
   isn't sure whether `pneumonia` is present can produce
   `NodeKind::Uncertainty` with a span citation. ADJ14 lowers it to
   a partially-observed atom contributing `log(c / (1 − c))`
   log-odds — not a hard yes/no.
2. **LP19e's LR aggregation is non-closed-world by construction.**
   A `contributes(LR, evidence, conclusion)` clause whose evidence
   is not observed simply isn't applied. The posterior is computed
   over what we *have* observed, not "what we know to be true."
   The prior + observed contributions yield a posterior; absence
   of evidence is silence, not falsity.
3. **ADJ18 active sensing** (specced; not yet implemented) lets
   the framework identify the highest-value-of-information atom
   to resolve. Brittleness at the edges becomes a structured
   question: "we don't know X; resolving X would shift the verdict
   from 60% to 85%; can we observe it?"

The framework's responses to the closed-world problem are not
asymptotic guarantees — they are structural mechanisms that make
the system's uncertainty *explicit and queryable* rather than
implicit and assumed-away.

### What's still open

- **Long tails.** No system handles every edge case correctly.
  When the source document or rulebook doesn't anticipate a case,
  the framework's behavior is bounded by what the rulebook
  declared, plus the prior. The verdict will be probabilistic
  rather than confidently-wrong (an improvement), but it will
  still be wrong in the cases the rulebook missed.
- **Unknown unknowns.** Active sensing (ADJ18) requires the
  framework to *know what to ask about*. The questions it doesn't
  know to ask remain unaddressed.

## 4. Combinatorial rule explosion

### What went wrong

R1/XCON at 10,000+ rules. CADUCEUS at ~700 disease entries. PIP
([Pauker et al. 1976](#citations)) on present illness at ~300
disease frames. The pattern: as the rule count grew, inference time
grew superlinearly because:

- Forward-chaining matched rules against working memory in
  worst-case O(rules × facts).
- Probabilistic networks (MUNIN, PATHFINDER) hit exponential
  blowup in joint-distribution computation.
- The Rete algorithm ([Forgy 1982](#citations)) optimized rule
  matching but couldn't fix the fundamental combinatorial cost.

The pragmatic response was *to keep KBs small* — which directly
contradicted the goal of building useful expert systems.

### How this framework addresses it

Two architectural advantages:

1. **LP19e LR aggregation is O(n) in number of active
   contributions.** No 2ⁿ-world enumeration. The asymptotic cost
   for the dominant probabilistic-adjudication query shape is
   linear, not exponential. WMC remains available for the cases
   that genuinely need joint conjunctive semantics, but those are
   typically small substructures within an otherwise
   LR-aggregated KB.
2. **Indexed retrieval** (deferred but specced in ADJ17 dependency
   graph). Standard Prolog-WAM-style functor/arity + first-argument
   indexing makes rule lookup O(log n) or O(1) per clause head.
   This is well-trodden territory — the optimization that R1/XCON
   *didn't* have, that LP19 will when ADJ17's knowledge store
   lands.

### What's still open

- **Empirical scaling.** The framework has been tested at
  rulebook sizes of dozens of rules. We have not measured behavior
  at 10,000+. The asymptotic claims should be empirically validated
  before we publish.
- **WMC blowup on edge cases.** A KB with many probabilistic
  indicators *and* a query whose conclusion does not participate
  in LR aggregation routes to `EnumerateAll + WMC` and could hit
  the 2ⁿ ceiling. LP19a (d-DNNF) and LP19d (Monte Carlo) are
  responses; they're specced but not yet implemented.

## 5. Calibration absence (MYCIN's certainty factors)

### What went wrong

MYCIN used **certainty factors** (CFs) in the range `[-1, +1]`,
combined by formulas like `CF_combined = CF₁ + CF₂(1 − CF₁)` for
positive evidence. The formulas were chosen because they:

- Were monotonic (more evidence → more confidence).
- Were associative and commutative (order of evidence didn't
  matter).
- Were computable.

But they were **not probabilistic** in any rigorous sense. Adams
([1976](#citations)) and Heckerman ([1986](#citations)) showed CFs
correspond to a particular Bayesian model only under restrictive
and usually-false assumptions. In practice, MYCIN's CFs were ad
hoc heuristics that *worked OK* on the cases the rulebook authors
had in mind and degraded silently outside that envelope.

PROSPECTOR ([Duda et al. 1976](#citations)) was the principled
response — a proper Bayesian network. But Bayesian networks had
their own problems:

- Independence assumptions were rarely satisfied; modelers had to
  add intermediate "context" nodes to capture correlation.
- Eliciting conditional probability tables (CPTs) was as
  bottlenecked as eliciting rules.
- Inference on dense networks was intractable; approximation was
  necessary.

The field never developed a clear successor to either approach.
EMYCIN ([van Melle 1979](#citations)) tried to package CFs as a
reusable shell; the same calibration problems showed up in the
shell.

### How this framework addresses it

The framework chooses the **likelihood-ratio formulation explicitly**:

- ADJ14 specifies every probabilistic rule as either
  `prior(P, conclusion)` (base rate) or `contributes(LR, evidence,
  conclusion)` (single-source likelihood ratio) or
  `contributes_jointly(LR_extra, [...], conclusion)` (interaction
  term).
- LP19e composes contributions in **log-odds space**, which is
  numerically stable, makes LRs interpretable ("LR=10 means this
  evidence makes the conclusion 10× more likely"), and matches the
  vocabulary evidence-based medicine and Bayesian forensics
  actually teach.
- The conditional-independence assumption is **named and
  recorded**: `engine_artifacts.independence_assumption_used: true`
  appears in the audit trail whenever LR aggregation produced the
  verdict. `contributes_jointly` is the explicit escape hatch for
  declared correlations.

This is a clear successor to both MYCIN's CFs (more rigorous) and
PROSPECTOR's Bayesian networks (more tractable for the dominant
inference shape). It is *not* a new mathematical framework — it's
the formalism the LR-based clinical-medicine and forensic
literatures have used for decades, finally wired into a working
adjudication framework.

### What's still open

- **Where do LRs come from?** Same problem as before — they are
  declared, not derived. The framework surfaces the declared LRs
  in the audit trail; it does not derive them from training data
  or expert elicitation. Future ADJ9-style calibration work is
  needed to learn LRs from labelled outcomes.
- **Calibration drift.** A rulebook with LRs calibrated for one
  population produces miscalibrated verdicts for a different one.
  The framework's audit trail makes the assumption auditable but
  does not detect drift automatically.

## 6. Explanation inadequacy

### What went wrong

MYCIN could trace which rules fired: "Rule 234 fired because A and
B were true." Clancey's GUIDON project ([Clancey 1983](#citations))
tried to turn rule traces into pedagogical explanations and found
that the rule structure carried almost none of the underlying
*pathophysiological* reasoning. Doctors asking "why?" wanted to
hear about disease mechanisms; the system had only rules.

The general failure: **rule firings are a mechanical trace, not a
domain-level rationale**. Even if the trace is accurate, it doesn't
satisfy users' actual explanation needs. The "explanation problem"
became its own research subfield in the late 80s and never produced
a fully satisfying answer.

### How this framework addresses it

ADJ16 derivation rendering (specced) turns the proof DAG into prose:

> P(stomach_bug | observed) = 78.3%. Derived from a prior of 10%
> (R1) and the following observed contributions:
> - Diarrhea (F1, bytes 9..17): LR+ 4.0 → +1.39 log-odds (R2)
> - Vomiting (F2, bytes 19..27): LR+ 3.0 → +1.10 log-odds (R3)
> - Mild fever (F3, bytes 29..39): LR+ 1.5 → +0.41 log-odds (R4)
> - Joint synergy of diarrhea + vomiting: ×1.8 → +0.59 log-odds (R6)

Every contribution cites a clause id, which cites an IR node id,
which cites source spans, which cite source bytes. The audit trail
makes every step *resolvable by data*, not narrated by commentary.

Importantly, the framework's explanation is **machine-trustable
*and* human-readable**:

- The numbers are reproducible: same inputs, same outputs.
- The prose grounds every claim in source bytes.
- The independence assumption is surfaced (per ADJ14).
- A reviewer can verify the math by recomputing in log-odds space
  from the cited LRs.

This is structurally better than MYCIN's rule trace because the
contribution model *is* the domain model (likelihood ratios are
the working vocabulary of evidence-based medicine, Bayesian
forensics, and quantitative legal-sufficiency review).

### What's still open

- **Domain rationale vs. mechanical trace.** ADJ16 produces a
  defensible derivation, but it doesn't necessarily produce an
  *intuitive* one. A doctor reading "LR+ 4.0 for diarrhea" gets a
  defensible verdict but may want to hear about gastrointestinal
  pathophysiology. The framework's response is "the rulebook is
  where domain rationale lives; the derivation cites the rulebook;
  the rulebook's prose is what the doctor reads if they want
  rationale." Whether that's *enough* is an empirical UX question.
- **Counterfactual reasoning.** "Would the verdict change if we
  didn't observe X?" — the framework can recompute, but the
  rendering of that recomputation as natural-language explanation
  is future work.

## 7. Validation crisis

### What went wrong

Expert system validation was notoriously weak:

- **Small test sets.** MYCIN was validated against 10–20 clinical
  cases reviewed by Stanford faculty (Yu et al. 1979). The cases
  were typically the same ones the rule base was developed on.
- **Self-confirming evaluation.** The experts who provided the
  rules also judged the outputs. Disagreement was rare by
  construction.
- **Pass/fail metrics on small cohorts.** No statistical power,
  no held-out test set, no measurement of calibration.

By the 90s, the field had a credibility problem: it was unclear
whether any deployed expert system *actually* matched its
benchmarks in real use. The "Eliza effect" — users attributing
intelligence to mechanical pattern-matching — was a real concern.

### How this framework addresses it

Two structural changes:

1. **Audit trail makes replay reproducible.** Every adjudication
   produces a complete record that can be replayed bit-for-bit.
   The validation question becomes "do the verdicts on a
   held-out test set match the gold labels?" — a measurable
   question, not a rhetorical one.
2. **Structural checkers (ADJ02–05) give explicit validation
   surface.** A model that produces an IR that fails ADJ02
   (coverage) or ADJ04 (round-trip drift) is producing a verdict
   the framework explicitly does not trust. These passes give a
   structured "the model is not ready" signal — distinct from
   "the model is wrong" — that classical expert systems didn't have.
3. **ADJ12 benchmarking is per-pass.** Reporting "ADJ02 5/5,
   ADJ04 0/5 (5/5 drift)" is informative in a way "answer
   correct, answer incorrect" is not. The failure mode is
   localized to a checker pass, which is itself diagnostic of what
   to improve.

### What's still open

- **Gold-label problem unchanged.** Who decides what the "correct"
  verdict is on a clinical chart? The same epistemological
  problem MYCIN had. The framework provides structured
  validation surface; it doesn't fix the underlying truth problem.
- **Realistic test fixtures.** Today's ADJ12 fixtures are 24
  bytes ("1 carry-on bag, matches."). Real-world adjudication
  inputs are 500–5000 bytes with multiple clauses, exceptions,
  and cross-references. The benchmark needs to scale to realistic
  fixture distributions before claims of "matches frontier-model
  performance" are defensible.

## 8. Knowledge engineer dependency

### What went wrong

The "knowledge engineer" — a specialized translator between
domain expert and rule base — was the bottleneck through which
every expert system passed. They were:

- **Expensive.** Top knowledge engineers commanded consulting
  rates competitive with senior software engineers in the 80s.
- **Slow.** Multi-year engagements were standard for any
  non-trivial KB.
- **Distorting.** The KE's interpretation of the expert's words
  introduced systematic biases.
- **A bottleneck for updates.** Even small changes required the
  KE to re-engage with the expert.

The role was *necessary* because the gap between "what an expert
says" and "executable rules" was wide. It was also *fatal*
because it priced expert systems out of most use cases.

### How this framework addresses it

The LLM **is** the knowledge engineer. The framework's loop is:

1. Domain expert writes the rulebook in natural language.
2. LLM (Role::Extractor) produces the typed IR.
3. ADJ02–05 verify structural properties; ADJ06 closes the
   correction loop.
4. The connector lowers the IR to the engine; the engine produces
   the verdict; the audit trail records everything.

There is no human in the loop between step 1 and step 4 except
for ADJ06's clarification interactions (which are themselves
machine-mediated). The domain expert's prose *is* the source of
truth; the framework handles the translation.

The cost shift is real:

- The 80s knowledge engineer's hour cost has been replaced by an
  LLM API call (or local inference).
- The 80s 6-month engagement has been replaced by a same-day
  iteration: edit the rulebook, re-extract, re-run.
- The 80s update bottleneck has been replaced by source-document
  editing.

### What's still open

- **Prompt engineering is the new specialized role.** Writing the
  `decompose_text` prompt that reliably produces a flat IR across
  domains is its own skill. It's a much shallower specialization
  than 80s knowledge engineering — the same prompt works across
  TSA, clinical, contract demos in this framework — but it's not
  zero.
- **Rulebook authorship.** The framework can extract what's in the
  rulebook; it can't fix poor rulebook authorship. Domain experts
  still need to write clearly. Whether *they* need a writing
  coach is a separate question.

## What this framework does *not* fix

Honesty is more useful than aspiration:

1. **The gold-label problem.** Validating a probabilistic verdict
   requires knowing the ground truth. In medicine, law, finance,
   and most other adjudication domains, ground truth is
   contested. The framework's audit trail makes the verdict
   reproducible; it does not make it correct.
2. **The miscalibrated-rulebook problem.** If the rulebook
   declares wrong LRs, the framework will produce wrong verdicts
   with correct audit trails. Calibration is the modeler's
   responsibility; the framework surfaces it but does not derive
   it.
3. **The long-tail problem.** Cases not anticipated by the
   rulebook will produce verdicts bounded by what the rulebook
   plus the prior implies. Active sensing (ADJ18) helps when the
   framework knows what to ask about; unknown unknowns remain
   unknown.
4. **The maintenance-at-scale problem.** R1/XCON's 10,000-rule
   maintenance crisis happened in deployment, not at small scale.
   This framework has not yet been tested at deployment scale.
   The architectural choices (rules cite source spans, audit
   trail is reproducible, type system catches inconsistencies)
   *should* keep maintenance cost sublinear, but we have not
   measured.

## The pitch this analysis enables

Six of the eight historical failure modes are **structurally
addressed** by this framework's architectural choices, not by
incremental engineering. One is **partially addressed** (validation
— the framework provides structured surface but cannot fix the
gold-label problem). One is **architecturally addressed but
empirically untested** (rule explosion — the framework should scale
better, but we have not measured).

This is not "MYCIN with an LLM front-end." It is a deliberate
redesign of the probabilistic-adjudication stack that maps onto the
failure modes the expert-systems literature documented, with
explicit attention to:

- the right division between machine extraction (LLM) and machine
  inference (deterministic logic engine);
- the right probabilistic semantics (LR aggregation, not CFs);
- the right audit / replay infrastructure (proof DAGs with source
  citations);
- the right human-machine loop (ADJ06 clarification, not
  knowledge-engineer interview cycles).

The paper's thesis writes itself:

> *Expert systems were prematurely abandoned in the 90s for eight
> specific, documented reasons. Six are structurally addressed by
> the architectural decisions of the adjudication framework
> presented here; one is partially addressed; one remains an open
> empirical question. The decision to revisit logic-engine-based
> reasoning in 2026 is justified not by ignoring the historical
> failures but by addressing them.*

## Citations

> Adams, J. B. (1976). A probability model of medical reasoning
> and the MYCIN model. *Mathematical Biosciences*, 32(3-4),
> 177-186.

> Bachant, J., & McDermott, J. (1984). R1 revisited: Four years in
> the trenches. *AI Magazine*, 5(3), 21-32.

> Buchanan, B. G., & Shortliffe, E. H. (1984). *Rule-Based Expert
> Systems: The MYCIN Experiments of the Stanford Heuristic
> Programming Project*. Addison-Wesley.

> Clancey, W. J. (1983). The epistemology of a rule-based expert
> system — a framework for explanation. *Artificial Intelligence*,
> 20(3), 215-251.

> Duda, R. O., Hart, P. E., & Nilsson, N. J. (1976). Subjective
> Bayesian methods for rule-based inference systems. *AFIPS
> National Computer Conference Proceedings*, 45, 1075-1082.

> Feigenbaum, E. A. (1977). The art of artificial intelligence:
> Themes and case studies of knowledge engineering. *Proceedings
> of IJCAI 1977*, 1014-1029.

> Forgy, C. L. (1982). Rete: A fast algorithm for the many
> pattern/many object pattern match problem. *Artificial
> Intelligence*, 19(1), 17-37.

> Heckerman, D. (1986). Probabilistic interpretations for MYCIN's
> certainty factors. *Uncertainty in Artificial Intelligence*,
> 167-196.

> Lenat, D. B. (1989). *Building Large Knowledge-Based Systems:
> Representation and Inference in the Cyc Project*. Addison-Wesley.

> McDermott, J. (1982). R1: A rule-based configurer of computer
> systems. *Artificial Intelligence*, 19(1), 39-88.

> Pauker, S. G., Gorry, G. A., Kassirer, J. P., & Schwartz, W. B.
> (1976). Towards the simulation of clinical cognition: Taking a
> present illness by computer. *American Journal of Medicine*,
> 60(7), 981-996.

> Pearl, J. (1988). *Probabilistic Reasoning in Intelligent
> Systems: Networks of Plausible Inference*. Morgan Kaufmann.

> Pople, H. E. (1982). Heuristic methods for imposing structure on
> ill-structured problems: The structuring of medical diagnostics.
> In *Artificial Intelligence in Medicine* (pp. 119-185). Westview
> Press.

> van Melle, W. (1979). A domain-independent production-rule
> system for consultation programs. *Proceedings of IJCAI 1979*,
> 923-925.

> Yu, V. L., Fagan, L. M., Wraith, S. M., Clancey, W. J., Scott,
> A. C., Hannigan, J., Blum, R. L., Buchanan, B. G., & Cohen, S.
> N. (1979). Antimicrobial selection by a computer: A blinded
> evaluation by infectious disease experts. *JAMA*, 242(12),
> 1279-1282.

## Status

Contextual analysis, not a strict specification. Treat as the
literature-review backbone for a publishable paper on probabilistic
adjudication.

## Where to read next

- [ADJ14](ADJ14-probabilistic-ir-semantics.md) — the
  likelihood-ratio aggregation semantics that addresses failure
  mode 5 (calibration).
- [ADJ16](ADJ16-derivation-rendering.md) — the human-readable
  derivation rendering that addresses failure mode 6 (explanation).
- [ADJ17](ADJ17-knowledge-store-fact-merge.md) — the persistent
  knowledge store that supports failure modes 2 and 4
  (maintenance, scaling).
- [ADJ18](ADJ18-active-sensing-voi.md) — the value-of-information
  sensing that addresses failure mode 3 (closed-world brittleness).
