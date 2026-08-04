# ADJ-ARGUMENT-DECOMPOSER — training a model to emit `argument` adj-lang from prose

Status: **Spec-first** (2026-07-30). No code in this PR. Opens the north-star front that
[`ADJ-ARGUMENT-IR.md`](ADJ-ARGUMENT-IR.md) was built to feed.

---

## 1. Why this exists

The Substrate/RS campaign's literal goal is *"we should be able to decompose entire research
papers … the language must reason and explain its reasoning."* We now have the **target
language** for that: [`ADJ-ARGUMENT-IR.md`](ADJ-ARGUMENT-IR.md) shipped the `argument` surface
(ADR-2), its structural + byte-anchor grounding gate (ADR-3/4), a worked paragraph decomposed
end-to-end (ADR-5), and `--explain` rendering of the derived chain (ADR-6). An `argument`
desugars to provenanced `relate` facts + `rule`s, the engine **derives** its thesis by SLD, and
`adj-verify` **byte-anchors** every citation back to the source. What is missing is the thing
that *produces* an `argument` from a paragraph of prose: a **decomposer**.

Today an `argument` is hand-authored. To decompose a research paper we need a model that reads a
paragraph and **emits the `argument` adj-lang directly** — premises citing verbatim byte slices,
inference steps citing their connective bytes, a `? thesis` query — with **zero** post-hoc
authoring. This spec defines that decomposer: its emission target, its training data, its
fidelity metric, its abstention discipline, and its staging.

### 1.1 What it is NOT (so it does not duplicate existing work)

Three decomposition stacks already exist; the argument decomposer is a **sibling**, not a
re-run of any of them:

| Existing | What it decomposes | Why it is not this |
|---|---|---|
| Span/structure — [`ADJ25-hierarchical-decomposition.md`](ADJ25-hierarchical-decomposition.md), the `adjudication-ir` crate | every byte → `Fact`/`Query`/`Uncertainty`/`Rule` nodes with coverage | has **no Premise/Inference/Conclusion kinds** and no prose-cited support/attack edges |
| Closed-vocab clinical — `warm/decompose.py`, `board/decompose_query.py` | a vignette → flat `findings`/`chart_facts`, or one recall query | a **fixed 33-relation** medical vocabulary; flat, not a graph; not open-vocab |
| Cited-source re-decomposition — [`ADJ40`](ADJ40-recursive-source-decomposition.md)/[`ADJ41`](ADJ41-decomposed-source-ir-store.md) | a *cited* source, to check it supports a claim | a **consumer** of an argument's citations, not a producer of an argument |

The argument decomposer is **open-vocab** (the predicate/entity vocabulary is the paragraph's
own, not a fixed list) and its output **is a graph** (premises → inferences → conclusion). Its
emission target is adj-lang text, not a JSON IR — consistent with
[`project_adj_native_no_python_middle`]: the LLM writes `argument { … }`, the compiler is the
only checker, no parallel typed-IR middle layer.

---

## 2. Emission target — the contract the model must satisfy

The model emits an **`.adj` program** conforming to the ADR-2 surface:

```
argument <name> {
    premise <pname> : <kind> <term>  quote "<verbatim slice>" at <offset> snapshot "<hex>"
                                     source "<doc>" trust <tier>
    …
    infer <sname> : <connective> conclude <term> from <ref>{, <ref>}
                                     quote "<verbatim slice>" at <offset> snapshot "<hex>"
                                     source "<doc>" trust <tier>
    …
}
? <thesis-term>
```

The output is **correct** iff all of:

1. **Compiles** — the emitted program lowers with no `LowerError` (well-formed premises,
   every `from` ref resolves, no duplicate names, ADR-3 structural grounding satisfied: every
   premise and every inference carries a non-empty `source`).
2. **Derives the thesis** — `adj-lang-cli <prog>` yields a non-empty `recall` answer for the
   `? thesis` query. The paragraph's conclusion is *reached by chaining the inference rules over
   the premise facts*, not asserted as a premise.
3. **Byte-anchors** — `adj-verify --snapshots <dir> <prog>` reports `verified: true` with
   `quotes_verified` equal to the number of cited elements (every premise + every inference
   warrant is a **verbatim slice** of the pinned source at its recorded offset).

These three are exactly the checks the ADR-5 worked example passes
(`argument_worked_example_e2e.rs`) and the ADR-6 `--explain` renders. They are the decomposer's
**hard gate**: an emission that fails any of the three is discarded, never scored as a partial
success in the wild (see §4 for how a partial IS scored during eval).

---

## 3. Training data — backward generation, reused from F3

The decomposer reuses the **backward-generation** methodology proven for the closed-vocab
shapes (`code/specs/data/mycin-2026/train/gen_data.py`, `gen_chart_data.py`): *sample the gold
structure first, then synthesize prose that states exactly it, then derive the label from the
prose deterministically.* This guarantees byte-provenance holds by construction and the label
is never a model's opinion.

For arguments the loop is:

1. **Sample a gold argument skeleton** — a small open-vocab graph: N premises (each a predicate
   over 1–2 entities), M inference steps (each concluding a new term from a subset of
   premises/prior conclusions), one thesis. Drawn from a generator, not a fixed vocabulary, so
   the predicate/entity names vary per example (open-vocab is the point).
2. **Teacher writes the paragraph** — a teacher model is asked to write a natural paragraph
   that *states each premise and asserts each inference's connective in its own words*, and
   nothing load-bearing beyond them. It also injects **near-miss distractor** sentences (see
   §3.1).
3. **Derive the gold `.adj` deterministically** — for each premise/inference, locate the
   **verbatim span** the teacher used, record its byte offset, and pin the paragraph's SHA-256
   as the snapshot. A premise whose span cannot be found verbatim is a **LEAP** and is *dropped*
   (mirrors `warm/ir_to_adj.py` dropping unentailed findings) — the gold argument only contains
   **ENTAILED** steps. Compute the offsets and hash with a tool, never by hand (byte-exact).
4. **Self-check** — the derived gold `.adj` must pass the §2 three-part gate against the
   generated paragraph. An example that does not is regenerated, never shipped.

### 3.1 Discard discipline (the hard negatives)

The single metric that most distinguishes a faithful decomposer from a fluent one is whether it
resists **near-miss** sentences — text that looks like a premise but must NOT become one. The
F3 `NEAR_MISS_DISTRACTORS` taxonomy transfers directly, plus argument-specific kinds:

- **wrong-subject** — states the predicate about a *different* entity than the thesis concerns.
- **hedge** — "may be", "could suggest" — a claim not actually asserted.
- **process-not-result** — describes a method, not a finding the argument rests on.
- **reference/background** — cites prior work, not a premise of *this* argument.
- **(argument-specific) unstated-warrant** — a plausible connective the paragraph never states;
  the model must not invent an `infer` step whose warrant is not in the bytes.
- **(argument-specific) counter-consideration** — a rebutting sentence ("although X…"); until
  the `rebut`/attack edge lands (ADR follow-up, needs ADJ73), it is a **discard**, not a premise.

Each gold example carries a `discard[]` list of these spans with a reason, exactly like the
findings/chart_facts schema.

### 3.2 Training-example schema

One JSONL line, extending the F3 schema with the `argument` shape:

```json
{
  "id": "...",
  "shape": "argument",
  "note": "<the paragraph — the pinned source document>",
  "gold": {
    "premises": [
      {"name": "p1", "kind": "extracted", "term": "stress_amplitude(axle, 420)",
       "span": "a stress amplitude of 420 MPa", "type": "stated"}
    ],
    "inferences": [
      {"name": "s1", "connective": "because", "conclusion": "exceeds_endurance(axle)",
       "from": ["p1", "p2"], "span": "exceeds its endurance limit", "type": "stated"}
    ],
    "thesis": "failed_by(axle, fatigue)",
    "discard": [{"span": "surface corrosion was noted on an adjacent part",
                 "reason": "wrong-subject"}]
  }
}
```

`span` is the **verbatim** substring of `note` (span-faithfulness is a byte-substring check, as
in `decompose_score.py`). Offsets and the snapshot hash are computed from `note` at gold-build
time, not stored, so they can never drift from the text.

---

## 4. Fidelity metric — model-free, extends `decompose_score.py`

Scoring is a **pure function** `(predicted_gold, gold, note) → metrics`, no model or network —
the same discipline as the L17 `decompose_score.py`. The argument shape adds structural metrics
on top of the existing precision/recall/faithfulness set:

- **premise P/R/F1** — over `(kind, term, normalized-span)` tuples (a premise matches only if
  its span is right, so a fluent-but-unfaithful premise scores 0).
- **inference P/R/F1** — over `(connective, conclusion, from-set, span)` — an inference must
  cite the right premises *and* the right connective bytes.
- **thesis-derivation** (0/1) — does the predicted argument, once compiled, actually **derive**
  the gold thesis? (the §2.2 check, run for real via `adj-lang-cli`). This is the headline
  end-to-end metric: a decomposition that does not reach the conclusion has failed regardless of
  per-premise F1.
- **span-faithfulness** — every predicted span is a verbatim substring of `note` (the §2.3
  byte-anchor, pre-checked cheaply as a substring before the real `adj-verify` run).
- **near_miss_violations** (must be 0) — a predicted premise/inference whose span equals a gold
  **discard** span. This is the one metric that can veto an otherwise-high-scoring output, as in
  F3.
- **fabrication** — a predicted premise/inference whose span is NOT in `note` at all (invented
  bytes). Must be 0 — the strongest possible faithfulness failure.

A held-out `argument_decompose_eval.jsonl` + a runner (mirroring `eval_decompose.py`) provides
`--self-check` (score gold as its own prediction → must be all-perfect), `--pred` (score a
predictions file), and `--model` (run a fine-tune over the set). Gold self-consistency is
pinned by a unit test, as for the existing shapes.

---

## 5. Abstention — never fabricate an argument

The decomposer inherits ADJ's core discipline: **an ungrounded step is not emitted.** Concretely:

- A premise/inference the model cannot tie to a **verbatim span** is dropped (LEAP), never
  emitted with a fabricated citation. If dropping it breaks the derivation, the whole paragraph
  **abstains** — output no argument, flag the provenance hole — rather than ship a chain that
  does not reach its thesis or cites bytes that are not there.
- When rival theses are equally supported by the paragraph, the decomposer emits the
  **underdetermination** signal (reuse ADJ64) — named provenance holes, not a coin-flip pick.
- The §4 `fabrication` and `near_miss_violations` metrics being non-zero on the held-out set is
  a **release blocker**, not a soft target: a decomposer that invents premises is worse than one
  that abstains, because the downstream engine will faithfully derive a false thesis from them.

This is the same asymmetry the recall/board path already enforces (`decompose_query.py`'s
two-sided gate → ABSTAIN): the model's only job is to *transcribe the argument the bytes make*,
never to *supply* one.

---

## 6. Staging (each its own PR; specs → data → harness → eval)

- **AD-1 (this PR)** — the spec: emission target, backward-generation loop, schema, metric,
  abstention. No code.
- **AD-2** — the gold **generator + deterministic gold-builder** (`gen_argument_data.py`): sample
  skeleton → offsets/hash tool → self-check against the §2 three-part gate. Ships a small seed
  set of gold `argument` examples (each of which already passes `adj-lang-cli` derive +
  `adj-verify`, reusing the ADR-5 fixtures as the first rows).
- **AD-3** — the **model-free scorer** (`argument_decompose_score.py`) + its unit tests
  (near-miss veto, fabrication veto, thesis-derivation via the real CLI), extending
  `decompose_score.py`.
- **AD-4** — the **held-out eval set** (`argument_decompose_eval.jsonl`) + runner
  (`--self-check`/`--pred`/`--model`), mirroring `eval_decompose.py`.
- **AD-5** — a **decompose→emit→verify** worked pipeline: a paragraph → the model's `argument`
  → `adj-lang-cli` derives the thesis → `adj-verify` byte-anchors it → `--explain` renders the
  chain. The end-to-end proof that a paragraph becomes a program the engine runs, explains, and
  audits — with the *emission* now done by the model, not by hand.
- **Later** — multi-paragraph / whole-paper composition (a paper = a DAG of paragraph arguments
  sharing conclusions); the `rebut`/attack edge (needs ADJ73 defeasibility) so a
  counter-consideration becomes a real attack instead of a discard.

## 7. Reuse map (no parallel engine)

- **Emission target**: [`ADJ-ARGUMENT-IR.md`](ADJ-ARGUMENT-IR.md) §2/§6 (surface), §2.3
  (desugaring), §4 (verify), §8 (worked example) — the decomposer's output IS this language.
- **Backward generation + discard taxonomy + fidelity scorer + held-out eval**:
  `code/specs/data/mycin-2026/train/{gen_data.py, decompose_score.py, eval_decompose.py}` —
  extended, not forked.
- **Grounding discipline**: ADJ61/62 (combined-span grounding), ADJ64 (underdetermination),
  ADJ42 (blind adversary) — the same stack the argument grounding gate already names.
- **Structural decomposition context**: [`ADJ25`](ADJ25-hierarchical-decomposition.md) (coverage
  invariant), [`ADJ40`](ADJ40-recursive-source-decomposition.md)/[`ADJ41`](ADJ41-decomposed-source-ir-store.md)
  (source re-decomposition — the *consumer* of what this produces).
