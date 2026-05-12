# ADJ09 — Rule-Compilation Pipeline: Rulebook → IR with the Same Checker Discipline

## Overview

The framework's two pipelines are symmetric:

```text
Input text   ─┐
              ├──── typed IR ───── checker passes ─── logic engine
Rulebook text─┘
```

The **input** pipeline (extraction) is what every previous spec
(ADJ02–07) is concerned with. The **rule** pipeline is the other half:
taking a written rulebook (a clinical guideline, a tax regulation, a
license text, the TSA carry-on rules) and compiling it into IR `Rule`
nodes that the logic engine can consume.

This spec defines the rule pipeline, including its **specific
differences** from the input pipeline. The high-level claim is:

> Rulebook compilation uses the same coverage / polarity / round-trip /
> adversarial discipline as input extraction. Compiled rules carry the
> same source-span provenance back to their rulebook origins. An
> expert who reviews the IR is *reviewing* — not *authoring* — which is
> meaningfully faster.

This is the workflow that collapses the *"you need a domain-expert
coauthor to write the rules"* objection. The expert reviews; the LLM
plus checker passes does the heavy lifting.

## Layer Position

```
   Rulebook text (a clinical guideline, tax code, ...)
        │
        ▼
   ADJ09 rule-compilation pipeline   ← this document
        │   uses the same ADJ02..ADJ05 passes as extraction
        │   plus rule-specific extensions
        ▼
   IR Rule nodes (per ADJ01)
        │
        ▼
   ADJ11 LP19 connector  ─────▶  LP19 engine
```

## What the Pipeline Produces

The output is a set of IR Rule nodes, each in one of the four subtypes
declared by ADJ01:

```text
Rule { term: definitional(head, body), polarity, modality, source_spans, ... }
Rule { term: constraint(body),         polarity, modality, source_spans, ... }
Rule { term: default(head, body, exceptions), polarity, modality, ... }
Rule { term: probabilistic(p, head, body), polarity, modality, ... }
```

The subtype is encoded in the term, not in the kind, so the well-
formedness check (ADJ01) stays unchanged. The Rule's `metadata` carries
the **mandatory** `as_of` ISO-8601 stamp and may carry per-source
metadata (e.g., for a regulatory citation: subsection, version,
publication date).

## Pipeline Stages

> **Note on Stage 0.** ADJ09 assumes the rulebook text already exists.
> When it doesn't — i.e., no published regulation is available, or
> the demo wants to bootstrap a rulebook from the LLM's own training
> data — [ADJ14](ADJ14-rule-elicitation.md) defines a Stage 0
> *Rule Elicitation* phase whose output (`Rulebook.source_text`) is
> the input to Stage 1 below. A Stage-0-sourced rulebook is marked
> `Tentative` until an expert review (per §"Expert Review Workflow")
> promotes it to `Reviewed`. The Stages 1–6 in this document are
> source-agnostic: an external regulatory document and a Stage 0
> elicitation both feed Stage 1 identically.

### Stage 1 — Segmentation

The rulebook is segmented into adjudicable units. A unit is the
smallest text span that produces at least one Rule node. Typical units:

- A numbered rule (e.g., 49 CFR § 1540.111(a))
- A bulleted list item
- A subsection of a guideline
- A single sentence in a license clause

Segmentation is *itself* an LLM-assisted task (LLMs are good at finding
section boundaries). The chosen segments are versioned alongside the
tagger; re-segmenting is a re-compilation.

### Stage 2 — Per-Segment Compilation

Each segment is compiled into one or more Rule nodes. The extractor
LLM is prompted with the segment text plus a typed schema for each
Rule subtype and produces structured output. The schema constrains the
output to be a valid Rule per ADJ01.

Concrete example — a rulebook segment:

```text
TSA-PROHIBITED §3:
Knives, including but not limited to pocket knives, are prohibited
in carry-on baggage if the blade exceeds 2.36 inches (60 mm) in
length, regardless of whether the blade locks.
```

Compiles to:

```text
Rule {
    term: definitional(
        prohibited(pocket_knife(BladeLength)),
        [Pos(carry_on_item(pocket_knife, blade_length=BladeLength)),
         Pos(gt(BladeLength, quantity(2.36, in)))]
    ),
    polarity: Affirmed,
    modality: Present,
    source_spans: [(tsa-rulebook-2025, §3-byte-range)],
    metadata: { as_of: "2025-09-01", citation: "TSA-PROHIBITED §3" }
}
```

### Stage 3 — Coverage Check (ADJ02 Adapted)

Every section, subsection, and paragraph of the rulebook must be
accounted for by either a Rule node or an explicit `Discarded` marker
(for preambles, examples, commentary). The same coverage discipline
from ADJ02 applies, with one extension: at the rulebook level, the
tagger's `NonMeaningfulReason` vocabulary adds:

```text
RulebookNonMeaningfulReason :=
    Preamble                  -- "Whereas..."
  | NonNormativeExample        -- "For instance, ..."
  | CrossReference             -- "see also §5"
  | StatutoryAuthority         -- "Under the authority of..."
  | EffectiveDateBoilerplate   -- "This rule takes effect 30 days..."
  | DefinitionalSentence       -- handled by separate definition table
```

Sections matching one of these reasons must be explicitly Discarded
with the appropriate reason. The compiled IR has 100% accountability
for the rulebook bytes.

### Stage 4 — Polarity / Modality Check (ADJ03 Adapted)

Each compiled Rule's polarity and modality are checked against scope
triggers in its source segment. The trigger taxonomy from ADJ03
applies, with two additions for legal/regulatory text:

```text
Additional triggers for rulebook compilation:
  - Obligation:  "shall", "must", "is required to"     -> Affirmed
  - Prohibition: "shall not", "may not", "is prohibited" -> Denied
                  (or rather, the rule is "prohibited(X) :- ...")
  - Permission:  "may", "is permitted to"               -> Affirmed
  - Conditional: "if ... then ...", "provided that ...", "except"
                                                          -> Conditional
```

The pass enforces: a segment containing "shall not" must produce a
Rule whose head is `prohibited(...)` or equivalent — not a positive
permission. The check catches the same class of polarity-flip error as
in the input pipeline.

### Stage 5 — Round-Trip Entailment (ADJ04 Adapted)

Each compiled Rule is rendered back to natural language and the
rendering is bidirectionally entailed against the source segment. The
same machinery as ADJ04, with one note: rule renderings tend to be
longer and more structured than fact renderings, so the strictness
threshold may need to be relaxed slightly. Configuration is per-domain.

### Stage 6 — Adversarial Reading (ADJ05 Adapted)

A separate adversary attempts to find a reading of the rulebook
segment that contradicts the compiled rule. For rules, the adversary's
prompt is specialized:

```text
"You are reviewing a regulatory text. A reader has compiled the
 following rule from this segment:

     <rule_rendered>

 Source segment:
     <segment_text>

 Your job is to find the strongest reading of the segment that would
 make the compiled rule wrong — either too broad (rule covers cases
 the segment doesn't mandate) or too narrow (rule misses cases the
 segment does cover). Return the alternative reading or 'CONCURS'."
```

The "too broad vs. too narrow" framing is more useful for rules than
the input-pipeline framing.

## Conflicts Between Sources

When two source rulebooks both produce rules that fire on the same
query but disagree on the conclusion, the framework **does not silently
resolve the conflict**. Both rules fire; both proofs are included in
the proof DAG; the engine's answer is reported as `disputed` with each
source cited.

```text
DisputedAnswer := {
    candidates: [
        { answer: ..., proof_path: ..., source_rulebook: ..., source_section: ... },
        { answer: ..., proof_path: ..., source_rulebook: ..., source_section: ... },
    ],
    resolution_required_from: Human | DomainConfig,
}
```

Some deployments may configure a *priority* policy (e.g., "FDA
guidelines take precedence over local hospital policy"); this is per-
deployment configuration recorded in the audit trail. The framework's
default is to surface the conflict.

## Time-Varying Rules

Every Rule node carries `metadata.as_of`. The engine respects this:

- When adjudicating with `as_of_date = D`, only Rules whose `as_of ≤ D`
  are considered.
- When a new version of a rulebook is compiled, both old and new Rule
  nodes coexist with different `as_of` stamps; old adjudications
  remain reproducible.

This is essential for compliance: a case from 2023 must be re-runnable
against the 2023 rules, not the 2026 rules.

The audit trail records the `as_of` cutoff used for the adjudication.

## Missing Rule Inputs

When an engine attempts to fire a Rule but a required input field
(e.g., `wh` for the lithium-battery rule) is missing from any Fact, the
engine generates a `missing_rule_input` event that ADJ06 surfaces as a
clarification:

> "To apply rule R5 (lithium battery limit), the system needs to know
>  the watt-hour rating of [item]. The current information does not
>  include this. Could you provide it?"

This is the mechanism worked through in `ADJ10`. It is *not* a checker-
pass failure — it is a normal part of engine execution that consults
ADJ06 for additional input.

## Expert Review Workflow

The compilation pipeline's output is *reviewed*, not *authored*, by
a domain expert. The review surface:

```text
ReviewableUnit := {
    rule:              IRNode,                  -- the compiled Rule
    source_segment:    Span,                    -- where it came from
    rendering:         string,                  -- NLI-passing rendering
    checker_flags:     [Violation],             -- any pass concerns
    adversary_log:     Option<AdversarialLog>,  -- adversary's findings
    suggested_review_priority: Priority,
}

Priority := High | Medium | Low
```

The pipeline emits ReviewableUnits in priority order:

- **High** — adversary flagged a plausible alternative, or round-trip
  drift required clarification
- **Medium** — polarity/modality required scope analysis, or rule has
  complex nested structure
- **Low** — all checks passed with high confidence, simple rule shape

A reviewing clinician (or attorney, or compliance officer) can focus
on High first, sample Medium, and rely on the structural checks for
Low. This is the workflow that makes the framework practical for real
domains.

Per-rule sign-off is logged in the audit trail. Re-running an
adjudication uses the most recent signed-off rule version.

## Rulebook Versioning

When a rulebook changes (new TSA regulation, updated UpToDate chapter,
revised license terms), the entire pipeline is re-run on the changed
sections. Unchanged sections retain their existing Rule nodes; changed
sections produce new Rule nodes with new `as_of` stamps. Removed
sections retain their Rule nodes with a `superseded_at` stamp and are
excluded from future adjudications (but remain available for replay
of past cases).

A `rulebook_diff` artifact records the changed/added/removed sections
between versions. This is a deployment-level concern, not a per-
adjudication concern.

## Worked Example

The TSA carry-on rulebook compiled to the five Rules shown in `ADJ10`:

| Source segment | Subtype | Rule head |
|---|---|---|
| `1540.111(a)` general allowance | definitional | `carry_on_allowed(Item)` |
| LAG §b1 liquid-aerosol-gel | constraint | (no head; volume bound) |
| Prohibited §1 matches | definitional | `prohibited(matches)` |
| Prohibited §3 knives over 2.36in | definitional | `prohibited(pocket_knife(BL))` |
| Batteries §c lithium spare limit | definitional | `carry_on_lithium_ok(...)` |

Each compiled rule has source_spans citing the specific subsection of
TSA's published rulebook; `metadata.as_of` set to the rulebook's
publication date.

Coverage: every byte of TSA-prohibited, TSA-LAG, and TSA-batteries
must be either in some Rule's source_spans or in a Discarded node with
a reason. Adversarial: a domain expert in airport security reviewed
the High-priority compilations and signed off; the audit trail records
this.

## Open Questions

1. **Reference resolution.** A regulatory text often refers to other
   sections ("as defined in §2"). The pipeline must resolve these
   references before compilation or carry them through as structured
   metadata. Cross-reference resolution is `ADJ09a`.
2. **Multi-document rulebooks.** A clinical guideline may reference a
   formulary, a billing manual, and a state regulation. The pipeline
   currently handles one rulebook at a time; multi-document
   compilation with cross-references is `ADJ09b`.
3. **Tabular rules.** Many regulations include tables (drug-dosage
   tables, weight-class tables). Compiling these to Rules is a
   structural transformation that needs careful handling.
4. **Counterfactual queries against versioned rulebooks.** "Would
   this case have been adjudicated differently under last year's
   rules?" The engine supports this via `as_of`; the UX of presenting
   counterfactual comparisons is deployment-specific.

## Limitations

1. **The pipeline is only as accurate as its components.** Compilation
   errors that survive the four checker passes will manifest as
   incorrect rule firings later. The mitigation is expert review,
   which the priority-ordering workflow makes tractable.
2. **Highly subjective rule text** — "appropriate", "reasonable",
   "as needed" — does not compile cleanly. The framework's response
   is to extract such terms as configuration knobs that domain experts
   set, not as autonomous rules.
3. **Cross-document reasoning** is currently out of scope; some real
   adjudications require it (HIPAA + a state privacy law + a hospital
   policy all firing on the same case).

## Status

Draft. Sufficient to implement the segmentation-and-per-segment-
compilation core. `ADJ09a` (reference resolution), `ADJ09b` (multi-
document rulebooks), and table-handling extensions follow as the rule
pipeline meets real corpora.
