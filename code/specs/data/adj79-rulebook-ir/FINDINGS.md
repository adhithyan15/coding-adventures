# ADJ79 — rulebook-side compact IR (closed-book), and why derivation belongs on the capable model

Same 3-kind IR as ADJ78, applied to a DERIVED rulebook: rules are conditional FACTs
("condition -> outcome"); basis is a recursive per-rule sub-node tagged
`claimed_from_model_memory` (ADJ70 lower-trust, since a 0.5B can't web-search); plus
applicability UNCERTAINTYs and case QUESTIONs.

## Result — IR structure holds at every scale; rulebook CONTENT is knowledge-bound

| model | conditional-structured rules | content quality |
|---|---|---|
| qwen2.5:0.5b | 4/9 | malformed (split "Condition:"/"Outcome:" into separate lines), vague, no values, errors (e.g. "driver's license?" for voting eligibility) |
| qwen2.5:14b | **6/6** | coherent, correct ("Must be a US citizen -> eligible"; "Cannot be convicted of a felony in most states -> ..."), with accurate uncertainties (felon-rights vary by state; ID laws differ) |

The framework's IR (typing rules / uncertainties / questions, recursion for basis,
authenticity tagging) works at **both** scales. But rulebook **derivation** is a
*knowledge* task: the 0.5B's closed-book knowledge is thin and error-prone; the 14B's is
sufficient and accurate.

## Why this matters: it validates the deployment division of labor
The tiny local model should **apply** a rulebook, not **derive** one. Derivation belongs on
the **capable model, offline, on public sources** (and ideally spider-grounded — all rule
provenance here is `claimed_from_model_memory`, which needs promotion to fetched-and-verified
on an online/capable model). The small local model ingests the input -> IR and applies the
pre-derived, governed rulebook. ADJ79 is direct evidence for that split.

## Limitations
- 2 domains, 2 models; "content quality" judged qualitatively. All rule provenance is
  model-memory (no grounding). The recursion is one level (per-rule basis).
