# ADJ00 — Adjudication Framework: Typed IR for Rule-Based Decisions on Ambiguous Input

## Overview

Many high-stakes domains share a structural pattern:

1. A **codified rule corpus** (clinical guidelines, tax code, securities law,
   open-source licenses, game rules, building codes, IRB protocols).
2. **Ambiguous natural-language input** (a clinical note, a deal document, a
   user-submitted question, a code review request).
3. A required **defensible output** — meaning the answer must be reconstructible,
   traceable to its sources, and able to survive challenge by another expert.

We call this pattern **rule-based adjudication under ambiguous input**. Medical
diagnosis, tax preparation, license-compatibility review, customs declarations,
patent prior-art search, peer review, and game-rules adjudication are all
instances of it.

The Adjudication framework is the layer that lets the existing Logic VM,
Constraints VM, and Symbolic VM in this repo be applied to problems of this
shape. It does so by introducing a typed intermediate representation between
natural-language input (or rulebook text) and the existing logic backends, and
by treating extraction errors as static type errors rather than as silent
downstream wrongness.

The central design claim is:

> **Natural language to logical form is a compilation pass.** It deserves the
> same discipline a compiler frontend applies to source code: a typed IR,
> coverage as a soundness criterion, named lowering passes, and source-span
> provenance preserved end-to-end. Inputs that cannot be fully lowered to the
> IR do not silently produce wrong answers. They fail the type check, and they
> trigger principled clarification.

This spec establishes the framework. Sub-specs (`ADJ01..`) define each
component in detail.

## Why a Typed IR Is the Right Abstraction

LLMs are good at reading ambiguous prose and emitting structured output. Logic
engines (the repo's existing Logic VM, Constraints VM, Symbolic VM) are good at
reasoning over structured facts. The current state of the art glues these
together with prompts, expecting the LLM to produce well-formed facts and the
engine to reason correctly over them.

This glue is the failure point.

The failure mode is not solver brittleness; the existing engines are sound.
The failure mode is **lossy extraction**. An LLM that reads "patient denies
chest pain" and emits `symptom(patient, chest_pain)` has produced a syntactically
valid fact that means the opposite of the source. The engine reasons correctly
over a wrong fact, producing a confident wrong answer with no signal that
anything went wrong.

The standard mitigations — chain-of-thought, self-refinement, retrieval — make
this kind of error less frequent but do not catch it when it happens. They have
no notion of "this clause of the input was not accounted for" or "the polarity
of this fact disagrees with negation cues in its source span." They cannot,
because they have no IR. The fact `symptom(patient, chest_pain)` carries no
record of where it came from.

A typed IR fixes this by construction. Every node in the IR carries:

- the term it represents,
- mandatory polarity and modality tags,
- the source spans it was derived from,
- provenance for any intermediate lowering steps.

A separate pass checks that the IR's coverage of the source is complete, that
its polarity and modality are consistent with the source spans they cite, and
that the IR round-trips to natural language without semantic drift. Inputs that
fail any check are rejected before the logic engine runs.

The contribution is not the IR alone, nor the LLM, nor the engines. It is the
**typed compilation discipline** that makes the three composable and the
pipeline auditable.

## Layer Position

```
                          natural-language input        rulebook text
                                  │                          │
                                  ▼                          ▼
                          ┌───────────────┐         ┌────────────────┐
                          │ extractor LLM │         │ compiler LLM   │
                          └───────┬───────┘         └────────┬───────┘
                                  │                          │
                                  ▼                          ▼
                                ┌──────────── ADJ IR ──────────────┐
                                │  Facts / Queries / Uncertainties │
                                │  Rules / Exceptions / Defaults   │
                                │  Polarity, modality, spans, prov │
                                └────────────────┬─────────────────┘
                                                 │
                          ┌──────────────────────┼──────────────────────┐
                          │                      │                      │
                          ▼                      ▼                      ▼
                  coverage check       polarity/modality check     round-trip check
                          │                      │                      │
                          └──────────────┬───────┴──────────────────────┘
                                         ▼
                              adversarial verifier
                                         │
                                  passes │ fails → clarification dialogue
                                         ▼                       │
                          ┌──────────────────────────────────────┘
                          ▼
                  LP** Logic VM / Constraints VM / Symbolic VM
                          │
                          ▼
                  proof DAG + answer
                          │
                          ▼
                  audit trail (source spans → IR nodes → fired rules → answer)
```

The IR sits above the existing logic backends and below any natural-language
surface. It is the only thing the backends consume. The backends do not need
to be modified.

## What This Spec Defines

This spec defines:

1. The pattern the framework addresses and the architecture at a glance.
2. The IR node grammar (informal here; formalized in `ADJ01`).
3. The four checker passes and what each is responsible for ruling out.
4. The clarification-dialogue protocol that failed checks invoke.
5. The escalation ladder used when automatic clarification is insufficient.
6. The audit-trail schema that emerges from the pipeline.
7. The symmetric rule-compilation pipeline (rulebook → IR) and why it uses the
   same checker discipline as input compilation.
8. A worked example (TSA carry-on baggage) traced end-to-end.
9. A roadmap of sub-specs (`ADJ01..`) covering each component in depth.

It does **not** define:

- The full grammar of the IR (`ADJ01`).
- Algorithms for each checker pass (`ADJ02`..`ADJ05`).
- The clarification dialogue protocol's wire format (`ADJ06`).
- The audit-trail storage schema (`ADJ07`).
- Specific domain ontologies (`ADJ10+`).

## The Typed IR

An IR node has the shape:

```text
IRNode := {
    id:             unique within document,
    kind:           Fact | Query | Uncertainty | Rule | Exception | Discarded,
    term:           a logic term (compound, atom, number, list, var),
    polarity:       Affirmed | Denied | Uncertain,
    modality:       Present | Past | Future | Hypothetical | FamilyHistory
                    | RuledOut | Conditional,
    source_spans:   non-empty list of (document_id, start_offset, end_offset),
                    *except* when kind = Rule or Discarded
                    (Rules may cite a single rulebook span; Discarded must cite
                    the input span being discarded and the discard_reason),
    confidence:     real in [0, 1] (informational; not used by the type check),
    lowered_from:   optional IRNode.id (when this node is the lowering of a
                    higher-level node),
    discard_reason: required when kind = Discarded,
}
```

The IR is a directed acyclic graph: `lowered_from` edges form a forest rooted at
nodes whose `source_spans` cite original input. Leaves are the IR nodes the
logic engine actually consumes.

Three properties are non-negotiable:

1. **Every IR node is span-grounded.** No node exists without a source span
   (or, for derived nodes, a chain of `lowered_from` edges that ultimately
   reach span-bearing nodes).
2. **Polarity and modality are mandatory.** There is no default. An extractor
   that produces a `Fact` without setting these fields produces an ill-formed
   IR and is rejected by the parser.
3. **Discarded is not the absence of a node, it is an explicit node.** Input
   spans the extractor judges irrelevant must be represented by a `Discarded`
   node citing the span and a reason, not by silently omitting them.

The full grammar, including the term language, the modality lattice, and the
typing rules for `lowered_from` edges, is specified in `ADJ01 — Adjudication IR
Grammar`.

## The Four Checker Passes

The type checker runs the four passes in increasing order of computational
cost. The first three are static; the fourth uses a second LLM at inference
time. Any pass may reject an IR; rejection routes to the clarification
dialogue described in the next section.

### Pass 1 — Coverage

**What it rules out:** the extractor silently dropping clinically (or
domain-) meaningful spans of the input.

**How it works.** A separate token classifier (or a constrained LLM call)
tags each token of the input as either *meaningful* (must be accounted for by
some IR node) or *non-meaningful* (function words, pleasantries, document
metadata that has no domain content). Every meaningful token must appear in
the `source_spans` of at least one IR node — either as part of a Fact, Query,
Uncertainty, or as an explicit `Discarded` node with a reason.

The classifier's vocabulary is domain-specific and is itself a versioned
artifact (`ADJ02` discusses its construction and how false-positives are
managed).

If coverage fails, the pass emits a list of uncovered spans. These become the
seed for clarification questions of the form *"You did not account for
'\<span\>'; what does it mean?"*

### Pass 2 — Polarity and Modality Consistency

**What it rules out:** the extractor reading "denies chest pain" as
`symptom(chest_pain)` with polarity Affirmed.

**How it works.** For each IR node whose kind is `Fact`, `Query`, or
`Uncertainty`, the pass inspects the node's source spans for lexical and
syntactic markers of negation, modality, temporality, and ownership. The
analysis is a rule-based scope detector in the tradition of NegEx (Chapman et
al. 2001) and ConText (Harkema et al. 2009), generalized to a configurable set
of trigger classes:

- **Negation:** "no", "denies", "without", "negative for", "ruled out", "not"
- **Hedging:** "possibly", "questionable", "suggestive of", "consistent with"
- **Temporality:** "history of", "in 2019", "previously", "currently",
  "on admission"
- **Subject:** "father", "mother", "brother", "family history of", "patient"
- **Conditional:** "if", "when", "in case of"

If the trigger structure in a span is incompatible with the IR node's polarity
or modality tags, the pass rejects the node. *Incompatible* is precisely
defined in `ADJ03` as a small set of forbidden combinations (e.g., span
contains an in-scope negation trigger and node has polarity = Affirmed).

The pass is decidable and fast. It does not use an LLM.

### Pass 3 — Round-Trip Entailment

**What it rules out:** lossy paraphrase that survives passes 1 and 2 — for
example, dropping a quantifier or replacing "severe" with "moderate".

**How it works.** A separate LLM is given each leaf IR node and asked to render
it back to natural language. A textual entailment model is then asked, in both
directions:

- Does the original source span entail the rendered IR?
- Does the rendered IR entail the original source span?

Bidirectional entailment is required for the node to pass. Failure in either
direction routes to clarification with the question *"Does '\<rendered IR\>'
mean the same as '\<original span\>'?"*

The entailment model is a deliberately weaker, separately trained model — not
the extractor. Using the same model for both creates a self-confirmation loop
and defeats the purpose. Pass 3 details are in `ADJ04`.

### Pass 4 — Adversarial Reading

**What it rules out:** novel failure modes that the static checks (1–2) and
the symmetric entailment check (3) do not anticipate. This pass is the
inference-time analogue of M&M conferences in medicine and appellate review
in law: a separate model whose job is to find the strongest reading of the
source that contradicts the IR.

**How it works.** The adversary LLM receives the source spans (without the
extractor's IR) and is prompted asymmetrically:

> *Assume the extraction is wrong. Find the strongest reading of this source
> span that contradicts \<IR\>. If you cannot find one, say so.*

If the adversary produces a contradicting reading that a separate plausibility
judge endorses, the node fails. Failure routes to clarification.

This is **adversarial verification, not adversarial training.** No weights are
updated. The adversary's output is one of: agreement, a specific contradicting
reading, or abstention. The dialogue is logged.

Failure-mode mitigations and the choice of base model for the adversary are
discussed in `ADJ05`.

## Clarification Dialogue as First-Class Output

When a checker pass rejects an IR, the rejection is not an error to be
swallowed. It is a structured artifact whose shape is determined by which
pass failed:

| Failed pass | Clarification question template |
|---|---|
| Coverage | *"You did not account for: '\<uncovered span\>'. What does it mean?"* |
| Polarity / modality | *"In '\<span\>', is the patient currently \<predicate\>, or denying any history of \<predicate\>?"* (and similar shapes for modality) |
| Round-trip | *"Does '\<rendered IR\>' mean the same as '\<original span\>'?"* |
| Adversarial | *"The verifier suggests '\<contradicting reading\>'. Is that closer to your meaning than '\<original IR\>'?"* |

The clarification is itself a normal pipeline input on the next iteration.
The response is appended to the document, the extractor re-runs, and the IR
is re-checked. The iteration count and intermediate IRs are logged.

This is not a fallback. Clarification is the system's default behavior on
ambiguous input. The Socratic dynamic (attending asking resident, judge asking
counsel) is the well-trodden cultural template for trustworthy decisions in
exactly these domains, and the framework reproduces that template in software.

The wire format and turn-taking semantics of the dialogue are specified in
`ADJ06`.

## Escalation Ladder

Not every clarification needs a human. The system tries cheap interventions
first:

| Rung | Description | Typical cost | Typical use |
|---|---|---|---|
| 0 | Re-prompt the extractor with the specific failure | Cents | "You tagged 'denies' as Affirmed; reconsider" |
| 1 | Re-query the input source for adjacent context | Cents | "Was this note from intake or follow-up?" |
| 2 | Patient- / user- facing clarification | Minutes | Surface as a question in the intake form |
| 3 | Domain-expert review | Hours | Clinician, CPA, or attorney reviews and answers |

The IR records which rung resolved each ambiguity. The per-deployment
distribution over rungs is itself useful compliance metadata: *"3.1% of facts
in Q2 required clinician-rung clarification."*

Rung selection policy and the metadata schema are in `ADJ06`.

## Audit Trail by Construction

The IR is not designed to produce an audit trail. The IR **is** the audit
trail.

Every IR node has source spans. Every lowered node has a `lowered_from` chain
ending at span-bearing nodes. Every Rule node has a citation to the rulebook
span it was compiled from. The proof DAG produced by the logic engine cites
the IR nodes it consumed. Therefore the final answer chains back, without
gaps, through:

```
   answer
     ↑
   proof DAG node (rule application)
     ↑   ↑
   Rule  Fact / Query
     ↑   ↑
   rulebook span     input span (possibly after clarification turns)
```

Every link in the chain is materialized data, not a post-hoc explanation. A
reviewer auditing the system can replay the dialogue, re-run the checker
passes, re-execute the proof, and verify that the answer is the only one
consistent with the audited inputs.

The schema is specified in `ADJ07`. Storage and replay tooling is in `ADJ08`.

## The Symmetric Rule-Compilation Pipeline

The framework treats rulebook compilation with the same discipline as input
extraction. A rulebook is run through a parallel pipeline:

1. **Section-level coverage.** Every section, subsection, and paragraph of the
   rulebook must be accounted for either by a Rule node, a group of Rule
   nodes, or an explicit `Discarded` marker (for preambles, examples,
   commentary).
2. **Rule polarity and modality.** "Do not allow X" must not be compiled to
   "allow X." The same NegEx/ConText machinery applies.
3. **Round-trip entailment.** Compiled rules must round-trip to natural
   language that bidirectionally entails the source paragraph.
4. **Adversarial reading.** A separate model attempts to find a reading of the
   rulebook paragraph that contradicts the compiled rule.

Rules additionally carry an `as_of` timestamp from their source so the engine
can re-run prior cases against superseded rules. Conflicts between sources are
not silently resolved: when two rules from different sources both fire and
disagree, both are surfaced in the proof DAG and the answer is reported as
*disputed*, with each rule's provenance attached.

Crucially, this is the workflow that lets a non-expert author the framework
without a domain-expert coauthor: the LLM does the compilation, the type
checker flags suspect rules, and the domain expert *reviews* (a faster, easier
task than authoring). This is closely analogous to legal codification efforts
such as Catala (Merigoux et al., Inria), but with the manual codification step
replaced by typed automatic compilation.

The rule-compilation pipeline is detailed in `ADJ09`.

## Worked Example: TSA Carry-On Baggage

The framework's first end-to-end demonstration is TSA carry-on baggage
adjudication. The choice is deliberate:

- The rulebook (TSA prohibited-items, Liquids-Aerosols-Gels rules) is public,
  finite (roughly 10–20 pages of normative content), and small enough to
  hand-verify the compiled rule IR against the source.
- Inputs are short, prose-style declarations from a passenger.
- Polarity and modality bait is abundant: *"I am **not** bringing the matches,
  only the lighter."*
- A ground-truth oracle exists: the published decision tree.
- The stakes are universally relatable but low.

A short trace:

**Input:** *"I'd like to bring a 4 oz tube of toothpaste, a 100 ml perfume,
three lithium camera batteries (rated 80 Wh each), a bottle of wine for my
mother, and a 4-inch pocket knife. I am not bringing matches, only a single
disposable lighter."*

**Coverage pass:** every noun phrase and quantity must appear in an IR node's
source spans. The negation in *"not bringing matches"* must also be covered —
which forces an IR node, not an omission.

**Resulting IR (sketch):**

```text
F1 Fact   carry_on_item(toothpaste, volume=4_oz)          Affirmed  Present
F2 Fact   carry_on_item(perfume, volume=100_ml)            Affirmed  Present
F3 Fact   carry_on_item(lithium_battery, count=3, Wh=80)   Affirmed  Present
F4 Fact   carry_on_item(wine, container=bottle)            Affirmed  Present
F5 Fact   carry_on_item(pocket_knife, blade_length=4_in)   Affirmed  Present
F6 Fact   carry_on_item(matches)                           Denied    Present
F7 Fact   carry_on_item(lighter, type=disposable, count=1) Affirmed  Present
```

**Polarity check:** F6 has `Denied` polarity and its span contains "not
bringing matches"; consistent. Had the extractor produced F6 with `Affirmed`,
the pass would have rejected it.

**Engine output:** rules compiled from the LAG section reject F2 (perfume
exceeds 100 ml LAG limit only if a separate container — needs clarification,
since 100 ml is exactly at the limit and the regulation says *less than 100
ml*). The system routes to clarification: *"Is the perfume bottle exactly
100 ml, or under 100 ml?"* F5 is rejected outright by the prohibited-items
rule. F3 is accepted (under 100 Wh per battery, under 2 spare batteries — but
the count is 3, so this also routes to clarification on whether they are in
the device).

**Final answer:** decision per item, with proof DAG citing rulebook section
for each ruling, and clarification questions for the two ambiguous cases. The
audit trail includes source spans, IR nodes, fired rules, and the
clarification dialogue.

The point of the worked example is not the difficulty (TSA rules are simple).
The point is that *every step is checkable by a skeptic*. A reviewer can read
the IR side-by-side with the input, read the compiled rules side-by-side with
the rulebook, and verify the final decision against both. This is the property
medicine and finance need; TSA is where we demonstrate that the framework
provides it.

## Sub-Specs Roadmap

The following sub-specs flesh out each component. They will be written
incrementally, with each driven by the next implementation step. Order is
expected but not strict.

| Spec | Title | Status |
|---|---|---|
| `ADJ00` | Adjudication Framework (this document) | draft |
| `ADJ01` | Adjudication IR Grammar | planned |
| `ADJ02` | Coverage Checker | planned |
| `ADJ03` | Polarity and Modality Checker | planned |
| `ADJ04` | Round-Trip Entailment Checker | planned |
| `ADJ05` | Adversarial Verifier | planned |
| `ADJ06` | Clarification Dialogue Protocol | planned |
| `ADJ07` | Audit Trail Schema | planned |
| `ADJ08` | Audit Replay Tooling | planned |
| `ADJ09` | Rule-Compilation Pipeline | planned |
| `ADJ10` | TSA Carry-On Worked Example | planned |
| `ADJ11` | Probabilistic Extension (ProbLog Integration) | planned |

`ADJ11` is the bridge to the existing Logic VM's probabilistic extension and
is the component that makes diagnostic domains (medicine, finance) tractable.
It is separated from the rest because it is a substantial body of work in its
own right, building on the existing Prolog implementation (`PR00`..`PR90`) and
adding distribution-semantics inference. It will be specified separately when
this framework spec is stable.

## Related Work

The framework borrows from and contrasts with several existing lines of work.
These are listed for orientation; a fuller treatment belongs in any
publication derived from this spec.

- **Neurosymbolic AI / LLM + solver pipelines.** Logic-LM (Pan et al. 2023),
  LINC (Olausson et al. 2023), SatLM, SymbCoT, Faithful-CoT. These pipelines
  translate natural language to a formal target and dispatch to a solver. None
  treat the translation step as a typed compilation pass with mandatory
  coverage; none preserve source spans through to the solver's output.
- **DeepProbLog** (Manhaeve et al. 2018). Embeds neural networks as
  probabilistic facts inside ProbLog. Important reference for `ADJ11`; complementary,
  not competing.
- **Catala** (Merigoux et al., Inria). A DSL for encoding law as executable
  code with citations to source. Closest cousin on the rule-compilation side.
  The framework's contribution is replacing manual codification with typed
  automatic compilation under the same provenance discipline.
- **NegEx and ConText** (Chapman 2001; Harkema 2009). Rule-based clinical
  negation and modality detection. The framework adopts these algorithms
  inside Pass 2 and generalizes them to a configurable trigger taxonomy.
- **Faithfulness metrics for summarization** (SummaC, QAGS, FactCC). Soft
  metrics for whether a summary is faithful to a source. The framework turns
  the same idea into a hard typecheck and uses it inside Pass 3.
- **AI Debate** (Irving, Christiano, Amodei 2018) and **Constitutional AI**
  (Bai et al. 2022). Two-model adversarial setups at inference time. The
  framework's Pass 4 is structurally an instance of this lineage.
- **Provenance semantics** (Buneman, Green, Tannen). Why/how/where provenance
  in databases. Closest formal cousin to the framework's audit-trail discipline.
- **Classic expert systems** (CLIPS, Drools, MYCIN). The lineage being updated:
  expert systems failed because NL → facts extraction was hand-built and
  brittle; the framework substitutes a typed LLM-driven extractor with
  coverage discipline.

## Limitations

The framework as specified does not address, and should not be claimed to
address, the following:

1. **Soundness of the underlying LLM.** Coverage / polarity / round-trip /
   adversarial checks reduce the rate of silent extraction errors, but they
   are not proofs of correctness. Adversaries that share biases with the
   extractor can collude on shared blind spots. Pass 4 mitigates this by
   demanding a different model family; nothing eliminates it entirely.
2. **Domain knowledge that is not codifiable.** Clinical judgment,
   professional discretion, and tacit expertise resist rule extraction by
   construction. The framework's response is to flag these as
   human-judgment-required rather than to pretend they have been codified.
3. **Calibrated probabilities.** The IR's `confidence` field is informational.
   The framework treats facts as either present or absent, with polarity. For
   domains where calibrated probabilities are essential (medical diagnosis,
   financial risk), the probabilistic extension (`ADJ11`, building on
   distribution-semantics ProbLog) is required. This spec does not deliver
   probabilistic reasoning on its own.
4. **Multi-modal input.** The framework as specified handles natural-language
   text and structured forms. It does not address images, audio, video, or
   other modalities. Extension to multi-modal input is plausible but
   out-of-scope here.
5. **Scalability of rulebook compilation.** Pass 4 (adversarial) and Pass 3
   (round-trip entailment) on the rule pipeline are expensive. For very large
   rulebooks (entire tax code, complete legal corpus), incremental compilation
   and rule-level caching are required and are not specified here.
6. **Adversarial inputs designed to defeat the checkers.** A user who knows
   the checker's trigger lexicon can craft inputs that bypass it. Pass 4
   reduces this but does not eliminate it. The framework does not claim
   robustness against motivated adversaries.

## Out of Scope

- Replacing domain experts. The framework's intended deployment is as
  decision-support for experts, not as autonomous decision-maker.
- Real-time / streaming input. The pipeline assumes a complete input document
  at the time of extraction.
- Cross-document reasoning over large corpora. Each adjudication operates on
  one input document plus the compiled rulebook IR.
- Regulatory certification (FDA, FINRA, etc.). Such certification is
  pursued, if at all, only on top of a concrete deployment in a specific
  domain and is out of scope for this framework spec.

## Status

This spec is the framework overview. It is intended to be sufficient to
- establish the design and authorship of the framework on the public record,
- serve as the technical-report basis for any subsequent publication, and
- guide the implementation of the sub-specs `ADJ01`..`ADJ11`.

It is not yet sufficient to implement against. Implementation requires the
sub-specs.
