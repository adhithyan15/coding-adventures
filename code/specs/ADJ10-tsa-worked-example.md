# ADJ10 — TSA Carry-On Worked Example: An End-to-End Adjudication Test Fixture

## Overview

This spec is the **runnable test fixture** for the Adjudication
framework. It traces a single TSA carry-on baggage adjudication
end-to-end: from a passenger's prose declaration through extraction,
all four checker passes, clarification dialogue, rule firing, and
audit-trail construction.

Every artifact below — input text, IR nodes, checker results, dialogue
turns, proof DAG, audit trail — is concrete. An implementation of the
framework should be able to load this document, replay the
adjudication, and reproduce the same final answer with byte-equal
audit trail.

This is the **single example** referenced throughout the rest of the
ADJ specs (ADJ00 §"Worked Example", ADJ02 §"Worked Example", ADJ03,
ADJ04, ADJ06, ADJ07). Here it is collected in one place, with every
detail.

## The Input

The passenger types or speaks the following at a TSA checkpoint:

> *"I'd like to bring a 4 oz tube of toothpaste, a 100 ml perfume,
> three lithium camera batteries, a bottle of wine for my mother, and
> a 4-inch pocket knife. I am not bringing matches, only a single
> disposable lighter."*

The document's normalized text is exactly this string. Byte offset
counting starts at 0; the document is identified by
`DocumentId("tsa-2026-05-11-001")`.

```text
Offset  Text
------  ----
   0  : I'd like to bring a 4 oz tube of toothpaste,
  45  :  a 100 ml perfume,
  62  :  three lithium camera batteries,
  93  :  a bottle of wine for my mother,
 124  :  and a 4-inch pocket knife.
 150  :  I am not bringing matches,
 176  :  only a single disposable lighter.
 209  : <end>
```

## The Rulebook (Compiled IR Excerpt)

Relevant TSA carry-on rules are compiled into IR rules by the rule-
compilation pipeline (`ADJ09`). For this example we show only the rules
that participate; the full rulebook IR contains many more.

```text
R1 Rule {
    term: definitional(
        carry_on_allowed(Item),
        [Pos(non_prohibited(Item)), Pos(within_size_limit(Item))]
    ),
    polarity: Affirmed, modality: Present,
    source_spans: [rulebook-tsa-cfr-1540.111 §a],
    metadata: { as_of: "2025-09-01" }
}

R2 Rule {
    term: constraint([
        Pos(carry_on_item(X, container=tube, volume=V)),
        Pos(le(V, quantity(100, ml)))  // LAG rule
    ]),
    polarity: Affirmed, modality: Present,
    source_spans: [rulebook-tsa-lag §b1],
    metadata: { as_of: "2025-09-01" }
}

R3 Rule {
    term: definitional(
        prohibited(matches),
        [Pos(carry_on_item(matches))]
    ),
    polarity: Affirmed, modality: Present,
    source_spans: [rulebook-tsa-prohibited §1],
    metadata: { as_of: "2025-09-01" }
}

R4 Rule {
    term: definitional(
        prohibited(pocket_knife(BladeLength)),
        [Pos(carry_on_item(pocket_knife, blade_length=BladeLength)),
         Pos(gt(BladeLength, quantity(2.36, in)))]
    ),
    polarity: Affirmed, modality: Present,
    source_spans: [rulebook-tsa-prohibited §3],
    metadata: { as_of: "2025-09-01" }
}

R5 Rule {
    term: definitional(
        carry_on_lithium_ok(Item),
        [Pos(carry_on_item(lithium_battery, wh=Wh, count=N, type=Item)),
         Pos(le(Wh, quantity(100, wh))),
         Pos(le(N, 2_spare_or_installed_in_device))]
    ),
    polarity: Affirmed, modality: Present,
    source_spans: [rulebook-tsa-batteries §c],
    metadata: { as_of: "2025-09-01" }
}
```

(Concrete CFR section references are illustrative; an actual deployment
would pull the latest TSA rulebook through ADJ09.)

## Extraction (Initial IR)

The extractor produces seven Fact nodes from the passenger's
declaration:

```text
F1 Fact {
    id: "F1", kind: Fact,
    term: carry_on_item(toothpaste, container=tube, volume=quantity(4, oz)),
    polarity: Affirmed, modality: Present,
    source_spans: [(tsa-2026-05-11-001, 5, 45)],
    confidence: 0.96, lowered_from: None,
}

F2 Fact {
    id: "F2", kind: Fact,
    term: carry_on_item(perfume, volume=quantity(100, ml)),
    polarity: Affirmed, modality: Present,
    source_spans: [(tsa-2026-05-11-001, 46, 62)],
    confidence: 0.94, lowered_from: None,
}

F3 Fact {
    id: "F3", kind: Fact,
    term: carry_on_item(lithium_battery, count=3),
    polarity: Affirmed, modality: Present,
    source_spans: [(tsa-2026-05-11-001, 63, 93)],
    confidence: 0.92, lowered_from: None,
}

F4 Fact {
    id: "F4", kind: Fact,
    term: carry_on_item(wine, container=bottle),
    polarity: Affirmed, modality: Present,
    source_spans: [(tsa-2026-05-11-001, 94, 124)],
    confidence: 0.95, lowered_from: None,
}

F5 Fact {
    id: "F5", kind: Fact,
    term: carry_on_item(pocket_knife, blade_length=quantity(4, in)),
    polarity: Affirmed, modality: Present,
    source_spans: [(tsa-2026-05-11-001, 125, 150)],
    confidence: 0.93, lowered_from: None,
}

F6 Fact {
    id: "F6", kind: Fact,
    term: carry_on_item(matches),
    polarity: Denied, modality: Present,
    source_spans: [(tsa-2026-05-11-001, 151, 176)],
    confidence: 0.97, lowered_from: None,
}

F7 Fact {
    id: "F7", kind: Fact,
    term: carry_on_item(lighter, type=disposable, count=1),
    polarity: Affirmed, modality: Present,
    source_spans: [(tsa-2026-05-11-001, 177, 209)],
    confidence: 0.96, lowered_from: None,
}
```

Plus the Query:

```text
Q1 Query {
    id: "Q1", kind: Query,
    term: carry_on_decision(items),
    polarity: Affirmed, modality: Present,
    source_spans: [(tsa-2026-05-11-001, 0, 209)],
    confidence: 1.0, lowered_from: None,
}
```

## Checker Pass Results (Round 1)

### ADJ02 Coverage

Every byte from offset 0 to 209 is covered by some IR node's
`source_spans`. The tagger classifies tokens like `"I'd like to bring"`
as `NonMeaningful` (pleasantry); the meaningful-token coverage check
sees `toothpaste`, `4 oz`, `tube`, `perfume`, `100 ml`, `lithium`,
`camera batteries`, `three`, `wine`, `bottle`, `mother`, `pocket
knife`, `4-inch`, `not`, `bringing`, `matches`, `lighter`,
`disposable`, `single` — all are in some node's spans.

**Result:** Pass. No violations.

### ADJ03 Polarity and Modality

- `F1..F5, F7`: Affirmed polarity. No negation triggers in their spans.
  **Pass.**
- `F6`: Denied polarity. Span `(151, 176)` is *"I am not bringing
  matches"*. The `Negation` trigger `"not"` has forward scope to
  sentence end; covers `matches`. Required polarity = `Denied`; actual
  = `Denied`. **Pass.**

**Result:** Pass for all seven Facts and Q1. No violations.

### ADJ04 Round-Trip Entailment

For each leaf node, render → NLI both directions.

- `F1` renders to *"The passenger is bringing a 4 oz tube of
  toothpaste."* — both directions entail with source span. **Pass.**
- `F2` renders to *"The passenger is bringing a 100 ml perfume."* —
  both directions entail. **Pass.**
- `F3` renders to *"The passenger is bringing three lithium camera
  batteries."* — round-trip fails: the source mentions camera-grade
  lithium batteries; the rule database (R5) requires watt-hour rating
  and whether the batteries are spare or installed-in-device. The
  rendering omits these structured fields **because they are not in
  the source**. The NLI score in the IR-to-source direction is high
  (the IR doesn't add information), but the engine recognizes that R5
  requires Wh and count-detail that F3 cannot provide.

  This is *not* an ADJ04 failure per se — ADJ04 confirms the IR
  faithfully represents the source. The downstream rule R5 will fail to
  apply for lack of structured fields. The engine surfaces this as a
  clarification need, which routes through ADJ06 with reason
  `MissingRuleInputs` (a new reason class added by ADJ09's rule-input
  protocol — see ADJ09 §"Missing-Input Clarification").

  For this example, F3 passes ADJ04 itself but produces a missing-rule-
  input event that ADJ06 handles separately.

- `F5` renders to *"The passenger is bringing a 4-inch pocket knife."*
  — both directions entail. **Pass.** (R4 will fire on this directly.)

- `F6`, `F7` likewise pass.

**Result:** Pass for all leaf nodes. One missing-rule-input event
queued for F3.

### ADJ05 Adversarial

The adversary examines each leaf node looking for a contradicting
reading. For `F6` (the denied matches Fact), the adversary considers:
*"Could 'I am not bringing matches' be reporting a forgetfulness ('I
forgot the matches') rather than an intent? Unlikely given the rest of
the sentence specifies the lighter as an alternative."* The
plausibility judge returns `IMPLAUSIBLE`. **F6 passes.**

For other nodes, the adversary returns `CONCURS`.

**Result:** Pass for all nodes. Adversarial-log entries recorded with
the adversary's reasoning and the judge's plausibility verdicts.

## Clarification Dialogue (Round 1 → Round 2)

The missing-rule-input event for F3 triggers a clarification:

```text
DialogueTurn 1 {
    turn_id: 1, rung: Rung0,
    failure: missing_rule_input(F3, R5, required_fields=[wh, spare_or_installed]),
    question: {
        kind: AmbiguousReference,
        text: "For the three lithium camera batteries: how many watt-hours
                (Wh) is each one rated for, and are they installed in
                cameras or carried as spares?",
        spans: [(tsa-2026-05-11-001, 63, 93)],
    },
    response: { source: Extractor, text: "<re-prompt with R5 input requirements>",
                outcome: Failed },
    outcome: Failed,
}
```

The Rung0 re-prompt produces an Uncertainty node instead of a refined
Fact, because the source genuinely lacks this information. Escalate
to Rung2.

```text
DialogueTurn 2 {
    turn_id: 2, rung: Rung2,
    failure: missing_rule_input(F3, R5, required_fields=[wh, spare_or_installed]),
    question: { kind: AmbiguousReference, text: "<same question>",
                spans: [(tsa-2026-05-11-001, 63, 93)] },
    response: {
        source: User,
        text: "80 Wh each. One is in the camera, two are spares.",
        actor_id: "passenger",
        timestamp: "2026-05-11T08:01:25Z",
    },
    new_spans: [(tsa-2026-05-11-001, 210, 257)],  // appended text
    outcome: Resolved,
}
```

After Round 2, F3 is replaced by a lowered node F3a:

```text
F3a Fact {
    id: "F3a", kind: Fact,
    term: carry_on_item(lithium_battery,
                        count=3,
                        wh=quantity(80, wh),
                        installed=1,
                        spare=2),
    polarity: Affirmed, modality: Present,
    source_spans: [(tsa-2026-05-11-001, 63, 93),
                    (tsa-2026-05-11-001, 210, 257)],
    confidence: 0.98,
    lowered_from: Some(NodeId("F3")),
}
```

ADJ02–05 re-run on F3a and all pass. The adjudication continues.

## Engine Resolution

With the IR document complete and all checks passing, the engine runs.
The KB consists of the seven Facts (F1, F2, F3a, F4, F5, F6, F7) plus
the five Rules (R1..R5). All clauses are `Certain`, so the engine's
`AutoDetect` selects `FindFirst`.

The engine processes the Query `carry_on_decision(items)` by examining
each Fact against the relevant rules:

| Item (Fact) | Rules consulted | Result | Reason |
|---|---|---|---|
| F1 toothpaste 4 oz | R1, R2 (LAG) | **denied** | 4 oz > 100 ml (3.4 oz = 100 ml). Container is tube; LAG applies. |
| F2 perfume 100 ml | R1, R2 (LAG) | **borderline** | 100 ml is exactly at the limit. LAG rule says < 100 ml. Engine flags as borderline; ADJ06 clarification asks if exactly 100 or under. *(For this fixture, assume the response is "under 100 ml" and F2 is permitted.)* |
| F3a lithium 80 Wh, 1 installed + 2 spare | R1, R5 | **permitted** | 80 Wh ≤ 100 Wh; ≤ 2 spares; rule R5 satisfied. |
| F4 wine bottle | R1 | **permitted** | Not in prohibited list; bottle volume not specified but no LAG-relevant issue (alcohol carry-on has separate rules not modeled here). |
| F5 pocket knife 4 in | R1, R4 | **denied** | Blade length 4 in > 2.36 in (TSA limit). R4 fires. |
| F6 matches (Denied) | R3 | **N/A** | The passenger declared they are *not* bringing matches; the polarity Denied means the Fact does not assert presence. R3 does not fire. The passenger's declaration is recorded. |
| F7 disposable lighter, count 1 | R1 | **permitted** | Disposable lighters are explicitly permitted, count 1 satisfies. |

## The Audit Trail

The complete adjudication trail (skeleton form) for this fixture:

```json
{
  "schema_version": "ADJ07-v1",
  "adjudication_id": "01HGTSA-2026-05-11-001",
  "started_at": "2026-05-11T08:01:23Z",
  "completed_at": "2026-05-11T08:01:34Z",
  "outcome": { "kind": "Resolved", "answer": {
      "verdict": {
          "F1_toothpaste": "denied (LAG: 4 oz > 100 ml)",
          "F2_perfume":    "permitted (after under-100 clarification)",
          "F3a_batteries": "permitted (80 Wh, count compliant)",
          "F4_wine":       "permitted",
          "F5_knife":      "denied (blade 4 in > 2.36 in)",
          "F6_matches":    "n/a (declared not bringing)",
          "F7_lighter":    "permitted"
      }
  } },
  "documents": [
      {
          "id": "tsa-2026-05-11-001",
          "normalized_text": "I'd like to bring a 4 oz tube of toothpaste...",
          "appended_turns": [
              { "turn_id": 2, "start_offset": 210, "end_offset": 257 }
          ]
      }
  ],
  "ir_nodes": [ /* F1..F5, F3 (root, kind: Fact), F3a (leaf), F6, F7, Q1 */ ],
  "checker_results": [
      { "pass_name": "ADJ02_coverage", "outcome": "Passed", ... },
      { "pass_name": "ADJ03_polarity_modality", "outcome": "Passed",
        "notes": "F6 negation correctly extracted via 'not' trigger" },
      { "pass_name": "ADJ04_round_trip", "outcome": "Passed (with missing-input flag on F3)" },
      { "pass_name": "ADJ05_adversarial", "outcome": "Passed",
        "telemetry": { "concur_rate": 6/7, "implausible_rate": 1/7 } }
  ],
  "dialogue": [
      { "turn_id": 1, "rung": "Rung0", "outcome": "Failed", ... },
      { "turn_id": 2, "rung": "Rung2", "outcome": "Resolved",
        "response": { "source": "User", "text": "80 Wh each. One is in the camera, two are spares.", ... } }
  ],
  "engine_artifacts": {
      "engine_version": "logic-engine-0.2.0",
      "search_mode": "FindFirst",
      "kb_summary": { "fact_count": 7, "rule_count": 5, "all_certain": true },
      "proof_dag": { /* per-item proof chains citing R1..R5 */ },
      "answer": { /* same as outcome.answer */ }
  },
  "configuration": {
      "extractor_model": { "name": "anthropic/claude-opus-4-7", "version": "2026-05-10" },
      "renderer_model":  { "name": "anthropic/claude-haiku-4-5", "version": "2026-05-10" },
      "nli_model":       { "name": "deberta-v3-base-mnli", "version": "2024-09-15" },
      "adversary_model": { "name": "openai/gpt-4-2024-09-15", "version": "2024-09-15" },
      "judge_model":     { "name": "anthropic/claude-haiku-4-5", "version": "2026-05-10" },
      "tagger":          { "name": "tsa-tagger-v1", "version": "1.0.0" },
      "trigger_taxonomy": { "name": "negex-context-tsa", "version": "1.0.0" },
      "coverage_strictness": "strict",
      "round_trip_strictness": "standard",
      "adversary_sample_rate": 1.0,
      "escalation_policy": "strict-cheap-first"
  }
}
```

## What Implementations Should Verify

A complete implementation of the framework, given the input above,
should reproduce:

1. The seven initial Fact nodes (F1..F7) with byte-equal source_spans.
2. Q1 with the document-spanning source span.
3. ADJ02–05 all passing on the initial IR plus the missing-rule-input
   event for F3.
4. The clarification dialogue with the user response appended to the
   document and F3 lowered to F3a.
5. The engine's per-item verdicts as in the table.
6. An audit trail with all the components above, modulo non-deterministic
   timestamps and ids.

JSON test fixtures derived from this spec live at
`code/specs/fixtures/adj10-tsa/` (forthcoming alongside the
implementation work). Each implementation can replay the fixture and
compare against expected outputs.

## Notes for Reviewers

- This example is **deterministic** in the engine sense (all rules
  Certain) but **not deterministic** in the LLM sense (every model call
  could produce different output on different runs). The audit trail
  captures every LLM call's prompt, response, and version, so
  replay is reproducible *given the same model versions*.
- The fixture exercises every checker pass, every rung of the
  escalation ladder (Rung0 fails, Rung2 succeeds, Rung1/Rung3 skipped),
  and the proof DAG with multiple rule applications.
- A future fixture (`ADJ10-medical`) will exercise the **probabilistic
  path** (LP19's `EnumerateAll` and WMC). That fixture requires the
  clinical-IR work and is not in scope for the first paper.

## Status

Draft. The fixture's structural shape is sufficient to derive concrete
JSON test files. The companion implementation work will produce those
files and validate them against this spec; deviations between fixture
expectations and implementation behavior trigger spec revisions.
