# ADJ-ARGUMENT-REBUTTAL — worked rebuttal + undercut example (AR-2)

The worked example for [`ADJ-ARGUMENT-REBUTTAL.md`](../../../ADJ-ARGUMENT-REBUTTAL.md): a paper
whose thesis is **attacked**, proving end-to-end that ADJ represents *and resolves* a paper's
dialectic — a defeated conclusion is **withdrawn by the engine** (not filtered in Python), and
every attacking premise is byte-anchored like any other. Both attack kinds reuse existing
machinery with zero new engine code (§2 of the spec).

## Paragraph sources (each its own pinned snapshot)

- **`pA-support.source.txt`** — the support: beach marks ⇒ *fatigue*.
- **`pB-reanalysis.source.txt`** — the **rebuttal**: a single-shear lip ⇒ *overload* (a rival
  conclusion).
- **`pC-limitation.source.txt`** — the **undercut**: a contaminated sample (a methods limitation).

## `rebuttal.adj` — REBUT (attack a conclusion)

The support paragraph concludes `failed_by(axle, fatigue)` under `context: initial_report`; the
reanalysis paragraph concludes `failed_by(axle, overload)` under `context: reanalysis`; the paper's
`context_order { reanalysis > initial_report }` says the reanalysis outranks. With
`functional failed_by(subject, mechanism)`, the two conclusions **conflict**, and the `governing`
query resolves it:

- `failed_by(axle, fatigue)` → **`status: defeated`**, `defeated_by: failed_by(axle, overload)`;
- `failed_by(axle, overload)` → **`status: governing`**.

`adj-verify --snapshots` byte-anchors both `relate` premises against their paragraphs
(`quotes_verified: 2`, `verified: true`). The rebuttal is grounded exactly like the support.

## `undercut.adj` — UNDERCUT (attack a warrant)

The fatigue inference fires only `when: shows(surface, beach_marks), not warrant_undercut`; a
limitation paragraph grounds `warrant_undercut` from `contaminated(sample)`. When the contamination
holds, the thesis **abstains** — `? failed_by(axle, $M)` returns no answer — because the *licence*
for the inference is removed; **no rival mechanism is asserted**. Remove the contamination and the
fatigue thesis derives again. (An undercut blocks the proof, so there is no derivation to
byte-anchor — the point is the honest abstention, not a citation count.)

Both are driven by `adj-lang-cli/tests/rebuttal_worked_example_e2e.rs`. Together they show a paper's
disagreements are first-class: **the engine withdraws what its counterarguments defeat, and every
attack is auditable back to the paragraph that makes it.**

## `rebuttal-inblock.adj` / `undercut-inblock.adj` — AR-3, attack IN the `argument` block

The same two papers, but the attack now lives **inside** the `argument` block using the AR-3
surface sugar (adj-lang 0.68.0), instead of raw `rule`s bolted alongside:

- **`rebuttal-inblock.adj`** — each `infer` carries a trailing `context:` (support in
  `initial_report`, reanalysis in `reanalysis`); with the top-level `functional` +
  `context_order`, the engine withdraws fatigue exactly as `rebuttal.adj` does.
- **`undercut-inblock.adj`** — the support `infer` carries `unless warrant_undercut` (→ a `not`
  body literal) and a second `infer` derives that defeater from the contamination premise; the
  thesis abstains while the contamination holds.

Driven by `adj-lang-cli/tests/argument_inblock_attack_e2e.rs`. Same desugaring, same audit — the
whole dialectic (support **and** attack) now decomposes into one construct.
