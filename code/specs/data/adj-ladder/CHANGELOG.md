# Changelog — adj-ladder

All notable changes to the ADJ-LADDER two-arm reasoning scoreboard.

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
