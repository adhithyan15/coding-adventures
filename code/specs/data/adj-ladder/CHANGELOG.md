# Changelog — adj-ladder

All notable changes to the ADJ-LADDER two-arm reasoning scoreboard.

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
