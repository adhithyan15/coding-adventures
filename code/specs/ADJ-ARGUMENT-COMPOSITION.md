# ADJ-ARGUMENT-COMPOSITION — composing paragraph arguments into a whole-paper argument

Status: **Spec-first** (2026-07-30). No code in this PR. The layer above
[`ADJ-ARGUMENT-IR.md`](ADJ-ARGUMENT-IR.md) (a single paragraph → an argument) that reaches the
literal north star: *decompose an **entire research paper***.

---

## 1. The claim, and the finding

A research paper is not one argument; it is a **DAG of paragraph arguments that share
conclusions** — the *methods* paragraph concludes something the *results* paragraph takes as a
premise, whose conclusion the *discussion* paragraph uses to reach the paper's overall thesis.
Composition is the layer that chains those paragraph arguments into one.

**Finding (empirically verified, not speculative): whole-paper composition is ALREADY supported
by the existing `argument` surface — it needs ZERO new language constructs.** Because a paragraph
argument desugars to provenanced facts + rules ([`ADJ-ARGUMENT-IR.md`](ADJ-ARGUMENT-IR.md) §2.3),
and the engine chains rules over facts *regardless of which paragraph they came from*, a paper is
composed by placing its paragraphs' premises and inferences in **one `argument` block** and having
a later paragraph's inference reference an earlier paragraph's conclusion by name. The engine
derives the paper's overall thesis by chaining across paragraphs, and — critically — **each proof
step keeps its own paragraph's provenance**, so byte-anchoring stays per-paragraph.

This spec's job is therefore **not** to add a construct but to (a) pin down the usage pattern that
makes composition sound, (b) name the grounding discipline across paragraphs, and (c) identify the
one real limitation and the *optional* ergonomic sugar that would remove it.

## 2. The composition pattern (verified)

A two-paragraph paper — paragraph A (methods) establishes `exceeds_endurance(axle)`; paragraph B
(discussion) uses it to reach the thesis `failed_by(axle, fatigue)`:

```
argument paper {
    premise a1 : extracted stress_amplitude(axle, 420)  quote "…" at … snapshot "<sha-A>" source "paper §2" trust authoritative
    premise a2 : extracted endurance_limit(axle, 380)    quote "…" at … snapshot "<sha-A>" source "paper §2" trust authoritative
    premise b1 : extracted shows(surface, beach_marks)   quote "…" at … snapshot "<sha-B>" source "paper §4" trust authoritative
    infer as1 : because   conclude exceeds_endurance(axle) from a1, a2  source "paper §2" trust authoritative
    infer bs1 : therefore  conclude failed_by(axle, fatigue) from as1, b1  source "paper §4" trust authoritative
}
? failed_by(axle, $Mechanism)
```

Running `adj-lang-cli` on this **derives `failed_by(axle, fatigue)`**, and the proof DAG is the
cross-paragraph argument — the thesis rule (`bs1`, paragraph B) chains into paragraph A's rule
(`as1`) which chains into paragraph A's facts, each step tagged with its paragraph's source:

```
failed_by(axle, fatigue)   [rule, source "paper §4"]        ← paragraph B
  exceeds_endurance(axle)  [rule, source "paper §2"]        ← paragraph A (the shared conclusion)
    stress_amplitude(axle, 420)  [fact, source "paper §2"]
    endurance_limit(axle, 380)   [fact, source "paper §2"]
  shows(surface, beach_marks)    [fact, source "paper §4"]  ← paragraph B
```

and `adj-lang-cli --explain` renders exactly that as premises → connective → conclusion (ADR-6),
with the mixed `source "paper §2"` / `source "paper §4"` provenance distinguishing the paragraphs.

**The load-bearing mechanism:** an inference's `from <name>` where `<name>` is *another inference*
uses that inference's **conclusion as a subgoal** (this is the axle two-step, §8 of ADR). So a
paragraph-B inference that lists a paragraph-A inference in its `from` list makes paragraph A's
conclusion a **genuine subgoal** — not an asserted fact. The engine must actually *derive* the
shared conclusion for the paper's thesis to hold: if paragraph A's support is removed, paragraph B's
thesis no longer derives. That is real composition, not restatement.

## 3. Grounding discipline across paragraphs

- **Each paragraph's premises are byte-anchored to that paragraph's own pinned snapshot.** In the
  block above, `a1`/`a2` cite `snapshot "<sha-A>"` (the methods paragraph's SHA-256); `b1` cites
  `snapshot "<sha-B>"` (the discussion paragraph's). `adj-verify --snapshots <dir>` — with *every*
  paragraph placed as a content-addressed snapshot in `<dir>` — re-anchors each citation against
  **its own** paragraph. Composition is therefore **multi-snapshot**: the document verifies iff
  every paragraph's citations verify against the paragraph they came from. (Empirical confirmation
  of multi-snapshot verify is AC-2.)
- **A shared conclusion carries no new byte-cite.** `exceeds_endurance(axle)` is a *derived*
  intermediate, not a quote — paragraph B references it structurally (`from as1`), and its
  soundness is that it **derives** from paragraph A's byte-anchored premises. Nothing is asserted
  un-grounded: the leaves of the cross-paragraph proof are all byte-anchored premises; the interior
  is inference.
- **A cross-paragraph link is sound iff the referenced conclusion actually derives.** If paragraph
  A underdetermines its conclusion (ADJ64), the paper's thesis inherits that hole — the composed
  argument is only as grounded as its weakest paragraph, and `adj-verify` + the ADJ61/62/64 grid
  surface that at the document level for free.

## 4. Decision: reuse, don't add a construct

Per the generic-substrate principle ([[project_adj_universal_rule_substrate]]) and "the engine
reasons; Python is glue", **AC-1 adopts the single-block usage pattern above as the composition
model. No `document` / `compose` / `paper` construct is added.** A paper is one `argument` whose
premises and inferences span its paragraphs, distinguished by per-step provenance, linked by
inference-to-inference `from` references. The whole existing stack — derive, `adj-verify`
(multi-snapshot), `--explain`, ADJ73 defeasibility, ADJ64 underdetermination — operates on it
unchanged.

### 4.1 The one real limitation, and optional future sugar

Two ergonomic frictions were found; **neither blocks the capability**, so both are deferred:

1. **Ordering.** The `argument` grammar is `{ premise } { infer }` — all premises precede all
   inferences. A multi-paragraph paper must therefore group *all* premises, then *all* inferences,
   losing the visual paragraph-by-paragraph grouping. (The provenance on each line still names the
   paragraph, so nothing is *lost* — only the source layout.)
2. **Single namespace.** `from` references are block-local: separate `argument para_a { … }`
   `argument para_b { … }` blocks **cannot** reference across each other (`ArgUnknownReference`).
   Composition therefore lives in one block.

**Optional future sugar (deferred, not required):** a `document { paragraph <name> { … } … }`
grouping that (a) preserves per-paragraph source blocks and their snapshots, and (b) resolves
qualified cross-paragraph references (`from para_a.as1`) by flattening to the single-block form this
spec already validates. It would be pure surface ergonomics over the *same* desugaring — worth doing
only if paper-scale authoring proves the flat block unwieldy. AC-1 does **not** commit to it.

## 5. Staging

- **AC-1 (this PR)** — the spec: the composition finding, the usage pattern, the multi-paragraph
  grounding discipline, the reuse decision. No code.
- **AC-2** — a worked **multi-paragraph** example end-to-end: 2–3 linked paragraph arguments
  (each its own pinned source snapshot) whose conclusions chain to a paper-level thesis, driven
  through **derive → multi-snapshot verify → `--explain`** (reusing `gen_argument_data` +
  `decompose_pipeline` patterns). This empirically confirms multi-snapshot `adj-verify` and commits
  the first whole-paper worked example.
- **AC-3** — a compose driver: extend `decompose_pipeline.py` to accept a *paper* (a list of
  paragraph sources + their arguments), place every paragraph as a snapshot, and run the whole-doc
  four-stage pipeline, printing the cross-paragraph chain with per-paragraph provenance.
- **Later** — if authoring at paper scale proves the flat block unwieldy, the optional
  `document`/`paragraph` sugar (§4.1); and the trained decomposer emitting a whole-paper argument
  from a multi-paragraph document (reuses the AD-1..5 scaffold, retargeted from one paragraph to a
  paper).

## 6. Reuse map

- **Surface + engine chaining**: [`ADJ-ARGUMENT-IR.md`](ADJ-ARGUMENT-IR.md) — the `argument`
  desugaring and inference-to-inference subgoal chaining that composition rides on, unchanged.
- **Emit / derive / verify / explain**: the AD-1..5 scaffold
  ([`ADJ-ARGUMENT-DECOMPOSER.md`](ADJ-ARGUMENT-DECOMPOSER.md) and
  `code/specs/data/mycin-2026/train/{gen_argument_data.py, decompose_pipeline.py}`), which already
  emit/derive/verify/explain a single argument and extend to multi-snapshot with no new engine code.
- **Grounding grid**: ADJ61/62 (combined-span justification), ADJ64 (underdetermination), ADJ42
  (blind adversary) — applied per paragraph, composed at the document level.
