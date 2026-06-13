# ADJ101 — 10-item adjudication pilot (rule-engine arm): FINDINGS

**Status: COMPLETE.** 10 adjudication items (4 underdetermined-baited, 3 override-precedence,
3 exception-encoding) run through the framework's extraction stage (Opus, 10 agents) → the
deterministic ADJ86/ADJ84 provenance engine. The model did ONLY extraction; the **engine owns the
verdict**. Scored on auditability/localizability; verdict-match is informational.

## Headline (n = 10)

| metric | value |
|---|---|
| IR **extracted** | 10/10 |
| **byte-accounting clean** (every rule `source_span` verbatim in policy; 0 hallucinated rules) | **10/10** |
| underdetermined → **structural INDETERMINATE** (abstained, did NOT fabricate) | **4/4** |
| override/exception → correct DETERMINATE | 6/6 |
| verdict-family match *(informational)* | 10/10 |

## The defensibility win: abstention with a named locus

On all 4 baited items the dispositive fact was withheld from the scenario. The model **correctly
omitted the slot** (it did not guess), so the engine returned **INDETERMINATE structurally** and named
the exact missing fact:

| item | withheld fact the engine named |
|---|---|
| MED2 | `deductible_met` |
| EMP2 | `months_employed` |
| INS2 | `sudden_and_accidental` / `gradual_seepage` / `maintenance_neglect` (the cause) |
| CON2 | `maintenance_announced_hours_in_advance` |

This is the property ADJ86 identified and the bare arm fails: a confident prose answer fabricates the
missing fact ("DENIED", "70%"); the framework **abstains and points at the hole**. The abstention is
itself the audit working — `missing_slots` is the locus a reviewer (or the questioner) resolves.

## Override + exception resolved correctly

The engine fired the right rule on the override (LAW1 stolen-before-violation → NOT_LIABLE; BEN1
oxygen override → QUALIFIES; IMM1 married-3yr → meets) and exception (LAW2 lease-for-cause →
no violation; BEN2 loan-default → ineligible; IMM2 13-month-absence → does not satisfy) items, each
with the rule traced to verbatim policy bytes.

## What this validates

Both halves of the framework arm now run end-to-end at pilot scale:
- **program-emission track** (`pilot10/`): 10/10 programs emitted, 10/10 auditable, 9/10 correct, the
  1 wrong localized + corrected;
- **adjudication track** (this run): 10/10 extracted, 10/10 byte-clean, 4/4 correct abstention with a
  named locus.

The model translates; the **engine/program reasons**; provenance is enforced on both. The
auto-derived rulebook (policy → rules, byte-anchored) is the same mechanism paper-2 MYCIN needs.

## Still ahead for the full ADJ101 pilot
- the **BARE arm** (prose, both models) for the head-to-head;
- the **corrected dual-judge** defensibility scoring (format-normalized, Opus+Sonnet);
- then the full 100 (10 domains × 10).

## Reproduce
`python3 run_adjud10.py` (engine over the saved `emissions_adjud10.json`). Extractor:
`translate_adjud.workflow.js`. Engine reused from `../../adj86-defensibility-benchmark/`.
