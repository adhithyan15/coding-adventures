# ADJ-ARGUMENT-IR — a byte-grounded argument graph, native to the ADJ substrate

**Status:** Spec-first. The decomposition target that lets ADJ take apart an *argument* —
a paragraph of a research paper, a legal holding, a policy memo — into premises, inference
steps, and conclusions that the **engine reasons over** and the audit trail can **re-check
and explain**. The north-star deliverable of the Substrate/RS campaign: *"we should be able
to decompose entire research papers … the language must reason and explain its reasoning."*
**Author:** substrate/RS campaign, 2026-07-30.

**Decision (owner-directed):** the argument graph is **adj-lang**, not a parallel typed IR.
A premise is a provenanced fact; an inference step is a rule with a body; a conclusion is a
derived head. The engine chains them; `--explain` renders the chain; `adj-verify` re-derives
it and re-checks its grounding. There is **no Python (or JSON-IR) rule layer in the middle** —
the LLM decomposer emits ADJ directly (cf. `project_adj_native_no_python_middle`).

---

## 1. Why now, and what's missing (grounded)

The campaign has built the pieces of an auditable reasoner: **RS-1/RS-2** made a formula *be*
a rule and gave rules **multi-step bodies** (`ADJ-RULE-SUBSTRATE.md`); **RS-3** added
long-horizon `statemachine` reasoning; **RS-4 (§E.8)** made the engine **explain** its trace;
**NUM-6v** made `adj-verify` re-execute both the logic *and* the arithmetic of a trail, failing
hard on any step that no longer holds. What ADJ can *reason over and explain* today is a
**rulebook + case** the author hand-writes or a closed-vocabulary decomposer emits.

What it cannot yet do is take a **paragraph of prose it has never seen** — open vocabulary, no
fixed relation set — and turn its **argument** into that same auditable substrate. The existing
decomposition stack is either:

- **Span/structure decomposition** (`ADJ01` IR grammar, `ADJ25` hierarchical
  Document→Sentence→Phrase→Claim→Component, in `adjudication-ir/src/lib.rs`): a generic
  byte-grounded typed *graph* with `NodeKind = Fact | Query | Uncertainty | Rule | …` and a
  closed edge taxonomy. It tiles a document into typed spans and coverage-checks it — but it
  has **no `Premise`/`Inference`/`Conclusion` node kinds** and **no support-typed inference
  edges** (`Supports`/`Rebuts`/`Warrants`) that cite the *connective's own bytes*. Its only
  inference-bearing edges (`Concludes`, `DerivedFrom`, `JustifiedBy`) are **engine-synthesized**
  proof-DAG links with empty `source_spans` by spec — not argument steps extracted from prose.
- **Closed-vocabulary clinical decomposition** (`decompose_query`, `warm/decompose.py`): maps a
  vignette to one `relation(subject,$Var)` query, or to flat `findings[].{type: stated|inferred}`
  annotations, against a fixed 33-relation medical vocabulary. Not open-vocab; not a graph.

Neither is an **open-vocabulary argument graph**. That is this spec.

The grounding *discipline* for it already exists and is reused wholesale: **ADJ61/62** — a claim
is grounded iff its cited bytes, **combined**, justify it (not a single verbatim span), with
claims typed `evidence` (strict) vs `conclusion` (hedged inference); **ADJ64** — a conclusion is
*underdetermined* if a rival hypothesis fits the same bytes and its discriminating observation is
absent (a named provenance hole); **ADJ42** — an independent adversary, blind to the extractor's
self-label, must fail to refute each link. These are the gate; the argument graph is the thing
they gate.

---

## 2. The model — an argument *is* a provenanced rule-chain

An **argument graph** `A` over a source document `D` (byte string) is a set of typed nodes and
directed edges. Every node and every edge carries `source_spans: Vec<Span>` into `D` (byte
offsets), exactly as `adjudication-ir::Span` / `CorrelationId` already do — so any element of the
argument is traceable to the bytes it was read from.

### 2.1 Nodes

- **Premise** — an asserted proposition the argument *starts from*. Sub-typed, reusing ADJ62's
  input kinds:
  - `extracted` — stated in `D` (strict grounding: the bytes *say* it).
  - `inferred` — a hedged reading the author had to infer from `D` (e.g. "He" → the subject is
    male): allowed, but marked, and gated as a `conclusion`-strength claim.
  - `imported` — a premise the paper *relies on but does not itself contain* (a cited result, a
    background fact). Its grounding is a **citation** to an external locator, verified the ADJ40
    way (recursively decompose the cited source, NLI-match), not to `D`'s bytes.
- **Inference** — a step that derives one proposition from others. Carries an **open-vocab
  relation label** (the connective as written: "therefore", "because", "suggests", "rules out")
  and its **own** `source_spans` (the connective bytes), plus a **warrant**: the justification,
  in ADJ61 form, quoting the bytes and saying *why* the antecedents entail the consequent.
- **Conclusion** — a derived proposition. A conclusion may be the antecedent of a further
  inference (arguments chain). The paper's *thesis* is the sink conclusion.

A node's **identity** is an ADJ-native term (a `relate`d atom or a compound), so two premises
that say the same thing unify through the **dictionary** synonym mechanism (`define … surface`,
cf. `feedback_provenance_justified_inference`) — "myocardial infarction" == "heart attack" — and
a conclusion *name* that never appears verbatim ("neurobrucellosis") is still expressible,
because grounding is justification-by-combined-bytes, not substring (ADJ61).

### 2.2 Edges

Two families, both `source_spans`-bearing:

- **Support** edges (`supports`, `entails`, `because`, `warrants`) — an antecedent → an
  inference, or an inference → its conclusion. These are the argument's spine.
- **Attack** edges (`rebuts`, `undercuts`, `contradicts`) — a premise or conclusion that
  *defeats* an inference (undercut) or a conclusion (rebut). A research argument is not a tree of
  support; it names its counterarguments. Attack edges desugar to ADJ's existing **defeasible
  precedence + negation-as-failure** (`ADJ73`, the `context`/`outranks_context` machinery), so a
  rebutted conclusion is *withdrawn by the engine*, not by a Python filter.

### 2.3 The desugaring — this is the whole point

The argument graph **lowers to adj-lang** with no new evaluator:

| Argument element        | adj-lang form                                                              | Reuses |
|-------------------------|----------------------------------------------------------------------------|--------|
| `extracted` premise     | `relate P(...)  source "<bytes>"  trust observed`                           | RS-1 provenanced facts |
| `imported` premise      | `relate P(...)  source "<quote>"  locator "<url>"  trust <tier>`            | A9 multi-source, ADJ40 citation-verify |
| `inferred` premise      | `relate P(...)  uncertain …  source "<bytes>"` (hedged)                     | ADJ65 uncertainty primitive |
| inference step          | `rule { head: C  when: P1, P2, …  source "<connective bytes>"  trust … }`   | RS-2 multi-step bodies |
| conclusion (query)      | `? C`  — the engine derives it, or abstains                                 | differential / proof-DAG |
| attack edge             | a defeater rule + `context` precedence so `C` is withdrawn under the attack | ADJ73 defeasible precedence |

Once lowered, **everything downstream is free**: the engine *derives* the thesis from the
premises by chaining the inference rules (the proof DAG **is** the argument graph); `--explain`
(§E.8) renders it premise-by-premise, connective-by-connective, with provenance on every line;
`adj-verify` re-derives it and — via the NUM-6v-style re-check extended to argument grounding
(§4) — confirms each premise's citation and each inference's warrant still hold. The argument a
paper makes becomes a program the engine can run, explain, and audit.

---

## 3. The grounding gate (open-vocabulary, per-element)

Decomposing *trusted* prose is not enough; the campaign's thesis is that **hallucination is an
accounting failure** — every asserted element must trace to bytes or be named as a gap. The gate
is the ADJ61/62/64 stack, applied **per premise and per inference-connective**:

1. **Byte-anchor (deterministic).** Every `source_spans` must be a verbatim slice of `D` (or, for
   an `imported` premise, of the cited source's snapshot). Fabricated citations are rejected
   before any semantic check. Coverage of `D` is tiled and checked (ADJ25 flat-coverage), so the
   decomposition cannot *silently drop* an argumentative move.
2. **Justification (semantic, adversarial).** For each `extracted` premise the cited bytes must
   *state* it (strict); for each `inference` the antecedents' bytes **combined with** the
   connective bytes must *warrant* the consequent (hedged). An independent adversary, blind to the
   decomposer's self-label (ADJ42), tries to refute each — a link survives only if unrefuted.
3. **Decision-sensitivity (don't over-abstain).** Per `project_adversarial_reading_and_cas_gate`,
   a LEAP inference is fatal only if it is **outcome-pivotal** — if its plausible alternatives
   flip the thesis. A locally-loose step that doesn't change the conclusion is flagged, not fatal.
4. **Underdetermination (ADJ64).** For the sink conclusion, enumerate rival theses that fit the
   same premises; each rival is *resolved* iff its discriminating observation is present-and-cited,
   else the conclusion is **UNDERDETERMINED** and the missing observation is a **named provenance
   hole** — a query for the spider / a follow-up read, not a fabricated answer.

The output of the gate is the three byte-disciplined states the framework already speaks:
**grounded / determined / underdetermined**. Only a fully-grounded, determined argument graph is
eligible for **CAS-write** (derive-once reuse) under the existing write gate (N adversarial
readers × byte-stable resampling × blind-judge concurrence).

---

## 4. Verification — `adj-verify` over an argument

`adj-verify` already re-executes the logic and arithmetic of a trail (NUM-6v). An argument graph,
being adj-lang, is re-checked by the *same* binary, extended with an **argument-grounding pass**
(the analogue of the narrowing re-check): for every inference `rule` and every `relate` premise
in the lowered program, re-confirm —

- the premise's `source`/`locator` bytes still verbatim-anchor to the pinned snapshot (existing
  quote re-check);
- the inference's **warrant** still holds under the adversarial reader (re-run the §3.2 refutation
  attempt against the pinned bytes, offline);
- the conclusion still **re-derives** from the premises (existing SLD/LR re-check — the proof DAG
  is the argument), and no **attack** edge now defeats it that didn't before.

A failure at any point is a hard `verified: false`, exactly as a narrowing mismatch is — so a
shared or cached argument that has since drifted from its sources is caught, not asserted.

---

## 5. Relationship to the existing IR (`adjudication-ir`)

This spec **does not fork** the ADJ01/25 span IR. Two clean options, settled in ADR-1:

- **Native path (preferred, owner-directed).** The argument graph is adj-lang directly; the
  ADJ01/25 IR remains the *span/coverage* front-end that tiles `D` and feeds the decomposer, and
  an argument node's `source_spans` are ADJ25 spans. The argument layer is new adj-lang surface
  (§6), not new `NodeKind`s. This keeps reasoning in the engine and avoids the Python middle.
- **IR-extension path (fallback, if a graph artifact is needed before lowering).** Add
  `NodeKind::{Premise, Inference, Conclusion}` and support/attack `EdgeRelation`s to
  `adjudication-ir/src/lib.rs` (the `EdgeRelation::DomainSpecific(String)` escape hatch already
  allows prototyping the relation labels before a taxonomy v-bump), emitting the argument graph as
  an ADJ01-family artifact that a thin lowerer turns into the adj-lang of §2.3.

The spec commits to the **native path** and treats the IR-extension only as an optional
serialization for tooling; the ladder (§7) is written for the native path.

---

## 6. Surface (sketch — settled in ADR-2)

Argument constructs are **recognised built-ins that desugar** to `relate`/`rule`, in the RS
"there is no second way to write it" spirit — so no parallel evaluator. Illustrative:

```
argument thesis_name {
    premise p1 : extracted  claim(...)   cite "<verbatim bytes>"
    premise p2 : imported   claim(...)   cite "<quote>"  locator "<url>"  trust authoritative
    infer  step1 : because  conclude interm(...)  from p1, p2
           warrant "<connective bytes> — why p1 and p2 entail interm"
    infer  step2 : therefore conclude thesis(...) from step1
           warrant "<bytes>"
    rebut  r1    : undercut step2  by claim(...)  cite "<bytes>"   % a named counterargument
}

? thesis(...)          % the engine derives the thesis, or abstains / reports UNDERDETERMINED
```

`premise`/`infer`/`conclude`/`from`/`warrant`/`rebut` are IDENT-matched literals (like
`formulabook`/`statemachine`), so the lexer is untouched. Each `cite`/`warrant` string is the
grounding payload the §3 gate checks. Final syntax fixed in ADR-2.

---

## 7. Staging (each: spec-sync → tests → impl → security-review → babysit)

- **ADR-1 (this PR, spec-first):** this document — the model, the desugaring table, the gate, the
  verification story, the native-vs-IR decision, the worked example (§8). Cross-links ADJ01/25/40/
  41/42/61-64, ADJ-RULE-SUBSTRATE, ADJ-REASON-MATH §E.8, ADJ-NUMERIC-SUBSTRATE. No code.
- **ADR-2 — argument surface:** the `argument { premise/infer/conclude/rebut }` grammar + AST +
  lowering to `relate`/`rule`/`? goal`, with an end-to-end test: a 3-premise argument derives its
  thesis and `--explain` renders it as premises → connective → conclusion. (Grammar via
  `regen_grammars`; mirrors the `statemachine` wiring.)
- **ADR-3 — the open-vocab grounding gate over the argument:** per-premise byte-anchor +
  combined-justification, adversarial reader, decision-sensitivity, applied to a lowered argument
  program; the gate's verdict (grounded / determined / underdetermined) emitted in the JSON.
- **ADR-4 — `adj-verify` argument pass:** the §4 re-checks (warrant re-refutation + re-derivation
  + attack-set stability) folded into `adj-verify`, with a tampered-warrant e2e that fails hard.
- **ADR-5 — worked research-paper decomposition end-to-end:** a real (open-access) paragraph →
  argument graph → engine derives the thesis → `--explain` → `adj-verify --snapshots`, one
  cross-domain example (deliberately non-medical), proving the whole pipeline on unseen prose.
- **Later:** the trained decomposer that *emits* this adj-lang from prose (reuses the F3 training
  harness, retargeted from closed-vocab findings to the open-vocab argument surface); multi-
  paragraph / whole-paper composition (a paper is a DAG of paragraph arguments sharing conclusions).

---

## 8. Worked example (illustrative; the real one lands in ADR-5)

Source paragraph (abridged, fabricated for illustration):

> "Because the alloy's operating stress (420 MPa) exceeded its fatigue limit (380 MPa), the axle
>  was destined to crack under cyclic loading; the fracture surface's beach marks confirm a
>  fatigue mechanism rather than a single overload."

Decomposed argument (native adj-lang, elided):

```
argument axle_failure {
    premise p1 : extracted  operating_stress(axle, 420)  cite "operating stress (420 MPa)"
    premise p2 : extracted  fatigue_limit(axle, 380)      cite "its fatigue limit (380 MPa)"
    premise p3 : extracted  shows(surface, beach_marks)   cite "beach marks"
    infer  s1 : because   conclude exceeds_limit(axle)
           from p1, p2   warrant "exceeded its fatigue limit — 420 > 380"
    infer  s2 : therefore conclude mechanism(axle, fatigue)
           from s1, p3   warrant "beach marks confirm a fatigue mechanism rather than overload"
    rebut  r1 : undercut s2 by shows(surface, single_shear)  % absent here → does not fire
}
? mechanism(axle, $M)
```

The engine derives `mechanism(axle, fatigue)`. `--explain` renders: p1 & p2 (each byte-cited) →
*because* → `exceeds_limit`; that plus p3 → *therefore* → `mechanism(axle, fatigue)`; r1's
undercut premise is absent, so it does not fire. `adj-verify` re-anchors the three citations,
re-refutes the two warrants, and re-derives the thesis. ADJ64 asks: does a rival (overload
fracture) fit the same bytes? Its discriminating observation (single-shear surface) is
**absent-and-uncited** → if the beach-marks premise were removed, the conclusion would be
UNDERDETERMINED, and "single-shear surface morphology" would be a named provenance hole.

---

## 9. Verification & invariants

- **Every argument element traces to bytes or is named as a gap** — the ADJ61/62/64 grid holds
  over premises *and* inference connectives; no element is asserted un-cited.
- **The engine reasons; Python is glue** — the thesis is *derived*, not decided in Python; the
  proof DAG is the argument graph; `--explain`/`adj-verify` operate unchanged.
- **Open vocabulary** — no fixed relation set; premise/conclusion identities are ADJ terms unified
  through the dictionary, and inference labels are the connectives as written.
- **Defeasible** — attack edges withdraw conclusions via the engine's precedence/NAF, not a filter.
- **Re-checkable** — a drifted or fabricated argument fails `adj-verify` hard, offline, in CI.
