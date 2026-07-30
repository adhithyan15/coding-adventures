# ADJ-ARGUMENT-IR — worked example (ADR-5)

The capstone of the decompose-argument-graph arc (`code/specs/ADJ-ARGUMENT-IR.md`): a
paragraph of prose decomposed into an `argument` the engine **derives** and `adj-verify`
**byte-anchors** back to the paragraph's own bytes. It closes the ladder — spec → surface
→ grounding gate → verify → *worked example*.

## Files

- **`axle-fatigue.source.txt`** — the pinned source document: an illustrative technical
  paragraph stating a real materials-science argument (a fatigue-fracture diagnosis). It is
  written in our own words and is **not** attributed to any specific paper; it stands in for
  "a paragraph of a research paper" and is the byte-for-byte source every citation resolves
  against. Its SHA-256 is the `snapshot` hex the `.adj` pins name.
- **`axle-fatigue.adj`** — the paragraph decomposed into an `argument { premise… infer… }`:
  four **premises** (each a provenanced fact citing a *verbatim slice* of the source via
  `quote "…" at <offset> snapshot "<hex>"`), two **inference** steps (each a warranted rule
  citing the connective bytes), and a `? failed_by(axle, $Mechanism)` query.

## What it proves

- `adj-lang-cli axle-fatigue.adj` **derives** `failed_by(axle, fatigue)` — the paragraph's
  thesis — by chaining the inference rules over the premise facts. There is no
  argument-specific evaluator: the `argument` desugared to `relate` facts + `rule`s, and the
  engine reasons over those (ADR-2).
- `adj-verify --snapshots <dir> axle-fatigue.adj` (with the source placed as a
  content-addressed snapshot) **byte-anchors all six citations** — the 4 premises and 2
  inference warrants — against the pinned paragraph (`quotes_verified: 6`), and re-derives the
  thesis (`FromRule`, `rechecked`). A citation that drifted from the source would fail the run
  (`verified: false`, `quote_missing`) — see `argument_verify_e2e.rs`.

Together: **the argument a paragraph makes becomes a program the engine can run, and every
step of that program is auditable back to the paragraph's own bytes.** Driven end to end by
`adj-lang-cli/tests/argument_worked_example_e2e.rs`.
