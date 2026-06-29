# Changelog — adj-ladder

All notable changes to the ADJ-LADDER two-arm reasoning scoreboard.

## [0.27.0] — 2026-06-29

### Added — rung 4: dimensional analysis, the MULTIPLICATION arm

- New **`rung4_products/items.json`**: 20 self-authored word problems whose answer is a
  **composite-unit quantity formed by multiplying two quantities** — work = force × distance
  (`N·m`), energy = power × time (`W·s`), impulse = force × time (`N·s`), charge = current × time
  (`A·s`), pressure × volume (`Pa·L`), apparent power = voltage × current (`V·A`). This is the
  multiplication counterpart to `rung4_dimensional` (the division arm: `km/h`, `mol/l`, …) and
  reuses the same `compute_dimensioned` harness — no harness or engine change.
- Each gold program `observe`s two typed quantities and binds `let answer = a * b`; the **engine**
  forms the composite tag `a·b` via `Dimension::combine` (combine_mul), never the model. Every item
  carries a wrong-unit distractor including the **reversed-order** composite (`m·n` when the engine
  produces `n·m` — an operand-order trap a number-only reasoner falls for) and a single-factor unit
  (`n`). Engine selects **20/20 in cached mode, zero wrong, zero unit mismatches**.
- Registered in `test_ladder_eval.py` `SELF_CONTAINED_RUNGS`; cached + contamination + json gates green.

## [0.26.0] — 2026-06-29

### Added — rung 4: dimensional analysis (the dimensional engine)

- New **`rung4_dimensional/items.json`**: 20 self-authored word problems whose answer is a
  **unit-bearing quantity** — speed (km/h, m/s), molarity (mol/l), flow rate (ml/min, ml/s),
  concentration (mg/ml, g/l), density (g/cc), dose rate (mg/h), and a dimensionless ratio
  (scalar). This is the *dimensional engine* rung promised by `rung4_physics_chem` (whose
  first PR deliberately kept units in prose only); see ADJ-LADDER.md §5.
- Each gold program `observe`s typed quantities (`quantity(240, km)`) and binds
  `let answer = distance / time`. The **engine** carries the dimension through the division
  via `Dimension::combine` and reports the result as `80 km/h` — the unit tag is computed,
  never written by the model. Every option is a `{"value", "unit"}` object and the new
  `compute_dimensioned` answer-extractor demands the engine's **(magnitude, unit)** match
  BOTH, so every item carries a wrong-unit-same-magnitude distractor (80 m/s vs 80 km/h)
  that a number-only reasoner falls for and the dimensional engine rejects.
- Engine selects **20/20 in cached mode with zero miscomputations and zero unit mismatches**
  — the same hard gate (`wrong == 0`) every self-contained rung must pass.

### Added — harness

- `ladder_eval.py`: `compute_dimensioned` answer type — `compute_dimensioned_to_letter`
  reads the CLI's new `derived` section and matches `_letter_for_engine_dimensioned`
  (value AND unit); `_option_dimensioned` parses the `{value, unit}` option shape;
  `program_engine_trace` audits a dimensional item as `derived/<unit-tag>`.
- `contamination_check.py`: `dimensioned_option_signature` — dimensioned options are
  distinct iff they differ in magnitude OR unit (a wrong-unit distractor is a legitimate,
  distinct choice), and the gold-maps-to-engine check runs the dimensional program through
  the CLI like every other program rung.
- Depends on a new `adj-lang-cli` `derived` JSON section (surfacing the engine's existing
  per-`let` dimensional analysis) and a read-only `KnowledgeBase::derived_bindings()`
  accessor in `logic-engine`.

## [0.25.0] — 2026-06-28

### Added — rung 4: physics & chemistry word problems (first PR)

- New **`rung4_physics_chem/items.json`**: 20 fresh, self-authored applied-science MCQs
  spanning kinematics (speed/distance/time), density, Ohm's law (V/I/R), unit
  conversions (min→s, km→m, kg→g, hr→min), molarity, stoichiometry (moles↔mass), and
  force/work/power/pressure. This is the ladder's first climb past pure algebra into
  applied science — the roadmap's rung 4 (ADJ-LADDER.md §5).
- Scoped this first rung-4 PR to the **exact-compute formula path**: every gold
  decomposition is a plain ASCII arithmetic expression whose numbers all appear in the
  stem (conversion factors such as "1 minute equals 60 seconds" are stated in the stem,
  so no constant is smuggled past the no-result-literals gate). The engine selects
  **20/20 in cached mode with zero miscomputations** — the same hard gate every rung
  passes. Unit symbols (km/h, g/cm³, mol/L, N, J, W, Pa) live in the prose; carrying
  dimensions through the engine as first-class *typed quantities* (so a wrong-unit answer
  is rejected) is the next rung-4 PR (dimensional engine, §5).
- Registered `rung4_physics_chem` in `test_ladder_eval.py`'s `SELF_CONTAINED_RUNGS`, so
  the contamination, ≥20-item, and cached-engine end-to-end tests now cover it. Full
  suite: **102 tests pass**; `contamination_check.py rung4_physics_chem` clean.
- A local-Gemma two-arm headline (like the rung-0/rung-3 tables) will be recorded as a
  follow-up; the divergence is expected to widen here.

## [0.24.0] — 2026-06-29

### Added — Gemma cubic-roots baseline

- Added **`ladder-scorecard.rung3_cubic_roots.gemma.json`**, a full 20-item
  local Gemma-3-4b trace artifact for native ADJ cubic root-solving questions.
- Recorded the current two-arm baseline: Gemma alone scored 16/20 with 4 wrong
  direct answers, while Gemma + ADJ scored 20/20 with zero wrong answers.
- The artifact confirms all 20 Arm B decompositions were faithful native ADJ
  `solve` programs and ADJ returned `solve/solved_roots` for every item,
  including LaTeX-backed cubic constraints.

## [0.23.0] — 2026-06-29

### Added — Gemma quadratic-roots baseline

- Added **`ladder-scorecard.rung3_quadratic_roots.gemma.json`**, a full 20-item
  local Gemma-3-4b trace artifact for native ADJ quadratic root-solving
  questions.
- Recorded the current two-arm baseline: Gemma alone scored 15/20 with 5 wrong
  direct answers, while Gemma + ADJ scored 20/20 with zero wrong answers.
- The artifact confirms all 20 Arm B decompositions were faithful native ADJ
  `solve` programs and ADJ returned `solve/solved_roots` for every item,
  including LaTeX-backed quadratic constraints.

## [0.22.0] — 2026-06-29

### Added — native engine outcome traces

- Program-backed model scorecards now record `arm_b_engine_kind` and
  `arm_b_engine_outcome`, making Arm B misses auditable as native ADJ outcomes
  such as `solve/no_unique_solution` instead of opaque abstentions.
- Tightened the native solve decomposition prompt so multi-variable systems ask
  Gemma to declare every unknown and include every variable in `solve for { ... }`;
  the harness still reads only the requested variable.

## [0.21.0] — 2026-06-29

### Added — Gemma linear-systems baseline

- Added **`ladder-scorecard.rung3_linear_systems.gemma.json`**, a full 20-item
  local Gemma-3-4b trace artifact for native ADJ two-variable linear-system
  solve questions.
- Recorded the current two-arm baseline: Gemma alone scored 2/20 with 18 wrong
  direct answers, while Gemma + ADJ scored 14/20 with zero wrong answers and 6
  abstentions.
- The artifact confirms all 20 Arm B decompositions were faithful native ADJ
  `solve` programs; the 6 bucket-`c` abstentions make the remaining solve-result
  selection gap auditable instead of letting the model fabricate answers.

## [0.20.0] — 2026-06-29

### Added — Gemma constraint-feasibility baseline

- Added **`ladder-scorecard.rung3_constraint_feasibility.gemma.json`**, a full
  20-item local Gemma-3-4b trace artifact for native ADJ constraint-feasibility
  questions.
- Recorded the current two-arm baseline: Gemma alone scored 13/20 with 7 wrong
  direct answers, while Gemma + ADJ scored 20/20 with zero wrong answers.
- The artifact confirms all 20 Arm B decompositions were faithful native ADJ
  `check` programs, giving an inspectable trail where ADJ owns the feasibility
  verdict instead of asking the model to solve constraints directly.

## [0.19.0] — 2026-06-29

### Added — Gemma optimization-witness baseline

- Added **`ladder-scorecard.rung3_optimization_witness.gemma.json`**, a full
  20-item local Gemma-3-4b trace artifact for native ADJ linear optimization
  witness questions.
- Recorded the current two-arm baseline: Gemma alone scored 6/20 with 14 wrong
  direct answers, while Gemma + ADJ scored 20/20 with zero wrong answers.
- The artifact confirms all 20 Arm B decompositions were faithful native ADJ
  optimization programs, giving an inspectable trail where ADJ owns the optimum
  and requested witness assignment.

## [0.18.0] — 2026-06-29

### Added — Gemma derived-probability baseline

- Added **`ladder-scorecard.rung3_derived_probability_decisions.gemma.json`**, a
  full 20-item local Gemma-3-4b trace artifact for the newest multi-step
  derived-evidence probability rung.
- Recorded the current two-arm baseline: Gemma alone scored 18/20 with 2 wrong
  direct answers, while Gemma + ADJ scored 20/20 with zero wrong answers.
- The artifact confirms all 20 Arm B decompositions were faithful native ADJ
  programs, giving an inspectable messy-input → Gemma program → ADJ execution
  trail for this rung.

## [0.17.0] — 2026-06-28

### Added — real Gemma run trace controls

- Model-mode runs now default to a 512-token MLX generation budget and expose
  `--max-tokens`, so Gemma has enough room to emit native multi-line ADJ programs
  on solve, probability, optimization, and constraint rungs.
- Added `--limit` and `--output` for faithful local Gemma smoke runs on newest
  rungs without clobbering committed headline scorecards.
- Model scorecards now include each raw Arm B model response, the extracted
  decomposition, its kind (`formula` or `program`), and its faithfulness verdict,
  making messy-input → Gemma decomposition → ADJ execution failures auditable.

## [0.16.0] — 2026-06-28

### Added — rung 3 derived probability decision scaffold

- **`rung3_derived_probability_decisions/items.json`** — 20 fresh MCQs where the
  gold decomposition observes findings, derives an intermediate evidence atom with
  `rule { ... }`, and lets that derived evidence drive native ADJ likelihood-ratio
  contributions.
- Decision-leader program items can now carry `answer_from.requires` proof checks,
  so the ladder verifies that the winning decision used the requested derived
  evidence proof before mapping `decision.leader` to an option.
- The model-mode decomposition prompt now includes derived-evidence probability
  examples, giving Gemma-style decomposers a native ADJ shape for multi-step
  evidence reasoning without selecting the answer in-model.

## [0.15.0] — 2026-06-28

### Added — rung 3 probability decision scaffold

- **`rung3_probability_decisions/items.json`** — 20 fresh MCQs where the gold
  decomposition is a native ADJ prior / likelihood-ratio / observation / query
  program.
- Program-backed items can now declare `answer_from: {"type": "decision_leader"}`.
  The engine returns `decision.leader`; the harness only maps that leader to the
  printed categorical choices.
- Probability-decision items can opt out of structural-weight stripping in the
  no-result-literals gate, so priors and likelihood ratios must be grounded in the
  question stem instead of treated as hidden scaffold constants.

## [0.14.0] — 2026-06-28

### Added — rung 3 constraint feasibility scaffold

- **`rung3_constraint_feasibility/items.json`** — 20 fresh MCQs where the gold
  decomposition is a native ADJ `check` program over linear constraints.
- Program-backed items can now declare `answer_from: {"type": "check_outcome"}`.
  The engine returns `check.outcome`; the harness only maps `sat` / `sat_real` to
  "feasible" and `unsat` to "infeasible" printed options.
- The model-mode decomposition prompt now includes native `check` examples, so
  local models can emit feasibility programs without deciding feasibility in-model.

## [0.13.0] — 2026-06-28

### Added — rung 3 optimization witness scaffold

- **`rung3_optimization_witness/items.json`** — 20 fresh MCQs where the gold
  decomposition is a native ADJ linear optimization program, but the answer is a
  requested optimal witness assignment rather than the objective value.
- Program-backed items can now declare
  `answer_from: {"type": "optimize_assignment", "name": "<var>"}`. The engine
  returns `optimize.assignments`; the harness only maps the requested witness value
  to the printed options.
- The optimization decomposition prompt now covers both objective-value and
  witness-value items without asking the model to compute either one.

## [0.12.0] — 2026-06-28

### Added — rung 3 linear optimization scaffold

- **`rung3_linear_optimization/items.json`** — 20 fresh MCQs where the gold
  decomposition is a native ADJ `maximize` or `minimize` program over linear
  constraints.
- Program-backed items can now declare `answer_from: {"type": "optimize_value"}`.
  The engine returns `optimize.outcome = optimal`; the harness only compares
  `optimize.value` to the printed options.
- The model-mode decomposition prompt now includes both maximize and minimize
  examples, so local models can emit bounded linear-programming programs without
  computing the optimum in-model.

## [0.11.0] — 2026-06-28

### Added — rung 3 linear systems scaffold

- **`rung3_linear_systems/items.json`** — 20 fresh algebra MCQs where the gold
  decomposition is a native ADJ two-variable linear-system program.
- The rung includes word problems and symbolic systems; several items use native
  `latex "..."` constraints, keeping natural algebra notation on ADJ's execution path.
- The solve-program decomposition prompt now includes ASCII and LaTeX two-variable
  examples, so local models can emit `symbol x`, `symbol y`, multiple constraints,
  and `solve for { x, y }` while the harness maps only the engine-returned assignment.

## [0.10.0] — 2026-06-28

### Added — rung 3 factored roots scaffold

- **`rung3_factored_roots/items.json`** — 20 fresh algebra MCQs where the gold
  decomposition preserves a zero-product equation as a native ADJ solve program.
- Several items use native `latex "..."` constraints, including adjacent factor
  products, so factored LaTeX stays on the ADJ execution path.
- The root-solve decomposition prompt now includes a factored example, giving local
  models a shape that matches common algebra wording without computing roots in-model.

## [0.9.0] — 2026-06-28

### Added — rung 3 quartic roots scaffold

- **`rung3_quartic_roots/items.json`** — 20 fresh algebra MCQs where the gold
  decomposition is a native ADJ quartic root-solving program.
- Several items use native `latex "..."` constraints, so quartic LaTeX input stays
  on the ADJ execution path and returns `solved_roots`.
- The root-solve decomposition prompt now includes a quartic example, giving local
  models a native ADJ shape for degree-4 equations without computing roots in-model.

## [0.8.0] — 2026-06-28

### Added — rung 3 cubic roots scaffold

- **`rung3_cubic_roots/items.json`** — 20 fresh algebra MCQs where the gold
  decomposition is a native ADJ cubic root-solving program.
- Several items use native `latex "..."` constraints, keeping LaTeX math input on
  the ADJ execution path instead of normalizing it in the ladder harness.
- The root-solve decomposition prompt now includes a cubic example so local models
  get a native ADJ shape for degree-3 equations as the ladder climbs.

## [0.7.0] — 2026-06-28

### Added — rung 3 quadratic roots scaffold

- **`rung3_quadratic_roots/items.json`** — 20 fresh algebra MCQs where the gold
  decomposition is a native ADJ root-solving program. Several items use native
  `latex "..."` constraints, so the ladder exercises ADJ's LaTeX math path directly.
- Program-backed items can now declare `answer_from: {"type": "solve_roots"}`. The
  engine returns `solve.outcome = solved_roots`; the harness only compares that root
  set to the printed root-set options.
- The bank-integrity gate now accepts distinct root-set option values while keeping
  formula rungs on numeric options.

## [0.6.0] — 2026-06-28

### Added — mixed derived-premise + solve rung

- **`rung2_derived_solve/items.json`** — 20 fresh repeated-groups MCQs where the
  gold decomposition is a native ADJ program that derives a setup premise with
  `observe` + `rule`, fires a queried `setup_ready` decision through the
  deduction→evidence bridge, then solves the requested unknown with
  `symbol` / `constrain` / `solve for`.
- Program-backed items can now declare optional `answer_from.requires` checks. The
  first requirement type verifies that the CLI produced a determinate decision leader
  and, when an `evidence` atom is named, that the contribution contains an
  `evidence_proof`.
- The no-result-literals gate now treats `prior`/`contributes`/`interacts` leading
  weights as structural ADJ confidence weights for program items while continuing to
  check every observed, constraint, and predicate-threshold number against the stem.

## [0.5.0] — 2026-06-28

### Added — rung 2 native solve-program scaffold

- **`rung2_prealgebra_solve/items.json`** — 20 fresh, self-authored pre-algebra MCQs
  where the gold decomposition is a native ADJ program (`symbol` / `constrain` /
  `solve for`) instead of a single arithmetic expression.
- `ladder_eval.py` can now run program-backed items, read an ADJ solver assignment,
  and map the engine-computed value to the printed options. Python still never solves
  the equation; it only performs option lookup against the engine's answer.
- The model-mode decomposition prompt can ask for a native ADJ solve program, and the
  same no-result-literals gate rejects model programs whose numeric literals do not
  appear in the stem.
- The contamination gate now understands formula-backed and program-backed rungs. When
  `adj-lang-cli` is built, it validates a program-backed gold key by running the native
  ADJ program and checking that the solved value maps to `gold_letter`.

## [0.4.0] — 2026-06-28

### Added — PR-1 deduction to evidence bridge

- `logic-engine` now lets a rule-derived atom gate an LR contribution. This is the
  first multi-step reasoning bridge the ladder needs: a small model can emit
  observations and rules, and ADJ can prove the intermediate premise before weighing
  it probabilistically.
- Derived evidence carries its SLD proof into the LR proof DAG and CLI JSON. The
  audit trail can now show both the deduction that established a premise and the
  likelihood-ratio step that used it.
- Probabilistic proof chains attenuate the applied LR delta by their fact/rule
  confidence; all-certain chains keep the old exact behavior.

## [0.3.0] — 2026-06-27

### Added — rung 1 fractions/percent scaffold

- **`rung1_fractions_percent/items.json`** — 20 fresh, self-authored MCQs covering
  fraction-of quantities, terminating fraction arithmetic, percent, ratios, and unit
  rates. The bank deliberately uses terminating fractions and integer percent results
  so today's ADJ numeric path can verify the cached engine gate before exact-rational
  engine work expands the rung.
- **Rung-generic integrity and cached-engine tests.** The contamination gate and
  cached Arm B end-to-end test now run against both self-contained starter rungs.
- Docs now make the split explicit: this PR grows the ladder one small rung; arbitrary
  fraction equality remains part of the exact-rational ADJ-REASON-MATH work.

## [0.2.0] — 2026-06-26

### Added — Gemma as the canonical local base target + first real two-arm number

- **Gemma base target.** `--model gemma` / `--model gemma-1b` aliases load the cached
  `mlx-community/gemma-3-{4b,1b}-it-bf16` instruct checkpoints via MLX — a small,
  non-frontier, **fully-local** model (no API, offline). MLX loading now applies the
  tokenizer chat template and greedy sampling (`temp=0`) for reproducible runs.
- **First real two-arm result (rung 0, Gemma-3-4b, greedy):** Arm A (model alone)
  **60%** (12/20, **8 wrong**); Arm B (model+ADJ) **95%** (19/20, **0 wrong**,
  defensibility **1.00**); **divergence +35% (+7 items)**. Arm B's single miss is a
  decompose error the engine caught and abstained on — zero fabrications.
  Artifact: `ladder-scorecard.gemma.json`.
- **Formula extraction stays strict.** A few-shot decompose prompt steers the model
  to either plain ASCII arithmetic or ADJ's native LaTeX wrapper; `extract_formula`
  accepts only a plain `+ - * / ()` line or native ADJ `latex "..."` expression
  (stripping an echoed `Formula:` label) and **abstains** on anything else. Bare
  LaTeX/unicode math is deliberately NOT normalized in the harness — the model must
  emit ADJ syntax, and adj-lang owns parsing/solving.
- **Per-model scorecards.** Model runs write `ladder-scorecard.<model>.json`; cached
  runs write `ladder-scorecard.json` — a cached CI run never clobbers a committed
  two-arm headline. Scorecard summary now records the `model`.
- Tests: 24 total, including native ADJ LaTeX extraction and a harness-to-engine
  `latex "$5 \times 12$"` smoke.

## [0.1.0] — 2026-06-26

### Added — PR-0: the two-arm instrument + rung 0 (no engine change)

- **`ladder_eval.py`** — the two-arm scorer.
  - Arm B builds an option-selection ADJ program (`let answer = <formula>` +
    `contributes 1000000 from answer == <option> to opt_X` per option), runs the
    native `adj-lang-cli`, and maps the `decision` back to a letter
    (`determinate`→leader letter, `kickback`/empty→ABSTAIN). The engine does all
    arithmetic; the harness never computes an answer.
  - Arm A prompts the model for a letter directly (model mode only).
  - Three-outcome scoring (correct / abstained / wrong) reused from `board_eval.py`,
    plus per-arm `raw_accuracy` / `defensibility` / `accuracy_on_attempted`, the
    cross-arm **divergence** (B − A), and per-item **failure buckets** (b
    decompose-error / c engine-gap).
  - Modes: `--mode cached` (default; Arm B engine-only, the CI path) and
    `--model mlx:<repo>` / `--model cmd:<shell>` (both arms with a local model).
  - **no-result-literals** gate: a model-produced formula whose numbers aren't all in
    the stem is rejected (abstain, bucket b) — the model may write the recipe, never
    the answer.
  - CLI discovery via standard target paths + `ADJ_LANG_CLI` override.
  - **Gate:** cached mode exits non-zero if the engine ever miscomputes (`wrong > 0`)
    or the CLI is missing.
- **`rung0_arithmetic/items.json`** — 20 self-authored, contamination-free
  grade-school arithmetic and one-step word-problem MCQs (fresh numbers; gold formula
  per item with all literals traceable to the stem).
- **`contamination_check.py`** — bank-integrity / anti-circularity gate: unique ids,
  five distinct option values, `gold_letter ∈ options`, gold-key correctness via a
  *restricted safe* arithmetic eval, no-result-literals on the gold formula, and
  self-containment (no external source/import at rung 0).
- **`test_ladder_eval.py`** — 18 tests: program building, faithfulness gate,
  decision→letter, letter/formula parsing, scoring & divergence math, bank integrity,
  safe-eval sandbox, and an end-to-end cached run asserting the engine selects every
  gold option (skips if the CLI isn't built).
- **Specs of record** committed alongside: `code/specs/ADJ-LADDER.md` (this campaign),
  `code/specs/ADJ-REASON-MATH.md` (engine evolution), `code/specs/MLE-PASS.md`
  (clinical rung harness).

### Result

Rung-0 cached run: Arm B **20/20 correct, 0 wrong, 0 abstain** — the engine computed
every answer exactly and selected the gold option. The mechanism is proven with zero
engine change; the ladder is ready to climb.
