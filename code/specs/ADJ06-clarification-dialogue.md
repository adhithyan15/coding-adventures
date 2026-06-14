# ADJ06 — Clarification Dialogue: Type-System-Generated Questions and an Escalation Ladder

## Overview

When any of the four checker passes (`ADJ02`..`ADJ05`) rejects an IR
document, the framework does not return an error. It generates a
**clarification question** whose shape is determined by *which* check
failed, and routes that question through an **escalation ladder** that
tries cheap automated answers before escalating to a human.

This is the spec for both halves of that mechanism:

1. **How clarification questions are generated** from a checker
   violation — including the question taxonomy and the surface-language
   templates that produce them.
2. **How the dialogue is routed** through the escalation ladder, what
   each rung does, and how the result is recorded.

Clarification is a *first-class output* of the framework. It is not an
error path; it is the normal way the system handles ambiguity. The
Socratic dynamic — attending asking resident, judge asking counsel,
auditor asking accountant — is the well-trodden cultural template for
trustworthy decisions in exactly the domains this framework targets,
and the framework reproduces it in software.

## Layer Position

```
   ADJ02..ADJ05 checker passes
        │     │     │     │
        └─────┴─────┴─────┘
                  │
                  ▼ violations
              ADJ06 clarification    ← this document
                  │
        ┌─────────┼─────────┐
        ▼         ▼         ▼
    rung 0     rung 1     rung 2     rung 3
    LLM       EHR /       user       human
    re-prompt context     prompt     expert
    re-query              UI
```

ADJ06 sits between the checkers and the user/clinician/expert. It owns
the question taxonomy, the question-generation templates, the routing
policy across rungs, and the dialogue-log schema that records every
exchange for the audit trail.

## The Question Taxonomy

Each checker failure produces a question of a specific class:

```text
ClarificationKind :=
    UncoveredSpan       -- ADJ02 found a meaningful span no node accounted for
  | PolarityConflict     -- ADJ03 found a trigger that disagrees with polarity
  | ModalityConflict     -- ADJ03 found a trigger that disagrees with modality
  | RoundTripDrift       -- ADJ04 found the IR doesn't round-trip to source
  | AdversarialReading   -- ADJ05 found a plausible contradicting reading
  | AmbiguousReference   -- entity resolution (e.g., "the patient" → who?)
  | UnparseableSpan      -- coverage Unparseable discard reason
```

Each class has a fixed question shape. The framework does **not** ask
the LLM to generate the question text; the question text is template-
generated from the violation data, with the violation's span and trigger
filled in. This keeps clarification questions *consistent, auditable,
and trainable* — the same violation always produces the same question
shape.

## Question Templates

```text
UncoveredSpan(span_text):
    "You did not account for: '{span_text}'.
     What does it mean in this context?"

PolarityConflict(span_text, trigger, term_summary, actual, required):
    "In the span '{span_text}', the cue '{trigger.surface}' suggests
     {required}, but the IR is {actual}.
     Is the {term_summary} being affirmed, denied, or uncertain?"

ModalityConflict(span_text, trigger, term_summary, actual, required):
    "In the span '{span_text}', the cue '{trigger.surface}' suggests
     {required} modality, but the IR is {actual}.
     Does this {term_summary} apply to the patient now, in the past,
     to a family member, or has it been ruled out?"

RoundTripDrift(span_text, ir_rendered):
    "Does '{ir_rendered}' mean the same as '{span_text}'?
     If not, how would you rephrase?"

AdversarialReading(span_text, ir_rendered, adversary_reading):
    "The verifier suggests an alternative reading:
     '{adversary_reading}'.
     The current IR says: '{ir_rendered}'.
     Which more accurately reflects '{span_text}'?"

AmbiguousReference(referring_expression, candidates):
    "The expression '{referring_expression}' could refer to:
     {candidates}.
     Which one is intended here?"

UnparseableSpan(span_text):
    "The span '{span_text}' could not be parsed into a structured fact.
     Could you rephrase it, or confirm it can be discarded?"
```

`term_summary` is a human-readable rendering of the IR node's term —
"chest pain" rather than `chest_pain(patient)`. The rendering function
is configurable per domain (clinical text uses one rendering, legal text
uses another).

## The Escalation Ladder

A clarification can be answered by any of four sources, in order of
cost. The framework tries each rung in turn until one succeeds.

### Rung 0 — Re-prompt the extractor

The cheapest response. The framework re-prompts the extractor with the
specific violation as a constraint:

> *"Re-examine the span '\<span\>'. The cue '\<trigger\>' indicates
> \<required\>. Re-emit the IR with this corrected."*

The re-prompt is purely mechanical — no human in the loop. The extractor
either produces a corrected IR that passes the failed check, or fails
again. The framework allows N re-prompt attempts (default 3) before
escalating. Each attempt is logged.

This rung handles the largest share of violations in practice: extractor
errors that arise from prompt nondeterminism rather than genuine
ambiguity in the input.

### Rung 1 — Re-query the source for adjacent context

The framework looks at adjacent text in the source document for context
that might resolve the ambiguity. For clinical notes, the EHR may
contain related records (prior visits, problem list, family history
section). For legal documents, related contracts or amendments. For TSA
declarations, the boarding pass or itinerary.

This rung is implemented as a domain-specific *context-fetcher* that
returns additional text snippets. The framework then re-runs extraction
including the fetched context. Rung 0 may then re-run on the enriched
input.

Rung 1 is optional per adjudication. If the domain has no enrichment
source available, the framework skips to rung 2.

### Rung 2 — User-facing question

The clarification question is surfaced to the user — the person who
originated the input — through a domain-specific UI. For a TSA
checkpoint, this might be a verbal question to the passenger. For an
intake form, a follow-up field. For a customer-support ticket, a reply
message.

The framework provides:

1. The exact question text (template-rendered).
2. The span(s) being clarified, suitably highlighted.
3. An optional structured response form (e.g., radio buttons for
   *Affirmed / Denied / Uncertain*).

The user's response becomes a new document span (per `ADJ01`) appended
to the same `DocumentId`, and extraction re-runs.

### Rung 3 — Human-expert escalation

If rungs 0–2 cannot resolve the ambiguity, the framework routes the
question to a domain expert (clinician, attorney, CPA). The expert
sees the original span, the IR's interpretation, all rung-0 / rung-1 /
rung-2 exchanges, and provides an authoritative answer.

The expert's answer is logged with their identifier and timestamp.
Their interpretation is treated as ground truth and the IR is updated
accordingly.

## Routing Policy

The default policy is *strict-cheap-first*:

```text
for failure in checker_violations:
    for rung in [Rung0, Rung1, Rung2, Rung3]:
        result = rung.attempt(failure, dialogue_so_far)
        if result == Resolved:
            log_resolution(failure, rung, result)
            break
        if result == Failed:
            log_attempt(failure, rung)
            continue
        if result == NotApplicable:
            continue
```

Per-deployment configuration may override the policy:

- **Skip rung 1** if no domain context-fetcher is wired up.
- **Skip rung 2** for back-office batch adjudications where no live user
  is available.
- **Start at rung 3** for adjudications flagged as high-stakes where
  human review is mandatory regardless.
- **Time-bound each rung** to prevent indefinite waiting on a missing
  user response.

The policy and its overrides are versioned configuration recorded in the
audit trail.

## Dialogue-Log Schema

Every clarification exchange is recorded. The schema is intentionally
small so that downstream tooling, replay, and human review are
straightforward.

```text
DialogueTurn := {
    turn_id:    integer,                  -- monotonic within an adjudication
    failure:    Violation,                -- the violation that triggered this
    rung:       Rung0 | Rung1 | Rung2 | Rung3,
    question:   {
        kind:     ClarificationKind,
        text:     string,                 -- rendered question
        spans:    [Span],                 -- highlighted regions
    },
    response:   {
        source:   Extractor | EHR | User | Expert,
        text:     string,                 -- the response, raw
        actor_id: Option<string>,         -- expert's id, EHR system id, etc.
        timestamp: ISO-8601,
    },
    outcome:    Resolved | Failed | TimedOut,
    new_spans:  [Span],                   -- spans appended to the document by this turn
}

Dialogue := [DialogueTurn]
```

The dialogue is **part of the IR document**. New text from rungs 2 and
3 becomes part of the document's normalized text, with span offsets
continuing into the appended region. This is the property described in
`ADJ01`: a document's lifecycle spans multiple clarification turns,
under a stable `DocumentId`.

## What Counts as Resolution

A clarification is **resolved** when:

1. The relevant check, re-run after the dialogue turn, **passes**.
2. The new check pass touches *only* the violation that triggered the
   dialogue (i.e., the response did not introduce new violations).

Condition (2) prevents runaway dialogue cascades. If a response
introduces a *new* violation, the framework opens a new dialogue turn
for it but tracks the cascade depth. A configurable cap (default 5)
forces escalation to rung 3 if the cascade is too deep.

## Failure of Resolution

If all rungs are exhausted without resolution, the adjudication
terminates with a **"clarification could not be resolved"** outcome.
The dialogue log, the IR document at the point of termination, and the
list of unresolved violations are all preserved in the audit trail.
This outcome is a valid system response — *the framework's idea of "I
don't know" is the dialogue log explaining why*.

This is important. A clarification-exhausted outcome is **more
defensible**, not less, than a system that silently guesses. Auditors,
regulators, and downstream reasoners prefer it.

## Worked Example

Continuing the TSA example. Input has the span *"three lithium camera
batteries"*. Coverage and polarity both pass. ADJ02 (coverage) recorded
no violations. ADJ03 (polarity/modality) recorded no violations.
ADJ04 (round-trip) runs and the entailment model reports drift: the
extractor produced `Fact { term: carry_on_item(lithium_battery,
count=3), ... }` but the round-trip rendering omitted the
"watt-hours" specification critical to the TSA rule.

ADJ06 generates a `RoundTripDrift` clarification:

> "Does *'lithium camera battery, count 3'* mean the same as *'three
> lithium camera batteries'*? If not, how would you rephrase?"

**Rung 0** (re-prompt extractor): the extractor is told that the round-
trip is missing watt-hours information. It re-prompts itself and
produces a refined extraction asking for watt-hours implicitly:

```text
Uncertainty { term: carry_on_item(lithium_battery,
                                  count=3,
                                  wh=unknown),
              ... }
```

But this introduces a new violation: ADJ02 may flag that "watt-hours"
was not in the original input. The cascade-depth counter ticks up.

**Rung 2** (user-facing): the passenger is asked:

> "How many watt-hours is each battery rated for?
>  (Look for 'mAh' or 'Wh' on the battery's label.)"

The passenger responds *"80 Wh each"*. The response is appended to the
document; extraction re-runs; the IR becomes:

```text
Fact { term: carry_on_item(lithium_battery, count=3, wh=80),
       polarity: Affirmed,
       modality: Present,
       source_spans: [(doc1, 60, 92), (doc1, 245, 257)] }
```

ADJ02, ADJ03, ADJ04, ADJ05 all pass on the re-run. The dialogue is
resolved. The audit trail records the entire exchange: original span,
extractor's first attempt, round-trip drift, re-prompt failure,
user-facing question, passenger response, resolution.

Downstream, the logic engine consults the TSA rule for lithium
batteries: ≤ 100 Wh batteries are permitted in carry-on, up to two
spare batteries. The query produces a verdict and a proof DAG citing
the rule. The audit trail now chains:

```text
original input span 60..92  →  IR Fact F3  →  TSA rule R7
                                                    │
                                                    ▼
                                              proof DAG step
                                                    │
                                                    ▼
                                              verdict: "Permitted but
                                                       only 2 spare;
                                                       the third must
                                                       be in a device."
```

Every link is materialized data.

## Open Questions

1. **Tone and register.** Clinical clarifications need a clinical
   register; passenger-facing clarifications need plain English. The
   template engine should pick a register per domain. Out of scope for
   the first paper.
2. **Question batching.** Multiple violations on a single document may
   produce multiple clarifications; surfacing them one at a time is
   tedious. A batching strategy that groups related violations into a
   single user-facing prompt is desirable. `ADJ06a`.
3. **Adversarial response handling.** A user may answer a clarification
   in a way that introduces *new* violations on purpose, e.g., to
   confuse the system. The cascade-depth cap is a coarse defense;
   intent detection is `ADJ06b`.
4. **Multi-modal clarifications.** "Can you show a photo of the
   battery's label?" Some domains will need image / file responses, not
   just text. Out of scope; could be added via the response.source =
   User route with a configurable response format.

## Limitations

1. The framework's clarification *prompts* are template-generated. The
   *underlying ambiguity* is what it is; we cannot turn an
   irreducibly-ambiguous input into a clear one through prompting.
2. Rung 1 (EHR / context re-query) requires per-domain integration
   work. It is the most powerful rung but the most operationally
   expensive to set up.
3. Rung 3 (expert) is the slowest and most expensive. Routing volume to
   rung 3 should be minimized through good rung 0–2 design, but never
   eliminated — high-stakes adjudications always have a place for
   expert review.

## Status

Draft. Sufficient to implement directly. `ADJ06a` (question batching)
and `ADJ06b` (adversarial response detection) are explicit follow-ups
tied to deployment experience.
