# ADJ-FORMULA-LIBRARIES — the graded, byte-provenanced *formula* standard library (the real ladder)

**Status:** Spec-first. Reframes and grounds the "standard library" that
[ADJ-LADDER](ADJ-LADDER.md) always named but never built. Supersedes the
inline-`items.json` rung treadmill (rungs 0–125): those stay as a *question bank*, but
the arithmetic they contain is **harvested into importable libraries** here.
**Author:** direction-correction pass, 2026-07-11.

**North star (unchanged, now made real):** a small, non-frontier model (Gemma-/Haiku-class,
fully local) passes a Medical Licensing Exam the model *alone* cannot — because the
**reasoning lives in the framework**: a **content-addressed standard library** of grounded
formulas the model *imports* rather than re-derives, and the engine computes over the CPU,
exactly, with **zero math performed by the LLM**. Built as a curriculum that climbs from
**what a kindergartner learns → medical school**, MLE as the apex you build *up to*.

---

## 1. What went wrong, precisely

ADJ-LADDER's Arm B was supposed to be: *the model decomposes a question into an ADJ program
that imports a grounded library; the engine computes.* In practice every rung shipped a
**self-contained** program that **inlines** the formula:

```
observe motor_response(18)
observe baseline_amplitude(7)
observe decrement_drop(3)
observe facilitation_gain(2)
let answer = motor_response / (baseline_amplitude - decrement_drop + facilitation_gain)   % ← inlined, per item
? answer
```

Nothing is a library; nothing is imported; no term enters a dictionary with provenance; the
model would be *handed* the formula rather than **recalling** it. So the "ladder" degenerated
into enumerating denominator arrangements (`a/(b-c+d)`, `a/(b-c-d)`, …) — 125 rungs proving the
same narrow claim ("the engine can evaluate arithmetic shape N"), with **no reusable asset** and
no exercise of the import-and-recall step the thesis depends on.

**Root cause (confirmed against the grammar):** ADJ has `dictionary`/`define` (terms),
`rulebook`/`relate` (ground relational facts — the 127 recall libraries), `import`/`use`, and
inline `let <name> = <expr>` — but **no construct for a reusable, parameterized formula that
lives in a library and is imported and applied.** The formula could only ever be inline. Fixing
that is rung-0 of the *real* ladder.

---

## 2. The hypothesis this spec operationalizes

> Most fields decompose into a series of **terms** (a dictionary). Terms with a mathematical
> foundation compose into **formulas** with variables. Expressed in ADJ, a formula becomes an
> **importable library**. At answer time the small model does only two things: **decompose the
> input with byte-provenance** and **bind the variables** to the extracted values. The ADJ
> engine then imports the library and **reasons over the CPU**, emitting an answer (or abstains).

Two consequences the design must honor:

1. **Write-once, use-many & compositional.** A clinical formula (e.g. Cockcroft–Gault
   creatinine clearance) *imports* lower-level libraries (a `ratio`, a `product`, a
   unit-conversion) rather than restating them. The curriculum is a DAG, not a flat list.
2. **The libraries themselves carry byte-provenance.** A formula is a *claim about the world*
   ("BMI is defined as mass ÷ height²"). It is not human-asserted; it is grounded in a citable
   source span with a trust tier — exactly like a `relate` edge. At answer time the engine can
   show the **full chain**: the formula's cited definition **+** the model's byte-provenance for
   every bound variable. (Ties to the standing rule: *nothing enters the CAS human-authored;
   facts arrive spider→provenance→adversarial-gate.*)

---

## 3. Rung-0 — the substrate: `formulabook` + `formula name(params) = expr`

A new top-level construct, a sibling of `dictionary` and `rulebook`, importable via `use`/`import`.

```adj
% code/…/stdlib/clinical/bmi.adj — the body-mass-index formula library.
dictionary bmi_vocab {
    define body_mass : quantity(mass)    surface "weight", "body weight", "mass"
    define height    : quantity(length)  surface "height", "stature"
    define bmi       : quantity          surface "body mass index", "BMI"
}

formulabook body_metrics {
    use bmi_vocab

    formula bmi(body_mass, height) = body_mass / (height * height)
        source "BMI is defined as body mass divided by the square of body height … kg/m²."
        locator "https://www.who.int/…/body-mass-index"
        trust authoritative
}
```

### 3.1 Grammar (additive; regenerated via `regen_grammars`, never hand-edited)

- `formulabook <ident> { use <dict>… <formula>… }` — a named collection; `use` brings a
  dictionary's terms into scope so the formula's parameter names are typed vocabulary, not free
  strings.
- `formula <name>(<param>, …) = <expr>` — `<expr>` is **the existing `let` expression grammar**
  (`ExprAst`: `Num`, `Ref`, `Bin(ArithOp,…)`, `Call(NamedFn,…)`, aggregations, the transcendental
  set). The **only** new idea: the leaves that name a `<param>` are **formal parameters**, bound at
  apply time, instead of `Ref`s to already-`observe`d facts.
- Each `formula` carries the **same provenance envelope as `relate`**:
  `source "<byte-quotable span>" locator "<url/isbn/loc>" trust <tier>`
  (AST `Cites { source, locator }` + `TrustTierName`). Provenance is **required** on a `formula`
  in a shipped library — the linter rejects an unsourced formula, mirroring the recall-library gate.

### 3.2 AST & lowering

- `FormulaDef { name, params: Vec<Term>, body: ExprAst, cites: Cites, trust: TrustTierName }`.
- The body is a **parameterized `ExprAst`**: a leaf `Ref(p)` where `p ∈ params` is a *parameter
  reference*; any other `Ref` must resolve to a term/fact in scope (a validation error otherwise).
- **Apply semantics** (`? bmi(body_mass, height)` in a consumer): the engine binds each parameter to
  the correspondingly-named `observe`d fact (or to the argument expression), substitutes into the
  body, and evaluates it through the **existing `ComputeExpr` evaluator** — the same exact/dimensional
  CPU path `let` already uses. No new evaluator; a formula is a *named, importable, reusable `let`*.
- The derivation tree records: the `FormulaDef`'s `cites`/`trust`, plus each parameter's binding and
  *its* provenance (the observed fact's source span). One computed answer ⇒ one auditable chain.

### 3.3 Consumer surface (what the model writes at answer time)

```adj
import "stdlib/clinical/bmi.adj"

observe body_mass(quantity(70, kg))     % ← bound by the model from byte-provenance decomposition
observe height(quantity(1.75, m))       % ← "…height of 1.75 m…"  (span cited)

? bmi(body_mass, height)                % engine imports the library, applies the cited formula → 22.86 kg/m²
```

The model states **no arithmetic** — it recalls *which* library applies and binds the variables.
The formula (and its source) live in the library; the numbers (and their spans) come from the input.

### 3.4 Non-goals for rung-0

No recursion, no higher-order formulas, no user-defined control flow. A `formula` is a pure,
total, parameterized arithmetic expression over the already-supported op set. Piecewise/threshold
behavior stays in the existing `contributes … from <pred>`/`constrain` machinery, composed *around*
formulas, not baked into them.

---

## 4. The curriculum — kindergarten → medical school (a DAG of libraries)

Each library = **grounded dictionary terms** (provenanced where the term itself is a claim) +
**one or more provenanced formulas** + a **decompose-and-bind worked query** + an **end-to-end
test**. Higher layers `import` lower ones (write-once-use-many).

| Layer | Libraries (formulas) | Imports |
|------|----------------------|---------|
| **K / early** | `count`, `add`, `subtract`, `compare` | — |
| **Elementary** | `multiply`, `divide`, `fraction`, `ratio`, `percent`, `average` | K layer |
| **Middle / HS** | `rate`, `proportion`, `unit_convert`, `power`, `root`, `area`, `volume` | elementary |
| **Pre-clinical** | `concentration`, `dosage`, `clearance`, `mean_arterial_pressure`, `bsa` | HS + units |
| **Clinical (MLE apex)** | `bmi`, `anion_gap`, `corrected_calcium`, `fena`, `cockcroft_gault_crcl`, `egfr`, `winters_formula`, weight-based dosing | pre-clinical + ratio/product/unit libs |

The **arithmetic content already exists** in rungs 0–125 (BMI@30, FENa@15, TTKG@13, MCV/MCH/MCHC@19,
De Ritis@20, lipid/iron indices, Starling@31, …). §6 harvests it.

The **knowledge** standard library already exists too: the **127 recall `.adj` libraries** (MICRO,
PHARM, CARDIO, …). Formula libraries are the **compute** standard library. Together they are the
importable stdlib a model draws on; an MLE item that needs *both* recall and compute is the apex.

---

## 5. Byte-provenance for the libraries (the non-negotiable)

- **Every shipped `formula` is sourced.** `source` is a byte-quotable span from an authoritative
  reference (WHO/NIH/textbook/primary literature); `locator` resolves it; `trust` tiers it. The
  recall-library **adversarial write gate** applies unchanged: a formula enters the stdlib only via
  spider→provenance→gate, never by fiat. Humans *correct*, they don't author.
- **Term definitions that are themselves claims** (a named clinical quantity, a reference range)
  carry provenance too; purely structural vocabulary (`define addend : number`) does not.
- **Answer-time chain.** `? formula(...)` yields, alongside the value, a derivation object citing:
  (a) the formula's `source`/`locator`/`trust`, and (b) for each bound parameter, the observed
  fact's byte-provenance span in the input. An independent checker (`adj-verify`) re-verifies the
  computation and the citations without the model. This is the paper thesis in miniature:
  *hallucination is an accounting failure; every step is auditable.*

---

## 6. Harvest plan — turn the 125 rungs into libraries (no work discarded)

For each existing rung, lift its FAMILY headline formula into the curriculum:

1. Map the rung's four observed quantities → **dictionary terms** (with dimensions + synonyms).
2. Lift the headline formula → a provenanced **`formula`** in the right layer (source it to the
   real definition; e.g. rung-30 BMI → WHO; rung-15 FENa → a nephrology reference).
3. Replace the rung's per-item inline `let` with an **`import` + bind + `? formula(...)`** query;
   the 21 items become **decompose-and-bind** exercises over the library (the model must recall the
   library and bind the numbers, not be handed the expression).
4. Keep the two-arm scoring — but Arm B now measures *recall-the-library + bind*, the real skill.

Shapes that were pure benchmark filler (the denominator-arrangement rungs with no named clinical
formula) collapse into the small set of **primitive** libraries (`ratio`, `sum_over_difference`, …)
they were all instances of — one library, not fifteen rungs.

---

## 7. The evaluation — decompose → bind → import → compute (converges with the pipeline proof)

The instrument stays two-arm (ADJ-LADDER §3) and reuses the existing harnesses (board-eval,
`decompose_query`, the offline board pipeline, the two-arm ladder scorer — mapped separately). The
**task changes**: given a natural-language item, the small local model must

1. **recall** which formula library (and/or recall library) applies,
2. **decompose** the input with **byte-provenance**, binding each parameter to a cited span,
3. **import + compute** (engine, CPU, exact) — **or abstain** when it cannot ground a binding.

Metrics: Arm A (model alone) vs Arm B (model + stdlib); the **B − A divergence** is the headline;
**abstention** is first-class (a grounded "I don't know" beats a confident wrong answer). The
end-to-end claim we are driving toward: *a Gemma-class local model + the ADJ formula & recall stdlib
passes an MLE slice it fails alone, with an audit trail an independent checker re-verifies.*

---

## 8. PR staging (specs → tests → impl → provenance-gate → changelog → review)

- **PR-0 (substrate):** `formulabook`/`formula` grammar + AST + validation (parameter scoping) +
  apply-lowering to the existing `ComputeExpr` evaluator + `import`/`use` wiring + `adj-lang-cli`
  render/eval + the provenance-required linter rule. Ships with a worked `bmi.adj` + an end-to-end
  test (`import` → bind → compute → assert value **and** citation chain). **This is the unblocker.**
- **PR-1…:** the curriculum, **bottom-up**, one small library per PR (K → elementary → …), each with
  terms + provenanced formula(s) + a decompose-bind query + an end-to-end test; higher layers import
  lower ones. Interleave the **harvest** of the matching existing rung.
- **PR-clinical:** the MLE-apex libraries, composing pre-clinical + primitive libs; wire into the
  board-eval harness for the two-arm + abstention run.

Each PR is a durable, composable asset — the anti-treadmill property: value compounds instead of
enumerating.

---

## 9. Verification & invariants

- `cargo test`/`cargo clippy` green per crate touched; `adj-lang` grammar regenerated via the CLI
  bin, never hand-edited.
- **Zero LLM arithmetic:** the model emits only `observe … / ? formula(…)`; every number in the
  answer is computed by the engine or cited from the input.
- **Provenance-complete:** no shipped `formula` without `source`/`locator`/`trust`; the linter
  enforces it; `adj-verify` re-checks the answer-time chain offline.
- **Compositional:** clinical libraries `import` primitive ones; no formula is restated across
  layers (a duplication check in CI).
- **Exact/dimensional:** formulas run the existing exact/dimensional CPU path; unit mismatches are
  errors, not silent coercions.
```
