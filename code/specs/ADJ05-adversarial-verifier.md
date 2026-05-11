# ADJ05 — Adversarial Verifier: Find the Strongest Reading That Contradicts the IR

## Overview

This is the fourth and final of the checker passes from [`ADJ00`](ADJ00-adjudication-framework.md).
After the three earlier passes have ruled out the structural and
semantic failure modes they target, ADJ05 asks a deliberately
asymmetric question:

> Assume the IR is wrong. Find the **strongest reading** of the source
> span that contradicts it. If you cannot find one, say so.

This is **adversarial verification**, not adversarial training. No
network weights are updated. A separate LLM, prompted to play
prosecutor rather than fair reviewer, attacks the IR at inference time.
Its findings either pass the IR through or trigger clarification.

The cultural template is medical M&M conferences, tumor boards,
appellate review, and academic peer review — institutions that have
already learned that one decision-maker plus one devil's advocate beats
two decision-makers. The framework reproduces that organizational
structure in software.

This pass catches novel failure modes that the earlier passes do not
anticipate. ADJ02 catches silent omission. ADJ03 catches scope errors.
ADJ04 catches lossy paraphrase. ADJ05 catches *whatever else* — the
class of failures that has no rule-based detector.

## Layer Position

```
   ADJ02 coverage
        │
        ▼
   ADJ03 polarity/modality
        │
        ▼
   ADJ04 round-trip entailment
        │
        ▼
   ADJ05 adversarial verification   ← this document
```

ADJ05 runs last because it is the most expensive (a full LLM call per
node, sometimes with retrieval) and because it depends on the earlier
passes having already caught their respective failure classes — there
is no point asking an adversary to find subtle reading errors when a
trivial polarity flip is present.

## What "Adversarial" Means Here

This is **inference-time adversarial verification**, in the lineage of:

- **AI Debate** (Irving, Christiano, Amodei 2018) — two models argue,
  judge decides.
- **Constitutional AI** (Bai et al. 2022) — a model critiques its own
  outputs against principles.
- **Adversarial NLI** (Nie et al. 2020) — humans iteratively craft
  examples that break NLI models; ADJ05 automates the role of human.

It is **not**:

- A training-time GAN. No gradients flow. No weights update. The
  adversary is a fixed model at the time of each adjudication.
- A reasoning trick. The adversary does not "think harder." It is
  given a deliberately asymmetric prompt — find a contradicting reading
  — and its output is one of: agreement, a contradicting reading, or
  abstention.

The framing matters when explaining to reviewers: the technique is
well-bounded and well-precedented; the novelty is putting it inside the
typed-IR pipeline as a final gate.

## The Adversary's Prompt

The adversary receives the source span text *only*. It does not see the
extractor's IR. Its prompt is asymmetric:

```text
prompt:
    "You are reviewing a clinical [or other domain] note. A reader has
     produced the following claim from this passage:

         <ir_rendered>

     Source passage:
         <source_span_text>

     Your job is to find the strongest reading of the passage that
     contradicts this claim. If multiple contradicting readings exist,
     return the most plausible. If no contradicting reading is
     plausible, return 'CONCURS'.

     A contradicting reading is one that:
       1. Is grammatically and semantically defensible given the passage
       2. Differs from the claim in polarity, modality, subject,
          quantity, severity, or temporal scope
       3. Would change downstream reasoning if accepted as correct

     Return only the contradicting reading (one sentence) or 'CONCURS'."
```

The asymmetry — *"find the strongest contradicting reading"* — is the
point. A symmetric "review this claim" prompt produces uninteresting
agreement. The adversarial prompt forces the model to look hard for
disagreement.

## The Plausibility Judge

The adversary may produce a contradicting reading that is technically
defensible but implausibly remote (e.g., interpretations no clinician
would actually entertain). A *third* model, separate from both extractor
and adversary, decides whether the adversary's reading is plausible:

```text
prompt:
    "A clinical note contained the following passage:
         <source_span_text>

     An extractor read this as:
         <ir_rendered>

     A verifier suggests an alternative reading:
         <adversary_reading>

     Is the verifier's alternative reading **plausible** —
     i.e., would a competent clinician in everyday practice
     interpret the passage this way?

     Return 'PLAUSIBLE' or 'IMPLAUSIBLE' with a single short reason."
```

A `PLAUSIBLE` judgment means the adversary has found a real ambiguity;
ADJ05 raises a violation. An `IMPLAUSIBLE` judgment means the adversary
was being too clever; the violation is suppressed but logged.

The triangulation — extractor, adversary, judge, three separate models
— is what prevents the adversarial pass from degenerating into a
nitpicking machine that flags everything. Reviewers will rightly ask:
*"How do you avoid an adversary that wins by being silly?"* The
plausibility judge is the answer.

## Three-Model Independence

ADJ04 used three models for renderer / NLI. ADJ05 uses three for
extractor / adversary / judge. The recommendation is:

| Role | Model family suggestion | Why |
|---|---|---|
| Extractor | Frontier model, JSON-mode | Best structured-output quality |
| Adversary | Different family, free-text | Avoids shared blind spots |
| Judge | Smallest competent model | Cheap, focused on a binary decision |

A deployment that uses the same model family for all three roles will
collude on its own biases. The framework cannot prevent this by
design — model-family configuration is up to the deployer — but it
strongly recommends in spec and provides per-role configuration.

## Cost and Throughput

ADJ05 is the most expensive pass. Two LLM calls per leaf node
(adversary + judge), each with the full source span context. The pass's
cost profile is:

- ~10–20× ADJ04's per-node cost.
- ~100× ADJ02's per-node cost.

This is acceptable because ADJ05 runs **only on nodes that already
passed ADJ02–04**. The earlier passes act as a funnel that filters out
the easy failure modes cheaply, leaving only the candidates worth
adversarial review.

Two optional cost reductions:

1. **Sample, do not exhaustively check.** For high-throughput
   adjudications, ADJ05 can be configured to run on a random
   sample (e.g., 10%) of leaf nodes, with the unselected nodes accepted
   without adversarial review. The sample rate is recorded in the
   audit trail and is itself a configurable strictness parameter.
2. **Cache the adversary's response per (span, ir_rendered) pair.**
   Same input, same output. The structural-hash cache from ADJ04
   generalizes.

For the first paper's evaluation runs, exhaustive ADJ05 is preferred so
the empirical catch rates are meaningful.

## What the Pass Returns

```text
AdversarialResult :=
    Concurs                                -- adversary returned CONCURS
  | ContradictionImplausible(reading,
                              judge_reason)
  | ContradictionPlausible(reading,
                            judge_reason)   -- violation
```

A `ContradictionPlausible` result generates an `AdversarialReading`
clarification (per `ADJ06`):

> "The verifier suggests an alternative reading:
>  *'\<reading\>'*.
>  The current IR says:
>  *'\<ir_rendered\>'*.
>  Which more accurately reflects *'\<source_span_text\>'*?"

A `ContradictionImplausible` result is logged in the audit trail
(useful telemetry for tuning the plausibility judge) but does not block
the adjudication.

## Adversary Failure Modes — Designed Against

Three failure modes are real and the spec addresses each:

### Shared-bias collusion

If extractor and adversary share a base model, they share blind spots
and never surprise each other. Mitigation: model-family diversity
recommended explicitly; the audit trail records each role's model. A
deployment that fails to diversify is making a documented choice.

### Adversary-wins-by-being-silly

An adversary that flags every input as "actually, it could be the
opposite" is useless — every input cycles through clarification, the
system degrades to nothing. Mitigation: plausibility judge with its
own asymmetric prompt asking for plausibility, not balance.

### Reviewer pushback ("self-play proves nothing")

Reviewers will object to two-LLM setups as untrustworthy without
external grounding. Mitigation: every ADJ05 deployment is paired with
an evaluation against held-out **human-labeled errors**. The
adversary's precision and recall on a labeled benchmark are reported.
Without that grounding, ADJ05 is opinion; with it, ADJ05 is empirical.

## Logging and the Audit Trail

Every ADJ05 result is logged regardless of outcome:

```text
AdversarialLog := {
    node_id:               NodeId,
    source_span:           Span,
    ir_rendered:           string,
    adversary_response:    "CONCURS" | AdversaryReading,
    adversary_model:       string,        -- versioned
    judge_response:        Option<"PLAUSIBLE" | "IMPLAUSIBLE">,
    judge_reason:          Option<string>,
    judge_model:           string,        -- versioned
    sampled_in:            bool,          -- whether this run sampled this node
    timestamp:             ISO-8601,
}
```

The log is part of the audit trail (per `ADJ07`). Two motivations:

1. **Telemetry.** Over time, the adversary-flag rate and the
   plausibility-flag rate provide signal for tuning prompts and choosing
   models.
2. **Defensibility.** Auditors of high-stakes adjudications can see
   not only what the extractor produced but also what the adversary
   considered, even if the adversary concurred. *The adversary
   concurring is itself useful evidence.*

## Worked Example: The Hidden Family History

Source: *"PMHx: HTN, DM2, MI in 50s."* (Past Medical History:
hypertension, type 2 diabetes, myocardial infarction in 50s)

Extractor produces:

```text
Fact { term: hypertension(patient), polarity: Affirmed, modality: Past, ... }
Fact { term: dm2(patient),          polarity: Affirmed, modality: Past, ... }
Fact { term: mi(patient),           polarity: Affirmed, modality: Past, ... }
```

ADJ02–04 all pass. Each fact is covered, polarity is Affirmed,
modality is Past (the "PMHx" header marker), round-trip succeeds.

ADJ05 runs on each leaf. For the MI fact:

Adversary prompt sees the source span and the rendered IR ("The patient
had a myocardial infarction in their 50s."). The adversary returns:

> *"The 'in 50s' may refer to a family member's MI, not the patient's
> own. Many notes abbreviate family history within PMHx headers when
> the clinician's prior note structure conflated them, especially when
> the patient is currently in their 30s or 40s."*

Plausibility judge: this is a known clinical-documentation gotcha.
Returns **PLAUSIBLE**. ADJ05 emits a violation.

Clarification: *"The verifier suggests this MI may belong to a family
member ('in 50s' is sometimes a family-history abbreviation). The
current IR attributes it to the patient. Which is correct?"*

Rung 2 surfaces the question to the on-shift clinician, who replies:
*"This is the patient's MI; PMHx header in this note format means
patient's own history."* The IR is preserved as-is. The dialogue is
logged. The adjudication continues.

Without ADJ05, the patient's record would carry an MI that may have
been a family member's. With ADJ05, the ambiguity is surfaced — and
even when the adversary turns out to be wrong, the audit trail records
that the question was considered.

## Open Questions

1. **Adversary specialization.** A general-purpose LLM playing
   adversary is good but not great. A purpose-trained adversary fine-
   tuned on confirmed extraction errors should outperform. Training
   data comes from the deployment's own dialogue logs (per ADJ06).
   `ADJ05a`.
2. **Multi-step adversarial reasoning.** The current spec is one-shot.
   A multi-turn adversary that probes ("If I'm wrong, what would
   change?") may catch more. Out of scope.
3. **Ensemble adversaries.** Several adversaries with different
   prompts, votes counted. Higher cost; possibly higher recall.
   `ADJ05b`.
4. **Calibrating confidence.** The adversary returns a discrete
   reading; the judge returns plausible / implausible. Probabilities
   are not exposed. Future: surface confidence so downstream tooling
   can rank ambiguity severity.

## Limitations

1. **The adversary is bounded by its base model.** Errors the model
   does not "see" are not caught.
2. **The plausibility judge can also be wrong.** A real failure mode
   is the judge marking a genuine ambiguity as implausible. The fix is
   periodic human re-review of `ContradictionImplausible` logs.
3. **No reasoning trace.** Unlike formal-methods verifiers, ADJ05
   produces a yes/no with a natural-language reading. Reviewers can
   inspect but not formally verify. This is the fundamental tradeoff
   of using LLMs as verifiers.

## Status

Draft. Sufficient to implement directly given a deployment-specific
choice of three models. The reference implementation will use distinct
model families for extractor / adversary / judge and document each
choice in the audit trail.
