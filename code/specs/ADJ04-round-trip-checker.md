# ADJ04 — Round-Trip Entailment Checker: Catching Lossy Paraphrase

## Overview

This is the third of the four checker passes from [`ADJ00`](ADJ00-adjudication-framework.md).
After [`ADJ02`](ADJ02-coverage-checker.md) has confirmed coverage and
[`ADJ03`](ADJ03-polarity-modality-checker.md) has confirmed polarity
and modality, ADJ04 catches the residue: **lossy paraphrase** that
survives the structural checks because none of its content words
trigger the polarity scope rules.

Examples of what slips through ADJ02 and ADJ03 but is caught here:

- *"Severe chest pain, 9/10, sudden onset"* → extracted as `Fact {
  term: chest_pain(patient) }`. Coverage passes (the span is
  covered). Polarity passes (no negation triggers). But the *severity*
  and *onset* are silently lost.
- *"Patient took 30 mg, not the prescribed 50 mg"* → extracted as
  `Fact { term: took_medication(patient, dose=30mg) }`. Coverage and
  polarity pass. But the relationship *"not the prescribed 50 mg"* —
  which is a separate fact, possibly a `Denied` claim about the
  prescribed dose — is omitted.
- *"Possible but unlikely PE"* → extracted as `Uncertainty { term:
  pe(patient) }`. Coverage and polarity (Uncertain) pass. But the
  qualifier *"unlikely"* is dropped, which matters when the
  probabilistic engine assigns weight.

The pattern is: structural checks (ADJ02 / ADJ03) operate on lexical
and scope evidence; they cannot see when the *information content* has
been simplified. ADJ04 asks a different question:

> Does the IR mean the same as the source span — in *both* directions?

## Layer Position

```
   ADJ02 coverage             ← all meaningful spans covered
        │
        ▼
   ADJ03 polarity/modality    ← no scope-trigger conflicts
        │
        ▼
   ADJ04 round-trip            ← this document — semantic equivalence
        │
        ▼
   ADJ05 adversarial verifier
```

ADJ04 is the most LLM-heavy of the static checker passes. It uses a
*separate* model (different prompt, ideally different base) to render
the IR back into natural language, and a *third* model (a natural
language inference / entailment model) to verify equivalence between
the rendering and the source. The pass is more expensive than ADJ02 and
ADJ03, but cheaper and more reliable than ADJ05 (which is openly
adversarial).

## The Check

For each leaf IR node (the nodes the logic backends will actually
consume), the pass performs:

1. **Render IR → natural language.** A small LLM call produces a plain-
   English rendering of the node's term, polarity, and modality.
2. **Compute entailment in both directions.**
   - Does the source span entail the rendered IR?
   - Does the rendered IR entail the source span?
3. **Decide.** Bidirectional entailment is required. Failure in either
   direction is a violation.

The pass operates on leaf nodes — the ones the logic engine consumes
— rather than intermediate lowering nodes, because only leaves carry
the full refinement chain. A failure on a leaf indicates that
something between extraction and final-form was lost.

## Render IR → Natural Language

A small constrained-output LLM call produces the rendering:

```text
render_node(node) -> string

prompt:
    "Render the following IR node into a single sentence of plain English
     that preserves all of its content. Do not add information not present
     in the IR. Polarity, modality, and quantitative arguments must be
     reflected explicitly."

constraints:
    output is a single sentence
    output mentions every ground-atom argument of node.term
    polarity is reflected (Affirmed -> "...", Denied -> "no ..." /
        "denies ...", Uncertain -> "possibly..." / "may have...")
    modality is reflected (Past -> "previously", FamilyHistory -> "father / mother
        / etc.", RuledOut -> "ruled out", ...)
```

The rendering function is *deliberately weak*. Its job is to be a
faithful but trivial paraphrase, not a clever rewrite. Cleverness here
masks IR loss; trivial paraphrasing exposes it.

The rendering function's prompt is versioned alongside the trigger
taxonomy and recorded in the audit trail.

## Entailment in Both Directions

Given the source span text *S* and the rendered IR *R*, the pass asks
a textual-entailment (NLI) model:

```text
entails(premise, hypothesis) -> Boolean

pass_check =
    entails(S, R) AND entails(R, S)
```

The NLI model is **separate from the rendering model and separate from
the extractor**. Three reasons:

1. **Independence.** Using the same model for two of the three roles
   creates a self-confirmation loop. The same model that produced a
   lossy IR will gladly confirm that its rendering of the IR matches
   the source.
2. **Specialization.** Modern NLI models are small, fast, and
   purpose-trained for the entailment task. A general-purpose LLM is
   wasteful and less reliable here.
3. **Auditability.** Three separately versioned components make the
   pass's behavior reproducible. If the entailment model is upgraded,
   the audit trail records which version was used for each adjudication.

The pass returns a violation on either-direction failure. The violation
includes the source span, the rendering, and a note of which direction
failed.

## Why Both Directions?

A one-direction check (e.g., does the IR entail the source?) catches
*invented* content but not *omitted* content. Both directions are
required to catch both classes:

- IR entails source, but source does not entail IR → the IR added
  information not in the source. (Less common but real — extractors
  sometimes "fill in" plausible details.)
- Source entails IR, but IR does not entail source → the IR omitted
  information from the source. (The "severe chest pain → chest pain"
  case.)

Bidirectional entailment treats both as failures. Equivalence — what
the framework actually wants — is the conjunction.

## False Positives and Strictness Levels

Real entailment is graded; perfect bidirectional entailment is hard to
achieve for any non-trivial paraphrase. The pass exposes a
**strictness** parameter:

```text
StrictnessLevel :=
    Tight       -- both directions must score >= 0.95 (NLI confidence)
  | Standard    -- both directions >= 0.80
  | Permissive  -- both directions >= 0.65
  | AuditOnly   -- log results but do not block
```

`Standard` is the default. Domains with high stakes (medical, legal)
may move to `Tight`; domains with lots of natural variation (customer
support, intake forms) may use `Permissive`.

The threshold is part of the configuration recorded in the audit trail.

## Cost Optimization

Round-trip is the most expensive of the static passes (three model
calls per leaf node, in the worst case). Two optimizations are
permissible:

1. **Cache the renderer.** If two adjudications produce structurally
   identical IR nodes (same term, polarity, modality), the renderer's
   output is cached. The cache key is the node's structural hash, not
   its id.
2. **Batch the entailment.** Many NLI models accept batch inputs. The
   pass should batch leaves into entailment calls. Latency dominates
   per-call; batching dominates throughput.

Neither optimization changes correctness. Both are documented in the
audit-trail metadata.

## The Output of a Round-Trip Violation

A `RoundTripDrift` violation (per `ADJ06`) carries:

```text
Violation := {
    node_id:           NodeId,
    source_span_text:  string,
    ir_rendered:       string,
    direction_failed:  SourceEntailsIR | IREntailsSource | Both,
    nli_score_S_to_R:  Real in [0, 1],
    nli_score_R_to_S:  Real in [0, 1],
    suggestion:        string,
}
```

The clarification generator turns this into the standard `RoundTripDrift`
question from ADJ06:

> "Does *'\<ir_rendered\>'* mean the same as *'\<source_span_text\>'*?
>  If not, how would you rephrase?"

## Interaction with Lowering

If an IR document has lowered nodes (`lowered_from` chains), the pass
checks the *leaf* of each chain, not the root. Two reasons:

1. The lowering process should preserve meaning, by construction. If
   it doesn't, the lowering rules (in `ADJ01`) are wrong.
2. The logic engine consumes leaves. Round-trip is the last check
   before engine input, and it should be evaluated where engine input
   sits.

The non-leaf nodes are still subject to coverage (ADJ02) and polarity
(ADJ03), which are span-based and not affected by lowering.

## Worked Example

Source: *"Severe chest pain, 9/10, sudden onset."*

Extractor produces:

```text
Fact { term: chest_pain(patient),
       polarity: Affirmed, modality: Present,
       source_spans: [(doc1, 0, 38)] }
```

ADJ02 (coverage): the span includes "Severe", "9/10", and "sudden
onset", each tagged Meaningful by the clinical tagger. They are all
inside `source_spans`. **Pass.**

ADJ03 (polarity/modality): no negation, no hedge, no family or
temporal cues. **Pass.**

ADJ04 (round-trip):

Render IR → "The patient has chest pain."

Entailment:
- *"Severe chest pain, 9/10, sudden onset"* entails *"The patient has
  chest pain"* — yes, with high confidence.
- *"The patient has chest pain"* entails *"Severe chest pain, 9/10,
  sudden onset"* — no, the original has more information.

Bidirectional entailment fails. **Violation.**

ADJ06 generates a clarification:

> "Does *'The patient has chest pain'* mean the same as *'Severe chest
> pain, 9/10, sudden onset'*? If not, how would you rephrase?"

Rung 0 re-prompts the extractor with the violation: *"The IR rendering
omitted 'severe', '9/10', and 'sudden onset'. Re-emit the IR with these
preserved as structured fields."*

The extractor produces:

```text
Fact { term: chest_pain(patient,
                        severity=severe,
                        pain_score=9,
                        onset=sudden),
       polarity: Affirmed, modality: Present,
       source_spans: [(doc1, 0, 38)] }
```

ADJ04 re-runs:

Render IR → "The patient has severe sudden-onset chest pain rated 9 out of 10."

Bidirectional entailment now succeeds at the `Standard` strictness
threshold. **Pass.** The adjudication continues.

Without ADJ04, the engine would have reasoned over the impoverished IR
without realizing the severity and onset information was lost, and any
rule conditioned on severity or onset would have been silently bypassed.

## Open Questions

1. **NLI model choice.** Several open-source NLI models are competitive
   on clinical text (BioBERT-NLI, etc.). The framework's reference
   implementation will pick one and document the choice. Domains can
   override.
2. **Quantitative comparison.** "9/10" vs "9 out of 10" vs "severe" —
   when do these entail one another? NLI models are surprisingly
   capable here but not perfect. May need domain-specific equivalence
   rules layered on top.
3. **Multi-sentence source spans.** If a node's source spans cover
   multiple sentences, the rendering may need to be multi-sentence
   too. Current spec assumes single sentences; multi-sentence support
   is `ADJ04a`.
4. **Languages other than English.** Cross-language NLI is a research
   area. Out of scope for the first paper.

## Limitations

1. **NLI is fundamentally probabilistic.** A high-confidence entailment
   judgment is not a proof. The pass reduces drift; it does not
   eliminate it.
2. **The rendering model's quality is a dependency.** A weak renderer
   produces drift artifacts that look like IR errors. The fix is good
   renderer prompting and versioning.
3. **Strictness tuning is empirical.** No closed-form rule says which
   threshold is right for a given domain. The framework provides
   `AuditOnly` mode to gather data before committing.

## Status

Draft. Sufficient to implement directly. `ADJ04a` (multi-sentence
spans), `ADJ04b` (quantitative equivalence rules) are explicit follow-
ups.
