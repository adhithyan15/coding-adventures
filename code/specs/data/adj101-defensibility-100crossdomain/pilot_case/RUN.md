# ADJ101 pilot case — one complex case, end-to-end through the framework

A single-case vertical slice proving the program-emission track runs end-to-end: messy input →
LLM translator (typed, provenanced facts + emitted program) → deterministic provenance executor →
result. And the rescored paradigm: when wrong, the audit localizes the error and a one-fact override
re-derives it.

## The case (combustion stoichiometry — genuinely multi-step)

> SOURCE: "When methane (CH4) undergoes complete combustion in oxygen, carbon dioxide and water are
> produced. A chemist burns 25.0 grams of methane to completion. The lab thermometer on the bench
> reads 21 degrees Celsius."
> QUESTION: "How many grams of carbon dioxide (CO2) are produced?"

Why it's a good test: needs molecular weights (CH4, CO2) that the model must get from a **tool**, not
its memory; needs the 1:1 carbon mole ratio as a **justified inference**; and carries a **distractor**
(the thermometer) that must be discarded, not silently dropped. In-head, a model can grab a wrong MW
or drop a step; the framework forbids both.

## What the LLM translator emitted (it did NOT compute)

- `mass_methane` = **stated** 25.0 g, span `"25.0 grams of methane"`.
- `carbon_ratio_ch4_to_co2` = **inferred** 1.0, `basis_span: "carbon dioxide and water are produced"`,
  `entailment: ENTAILED` (conservation of carbon).
- **discarded**: `"21 degrees Celsius"` — "ambient bench temperature; stoichiometry depends only on
  molar masses and the 1:1 ratio, not temperature."
- **program**: gets `mw_ch4`, `mw_co2` from **RDKit** (no hard-coded molecular weights), pulls mass +
  ratio from `facts[...]`, computes `RESULT = (mass/mw_ch4) * ratio * mw_co2`.

(`emission_combustion.json`.)

## (1) The clean run — a program executed and produced a result

```
result: 68.58   exec_ok: true   auditable: true
fabrications: []   unfaithful_facts: []   missing_coverage: []   magic_numbers: []
correct: true   (independent RDKit gold = 68.58 g CO2)
```

A program **emitted, executed, and produced 68.58 g** — fully byte-provenanced. The molecular weights
came from the tool (not laundered from model memory → no `magic_numbers`); the distractor was
accounted for (→ no `missing_coverage`).

## (2) The rescored paradigm — wrong is fine if localized + correctable

Inject a realistic extraction error: the model misreads `25.0` as `52.0` but still cites the span
`"25.0 grams of methane"`.

```
WRONG RUN:   result: 142.65   correct: false
             error_locus.unfaithful_facts: ['mass_methane']   <- the audit points EXACTLY here
             (value 52.0 contradicts its own cited span "25.0 grams of methane")

OVERRIDE:    override mass_methane {from: 52.0, to: 25.0}      (fix the fact, not the weight)
             re-derived: 68.58   correct: true   auditable: true   model calls used: 0
```

The wrong answer is **localized to the exact fact** (value-vs-span faithfulness), and a **single
override re-derives the correct answer with zero model calls** — the program-track instance of E2
(localize→fix→persist) and MYCIN's edit-override-propagate.

## Reproduce
`python3 ../provenance_program.py` (gates + smoke); the case run is the snippet above over
`emission_combustion.json`.
