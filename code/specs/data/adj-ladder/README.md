# ADJ-LADDER — two-arm reasoning scoreboard

A graduated benchmark that proves *reasoning and math live in the framework, not the
weights*. At every rung (grade-school arithmetic → … → medical licensing exam) the
same question set runs through two arms:

- **Arm A** — the small model **alone** (it does the math in its head).
- **Arm B** — the small model **+ the ADJ engine** (the model only *decomposes*; the
  engine does every bit of arithmetic on the CPU, exactly, and emits a checkable proof).

The headline is the **divergence B − A**, which widens as complexity rises. See the
spec of record: [`code/specs/ADJ-LADDER.md`](../../ADJ-LADDER.md) (and its siblings
[`ADJ-REASON-MATH.md`](../../ADJ-REASON-MATH.md), [`MLE-PASS.md`](../../MLE-PASS.md)).

The **canonical base target is Gemma** — a small, non-frontier model that runs
**fully locally** (no API, offline). `--model gemma` loads the cached
`mlx-community/gemma-3-4b-it-bf16` via MLX; `--model gemma-1b` the 1B variant.

### First real two-arm number (rung 0, Gemma-3-4b, greedy)

| Arm | raw accuracy | wrong (fabrications) | defensibility |
|-----|--------------|----------------------|---------------|
| **A** — Gemma alone | **60%** (12/20) | **8** | 0.60 |
| **B** — Gemma + ADJ | **95%** (19/20) | **0** | **1.00** |

**Divergence B − A = +35% (+7 items).** Even at grade-school arithmetic a small local
model fabricates 8 wrong answers; the engine arm makes **zero** — its one miss is a
*decompose* error (bucket `b`) the engine caught and **abstained** on. The gap is
expected to widen as the ladder climbs. (Artifact: `ladder-scorecard.gemma.json`.)

### Current multi-step baseline (rung 3 derived probability, Gemma-3-4b, greedy)

| Arm | raw accuracy | wrong (fabrications) | defensibility |
|-----|--------------|----------------------|---------------|
| **A** — Gemma alone | **90%** (18/20) | **2** | 0.90 |
| **B** — Gemma + ADJ | **100%** (20/20) | **0** | **1.00** |

**Divergence B − A = +10% (+2 items).** On the newest multi-step probability rung,
Gemma emitted faithful native ADJ derived-evidence programs for all 20 items; ADJ
then proved the derived evidence, applied the likelihood-ratio contribution, and
selected the gold decision leader. The committed trace artifact records the raw
Gemma output and extracted ADJ program per item:
`ladder-scorecard.rung3_derived_probability_decisions.gemma.json`.

### Current optimization baseline (rung 3 optimization witness, Gemma-3-4b, greedy)

| Arm | raw accuracy | wrong (fabrications) | defensibility |
|-----|--------------|----------------------|---------------|
| **A** — Gemma alone | **30%** (6/20) | **14** | 0.30 |
| **B** — Gemma + ADJ | **100%** (20/20) | **0** | **1.00** |

**Divergence B − A = +70% (+14 items).** On the optimization-witness rung, Gemma
successfully translated every messy prompt into a faithful native ADJ optimization
program, while the direct-answer arm fabricated most requested witness values. ADJ
owned the linear optimization and returned the requested optimal assignment. Artifact:
`ladder-scorecard.rung3_optimization_witness.gemma.json`.

### Current constraint baseline (rung 3 constraint feasibility, Gemma-3-4b, greedy)

| Arm | raw accuracy | wrong (fabrications) | defensibility |
|-----|--------------|----------------------|---------------|
| **A** — Gemma alone | **65%** (13/20) | **7** | 0.65 |
| **B** — Gemma + ADJ | **100%** (20/20) | **0** | **1.00** |

**Divergence B − A = +35% (+7 items).** On the constraint-feasibility rung,
Gemma emitted faithful native ADJ `check` programs for all 20 messy prompts; ADJ
owned the feasibility verdict and mapped it back to the printed options. Artifact:
`ladder-scorecard.rung3_constraint_feasibility.gemma.json`.

### Current equation-solving baseline (rung 3 linear systems, Gemma-3-4b, greedy)

| Arm | raw accuracy | wrong (fabrications) | defensibility |
|-----|--------------|----------------------|---------------|
| **A** — Gemma alone | **10%** (2/20) | **18** | 0.10 |
| **B** — Gemma + ADJ | **70%** (14/20) | **0** | **1.00** |

**Divergence B − A = +60% (+12 items).** On the two-variable linear-system rung,
Gemma emitted faithful native ADJ `solve` programs for all 20 messy prompts. ADJ
returned 14 correct answers with zero wrong selections; the remaining 6 faithful
programs abstained as bucket `c`, exposing the next solve-result selection gap rather
than hiding it as a model answer. Artifact:
`ladder-scorecard.rung3_linear_systems.gemma.json`.

### Current algebra baseline (rung 3 quadratic roots, Gemma-3-4b, greedy)

| Arm | raw accuracy | wrong (fabrications) | defensibility |
|-----|--------------|----------------------|---------------|
| **A** — Gemma alone | **75%** (15/20) | **5** | 0.75 |
| **B** — Gemma + ADJ | **100%** (20/20) | **0** | **1.00** |

**Divergence B − A = +25% (+5 items).** On the first nonlinear algebra rung,
Gemma emitted faithful native ADJ `solve` programs for all 20 prompts. ADJ owned
the quadratic root computation and returned `solve/solved_roots` for every item,
including the LaTeX-backed constraints. Artifact:
`ladder-scorecard.rung3_quadratic_roots.gemma.json`.

### Current algebra baseline (rung 3 cubic roots, Gemma-3-4b, greedy)

| Arm | raw accuracy | wrong (fabrications) | defensibility |
|-----|--------------|----------------------|---------------|
| **A** — Gemma alone | **80%** (16/20) | **4** | 0.80 |
| **B** — Gemma + ADJ | **100%** (20/20) | **0** | **1.00** |

**Divergence B − A = +20% (+4 items).** On the expanded cubic algebra rung,
Gemma emitted faithful native ADJ `solve` programs for all 20 prompts. ADJ owned
the cubic root computation and returned `solve/solved_roots` for every item,
including the LaTeX-backed constraints. Artifact:
`ladder-scorecard.rung3_cubic_roots.gemma.json`.

## Layout

```
adj-ladder/
  ladder_eval.py            two-arm scorer (cached engine-only, or --model both arms)
  contamination_check.py    bank-integrity / anti-circularity gate
  test_ladder_eval.py       unit + cached end-to-end tests
  rung0_arithmetic/
    items.json              20 fresh grade-school MCQs {id,stem,formula,options,gold_letter}
  rung1_fractions_percent/
    items.json              20 fresh fractions/percent/ratio MCQs, self-contained
  rung2_prealgebra_solve/
    items.json              20 pre-algebra MCQs backed by native ADJ solve programs
  rung2_derived_solve/
    items.json              20 MCQs requiring a derived setup premise + native solve
  rung3_linear_systems/
    items.json              20 two-variable linear-system MCQs backed by native ADJ solve
  rung3_constraint_feasibility/
    items.json              20 feasibility MCQs backed by native ADJ check
  rung3_probability_decisions/
    items.json              20 diagnosis/decision MCQs backed by native ADJ priors/LRs
  rung3_derived_probability_decisions/
    items.json              20 MCQs combining native ADJ rules with priors/LRs
  rung3_linear_optimization/
    items.json              20 linear-optimization MCQs backed by native ADJ optimize
  rung3_optimization_witness/
    items.json              20 optimization-witness MCQs backed by native ADJ optimize
  rung3_quadratic_roots/
    items.json              20 algebra MCQs backed by native ADJ solved_roots
  rung3_cubic_roots/
    items.json              20 cubic algebra MCQs backed by native ADJ solved_roots
  rung3_quartic_roots/
    items.json              20 quartic algebra MCQs backed by native ADJ solved_roots
  rung3_factored_roots/
    items.json              20 factored-polynomial MCQs backed by native ADJ solved_roots
  ladder-scorecard.json     emitted artifact (per-arm metrics + divergence + buckets)
  ladder-scorecard.rung3_derived_probability_decisions.gemma.json
                            full local Gemma trace artifact for newest multi-step rung
  ladder-scorecard.rung3_optimization_witness.gemma.json
                            full local Gemma trace artifact for optimization witness rung
  ladder-scorecard.rung3_constraint_feasibility.gemma.json
                            full local Gemma trace artifact for constraint feasibility
  ladder-scorecard.rung3_linear_systems.gemma.json
                            full local Gemma trace artifact for linear systems
  ladder-scorecard.rung3_quadratic_roots.gemma.json
                            full local Gemma trace artifact for quadratic roots
  ladder-scorecard.rung3_cubic_roots.gemma.json
                            full local Gemma trace artifact for cubic roots
```

## How Arm B answers without computing the answer itself

For options `{A:59,…}` and gold formula `7 * 8 + 3`, the harness emits an ADJ program
with one equal-prior hypothesis per option and one predicate that fires when the
engine-computed `answer` equals that option's value:

```adj
let answer = 7 * 8 + 3
contributes 1000000 from answer == 59 to opt_a
…
? opt_a … ? opt_e
```

The formula can be plain ADJ arithmetic or native ADJ LaTeX syntax such as
`latex "$5 \times 12$"`; option values can be numeric literals or ADJ expressions
such as `3 / 10`. Either way, `adj-lang-cli` owns parsing and execution. The engine
computes `answer`, compares it against each option expression, the matching predicate
fires, and the decision returns `determinate` with `leader = opt_a` → **A**. No match
(or a tie) → `kickback` → **abstain**. The harness supplies only the formula and the
printed option values; the arithmetic and the selection are the engine's.

## Run it

```bash
# 1. build the engine
cargo build -p adj-lang-cli          # from code/packages/rust/

# 2. bank integrity (off the answer path)
python3 contamination_check.py rung0_arithmetic
python3 contamination_check.py rung1_fractions_percent
python3 contamination_check.py rung2_prealgebra_solve
python3 contamination_check.py rung2_derived_solve
python3 contamination_check.py rung3_linear_systems
python3 contamination_check.py rung3_constraint_feasibility
python3 contamination_check.py rung3_probability_decisions
python3 contamination_check.py rung3_derived_probability_decisions
python3 contamination_check.py rung3_linear_optimization
python3 contamination_check.py rung3_optimization_witness
python3 contamination_check.py rung3_quadratic_roots
python3 contamination_check.py rung3_cubic_roots
python3 contamination_check.py rung3_quartic_roots
python3 contamination_check.py rung3_factored_roots

# 3. engine-only (cached) run — expect Arm B 100%, wrong 0
python3 ladder_eval.py rung0_arithmetic
python3 ladder_eval.py rung1_fractions_percent
python3 ladder_eval.py rung2_prealgebra_solve
python3 ladder_eval.py rung2_derived_solve
python3 ladder_eval.py rung3_linear_systems
python3 ladder_eval.py rung3_constraint_feasibility
python3 ladder_eval.py rung3_probability_decisions
python3 ladder_eval.py rung3_derived_probability_decisions
python3 ladder_eval.py rung3_linear_optimization
python3 ladder_eval.py rung3_optimization_witness
python3 ladder_eval.py rung3_quadratic_roots
python3 ladder_eval.py rung3_cubic_roots
python3 ladder_eval.py rung3_quartic_roots
python3 ladder_eval.py rung3_factored_roots

# 4. tests
python3 -m pytest test_ladder_eval.py -q

# 5. two-arm run with the local Gemma base target (needs mlx-lm; cache-only load)
pip install mlx-lm                                   # one-time, into your run env
# Use the Python that can `import mlx_lm` (for example, the repo's mise Python).
HF_HUB_OFFLINE=1 python3 ladder_eval.py rung0_arithmetic --model gemma      # 4B
HF_HUB_OFFLINE=1 python3 ladder_eval.py rung0_arithmetic --model gemma-1b   # 1B
# newest-rung smoke runs: enough room for multi-line ADJ programs, without
# overwriting the committed headline scorecard
HF_HUB_OFFLINE=1 python3 ladder_eval.py rung3_probability_decisions \
  --model gemma --max-tokens 512 --limit 3 --output /tmp/gemma-probability-smoke.json
HF_HUB_OFFLINE=1 python3 ladder_eval.py rung3_derived_probability_decisions \
  --model gemma --max-tokens 512 --limit 3 --output /tmp/gemma-derived-probability-smoke.json
# any other local model works too:
python3 ladder_eval.py rung0_arithmetic --model mlx:<hf-repo>
python3 ladder_eval.py rung0_arithmetic --model 'cmd:ollama run <model>'
```

`--model gemma` writes its scorecard to `ladder-scorecard.gemma.json` (per-model files,
so a cached CI run never clobbers a committed two-arm headline). Model scorecards
also record the raw Arm B model output, the extracted ADJ decomposition, its kind
(`formula` or `program`), and whether it passed the no-result-literals faithfulness
gate. Program-backed model scorecards also record the native ADJ engine family and
outcome (`arm_b_engine_kind` / `arm_b_engine_outcome`), so misses remain auditable as
e.g. `solve/no_unique_solution` rather than opaque abstentions. Those trace fields are
the audit trail for the real north-star run: messy human input → local Gemma
decomposition → native ADJ execution.

If the `adj-lang-cli` binary lives somewhere non-standard, point `ADJ_LANG_CLI` at it.

## Adding a rung

Drop a new `rungN_<name>/items.json` with the same schema and a mini standard library
the engine imports; reuse `ladder_eval.py` unchanged. Each rung pulls in the next
engine capability from ADJ-REASON-MATH (exact rationals, CAS wiring, dimensional
units, the deduction↔evidence bridge) — see ADJ-LADDER.md §5.

`rung1_fractions_percent` is intentionally a starter scaffold. It now has the native
surface it needs for fractional option matching (`answer == 3 / 10`), with exact
rational sidecars carrying integer/rational arithmetic through predicate equality.
The next climb connects multi-step deductions into probabilistic evidence: ADJ can
derive an intermediate premise with `rule { ... }`, then let that derived atom fire a
`contributes ... from <premise>` clause with the SLD proof attached to the audit trail.
`rung2_prealgebra_solve` starts the next step upward: the gold decomposition is now a
native ADJ program (`symbol` / `constrain` / `solve for`) rather than a single
arithmetic expression, and the harness maps the engine's solved variable to the
printed options without solving the equation in Python.
`rung2_derived_solve` mixes the two paths: the program first derives a setup premise
with `observe` + `rule`, uses that derived atom to fire a queried `setup_ready`
decision with an SLD `evidence_proof`, then solves the requested unknown from the
observed quantities.
`rung3_linear_systems` takes the next native solve step: ADJ solves two-variable
linear systems, including native LaTeX constraints, and the ladder maps one requested
engine assignment to the printed options without solving the system in Python.
`rung3_constraint_feasibility` adds native feasibility checks: local models emit
`symbol` / `constrain` / `check` programs, ADJ returns `check.outcome`, and the
ladder maps that categorical verdict to printed feasibility options.
`rung3_probability_decisions` returns to the native evidence calculus directly:
local models emit `prior` / `contributes` / `observe` / `?` programs, ADJ computes
the posterior ranking, and the ladder maps only `decision.leader` to the printed
diagnosis or decision options.
`rung3_derived_probability_decisions` combines that probability path with the
deduction bridge: local models emit observations plus a `rule`, ADJ proves the
derived evidence atom, uses it to license an LR contribution, and the ladder requires
that proof before mapping `decision.leader`.
`rung3_linear_optimization` adds the first optimization rung: local models emit
native ADJ `maximize`/`minimize` programs, ADJ returns `optimize.value`, and the
ladder maps only that engine optimum to the printed options.
`rung3_optimization_witness` keeps the same native optimizer path but asks for a
specific optimal assignment from `optimize.assignments`, so ADJ selects the planning
choice as well as the objective value.
`rung3_quadratic_roots` starts the algebra rung: ADJ solves `x^2 = n` programs,
including native LaTeX constraints, and the harness maps the engine's returned
`solved_roots` set to the printed root-set options without computing the roots.
`rung3_cubic_roots` takes the next small algebra step: ADJ solves expanded cubic
polynomial equations, including native LaTeX constraints, through the same
`solved_roots` option-mapping path.
`rung3_quartic_roots` finishes the current closed-form polynomial root scaffold:
ADJ solves expanded quartic equations, including native LaTeX constraints, and the
ladder still maps only the engine-returned real root set to the printed options.
`rung3_factored_roots` keeps the same solver boundary but switches the decomposition
shape to zero-product equations like `(x - 2)(x - 5) = 0`, so local models can emit
the natural factored program and let ADJ expand and solve it.
