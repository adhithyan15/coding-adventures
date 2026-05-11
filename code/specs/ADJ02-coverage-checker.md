# ADJ02 — Coverage Checker: Every Meaningful Span Must Be Accounted For

## Overview

This spec defines the **coverage** invariant: the first and cheapest of the
four checker passes described in [`ADJ00`](ADJ00-adjudication-framework.md).
Coverage is what catches the extractor *silently dropping* clinically or
domain-meaningful spans of the input.

The invariant is one sentence:

> Every meaningful token in the input must belong to the source spans of
> at least one IR node — either as part of a Fact, Query, Uncertainty, or
> as an explicit Discarded node citing a reason.

"Meaningful" is domain-specific; a token classifier (or LLM call) decides
which tokens carry meaning. Coverage is then a structural property of the
IR document against the input.

## Why a Coverage Invariant Matters

The failure mode coverage catches is the one most invisible to chain-of-
thought, retrieval, or self-refinement: extraction that silently *omits*
information. A patient's note that mentions a recent foreign travel
relevant to differential diagnosis, an LAG-rule input that mentions a
specific ml volume, a license disclosure that mentions a date — these can
be dropped by an extractor focused on the "main" content, and downstream
reasoners have no way to know.

Coverage as a *hard* invariant — not a soft metric — means the system
refuses to ship an IR that misses meaningful input. Failure routes to
clarification, not to a wrong answer with no signal.

## Layer Position

```
   ADJ01 IR grammar
        │
        ▼
   ADJ02 Coverage Checker          ← this document
        │
        ▼
   ADJ03 polarity/modality checker
        │
        ▼
   ADJ04 round-trip verifier
        │
        ▼
   ADJ05 adversarial verifier
```

The four checker passes run in order. Coverage runs first because it is
the cheapest and because subsequent passes assume that every meaningful
span is already linked to an IR node.

## Inputs and Outputs

The coverage check takes:

- **`Document`** — the input document (per `ADJ01`), with byte-offset
  ranges over normalized text.
- **`Document.normalized_text`** — the text the extractor saw.
- **`IRDocument`** — the IR produced by extraction (per `ADJ01`).
- **`Tagger`** — the token classifier (described below) appropriate to
  the adjudication domain.

It returns either:

- **`Pass`** — every meaningful token is covered.
- **`Fail(uncovered_spans)`** — at least one meaningful span is not
  covered by any IR node. The list of uncovered spans is the seed for
  clarification questions.

The check is **deterministic** and does **not** call an LLM at check time.
The tagger may itself be an LLM call, but that call happens *before* the
check and its output is treated as data.

## The Token Classifier (Tagger)

The tagger is a function:

```text
classify_tokens : Document -> [TokenAnnotation]

TokenAnnotation := {
    span:    Span,
    label:   Meaningful | NonMeaningful,
    reason:  Option<String>,  // for telemetry / debugging
}
```

Implementations:

1. **Rule-based tagger.** A configurable set of regex patterns and stopword
   lists. Reproducible, fast, debuggable. Sufficient for narrow domains
   like TSA carry-on rules.
2. **Small classifier model.** A purpose-trained classifier (BiLSTM or
   tiny transformer) running locally. Sufficient for medium-complexity
   domains.
3. **LLM call with constrained output.** An LLM prompted to label each
   token with a single label, with output constrained to the label
   vocabulary. Required for complex domains (clinical notes); accepted
   despite cost because the output is data, not reasoning.

Whichever implementation is used, the tagger's *output* is what the
coverage check operates on. The check itself is mechanical.

**Versioning is mandatory.** Every IR document records the tagger version
that produced its meaningful-token annotation. Re-running coverage with a
different tagger version produces a different result and is a
re-adjudication, not a re-check.

## What Counts as Meaningful

The tagger's vocabulary is domain-specific, but the framework specifies a
common shape for `NonMeaningful` reasons so coverage failures are
auditable:

```text
NonMeaningfulReason :=
    Whitespace
    Punctuation
    Stopword               -- "the", "a", "of", language-dependent list
    SocialPleasantry       -- "hello, doctor", "thanks for seeing me"
    DocumentChrome         -- headers, footers, page numbers
    Boilerplate            -- "signed electronically by"
    Determiner             -- "this", "that" — usually safely ignored
    Filler                 -- "umm", "you know"
```

Anything not falling into one of these categories is `Meaningful` by
default. The defaulting direction matters: false-negative `Meaningful`
labels are corrected by an over-eager coverage pass (which routes to
clarification, slowing the system but never producing wrong output);
false-positive `NonMeaningful` labels are silent extraction errors and
must be designed against.

Domains may extend the `NonMeaningful` vocabulary by registering new
reasons with a domain tagger. Extensions are recorded in a domain
configuration file alongside the tagger version.

## The Coverage Algorithm

Given the tagger output and the IR document, the coverage check is a
classical interval-cover problem.

```text
coverage_check(doc, ir_doc, tagger) -> Result:
    annotations = tagger.classify_tokens(doc)
    meaningful_spans = [a.span for a in annotations if a.label == Meaningful]

    covered_spans = []
    for node in ir_doc.nodes:
        covered_spans.extend(node.source_spans)

    # interval-cover: is every meaningful span fully contained in the
    # union of covered_spans?
    uncovered = []
    for m in meaningful_spans:
        if not is_fully_covered(m, covered_spans):
            uncovered.append(m)

    return Pass if uncovered.is_empty() else Fail(uncovered)
```

`is_fully_covered` is a straightforward interval check: the meaningful
span's `[start, end)` is a subset of the union of the covered spans
restricted to the same document. Implementations should pre-sort and merge
covered spans to make the check linear in their count.

## Multi-Span and Overlap Handling

Several edge cases require explicit policy:

1. **Multiple IR nodes covering the same span.** Permitted. A negation
   span like *"denies chest pain"* may belong to both a `Fact(chest_pain,
   Denied)` node and to an internal `Discarded` node citing the "denies"
   trigger. Both source-span entries count toward coverage.
2. **An IR node citing more text than just its meaningful core.** The
   extractor is expected to cite the *minimum span that captures the
   meaning*. Citing extra surrounding text is permitted but logged; it is
   not a coverage failure but it makes downstream polarity/modality checks
   slower and more error-prone.
3. **A meaningful span spanning a node boundary.** Permitted as long as
   *every byte* of the span is covered by some node. The coverage check
   does not require a single node to fully contain a meaningful span —
   only that the union of all node spans does.
4. **Empty IR.** An IR document with zero nodes against an input with at
   least one meaningful token fails coverage. The framework's idea of
   "this input is irrelevant" is a single `Discarded(NonDomainContent)`
   node covering the input, *not* an empty IR.

## Failure Modes and Clarification

A coverage failure produces a list of uncovered spans. The clarification
generator (specified in `ADJ06`) turns each uncovered span into a question
shaped like:

> "You did not account for *'\<text of uncovered span\>'*. What does it
> mean in this context?"

If multiple adjacent uncovered spans are short, they are merged into a
single clarification before being surfaced, so the user is not asked to
clarify three consecutive words separately.

Coverage failures are *not* fatal on their own. The escalation ladder
applies (per `ADJ00`): the cheapest response is to re-prompt the extractor
with the specific uncovered spans. Only if re-prompting fails does the
question reach a human.

## Special Case: The Unparseable Discard Reason

`ADJ01` specifies that the `Unparseable` discard reason is always a
coverage failure. The coverage check enforces this:

```text
For every Discarded node N:
    if N.discard_reason == Unparseable:
        Fail([N.source_spans])
```

This is the rule that prevents extractors from quietly "shipping" spans
they don't understand. An Unparseable span is a clarification trigger, not
a valid IR shape.

## Configuration Surface

A coverage configuration declares:

- **Tagger:** which classifier to use (and its version).
- **Extended NonMeaningfulReasons:** domain-specific additions.
- **Strictness mode:**
  - `strict`: any uncovered byte fails coverage.
  - `permissive`: uncovered tokens flagged as `Filler` or `Determiner`
    are allowed, all others fail.
  - `audit-only`: never blocks; logs failures for offline review.
- **Span-overlap policy:** see Multi-Span section above. Default is
  permissive overlap, audit-only logging on excess citation.

Configurations are versioned alongside the tagger and are part of the
audit-trail metadata for every adjudication.

## Worked Example

Continuing the TSA running example:

Input span 142..168: `"I am not bringing matches"`

Tagger annotations:

| Tokens | Label | Reason |
|---|---|---|
| `I am` | NonMeaningful | Determiner/auxiliary |
| `not bringing` | Meaningful | (default) |
| `matches` | Meaningful | (default) |

The extractor produces:

```text
F6 {
    kind:         Fact,
    term:         carry_on_item(matches),
    polarity:     Denied,
    source_spans: [(doc1, 142, 168)],
}
```

`F6.source_spans` covers the whole `"I am not bringing matches"` span,
which is a superset of both meaningful sub-spans. Coverage check: **pass**.

Counterexample: if the extractor instead produced

```text
F6' {
    kind:         Fact,
    term:         carry_on_item(matches),
    polarity:     Affirmed,
    source_spans: [(doc1, 161, 168)],   // just "matches"
}
```

then the meaningful span `"not bringing"` (149..160) is uncovered.
Coverage **fails**, clarification fires: *"You did not account for 'not
bringing'. What does it mean in this context?"*. The re-prompted extractor
should then produce the correct `Denied` polarity by including the
negation phrase in `source_spans`. This is the exact failure mode the
polarity check (`ADJ03`) is designed to catch, but coverage catches it
first and cheaper.

## Open Questions

1. **Punctuation that carries meaning.** "Don't" vs. "Do not" — the
   apostrophe is structurally meaningful for negation parsing. Current
   spec lumps punctuation under NonMeaningful; clinical and legal
   subdomains may need to special-case some punctuation.
2. **Quoted speech.** "Patient said 'I don't want surgery'" — the quoted
   span contains its own polarity that must be preserved. Probably handled
   by Modality (a separate node with `Hypothetical` modality and a
   `quoted_speech` metadata flag), but worth specifying.
3. **Tabular and form data.** Documents with structure (vitals tables,
   medication lists) carry meaning through *layout*, not just tokens.
   Currently treated as if the table were linearized; structured-doc
   extraction is `ADJ02a` future work.

## Limitations

1. Coverage cannot catch *incorrect* extraction whose source spans happen
   to cover the right input. That's what passes 2–4 are for.
2. The tagger is a dependency. A bad tagger produces bad coverage. The
   versioning discipline is the mitigation, not a fix.
3. For very long documents (multi-thousand-word clinical histories),
   coverage may have many uncovered spans simultaneously. The
   clarification UX may need batching or summarization, specified in
   `ADJ06`.

## Status

Draft. Sufficient to implement against. Sub-spec `ADJ02a` will follow once
the first concrete tagger is built, formalizing the structured-document
extension.
