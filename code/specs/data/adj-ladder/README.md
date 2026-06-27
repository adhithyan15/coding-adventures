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

## Layout

```
adj-ladder/
  ladder_eval.py            two-arm scorer (cached engine-only, or --model both arms)
  contamination_check.py    bank-integrity / anti-circularity gate
  test_ladder_eval.py       unit + cached end-to-end tests
  rung0_arithmetic/
    items.json              20 fresh grade-school MCQs {id,stem,formula,options,gold_letter}
  ladder-scorecard.json     emitted artifact (per-arm metrics + divergence + buckets)
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

The engine computes `answer`, the matching predicate fires, and the decision returns
`determinate` with `leader = opt_a` → **A**. No match (or a tie) → `kickback` →
**abstain**. The harness supplies only the formula and the printed option values; the
arithmetic and the selection are the engine's.

## Run it

```bash
# 1. build the engine
cargo build -p adj-lang-cli          # from code/packages/rust/

# 2. bank integrity (off the answer path)
python3 contamination_check.py rung0_arithmetic

# 3. engine-only (cached) run — expect Arm B 100%, wrong 0
python3 ladder_eval.py rung0_arithmetic

# 4. tests
python3 -m pytest test_ladder_eval.py -q

# 5. two-arm run with a local model (Apple-silicon MLX, or any cmd: wrapper)
python3 ladder_eval.py rung0_arithmetic --model mlx:mlx-community/Qwen2.5-0.5B-Instruct-4bit
python3 ladder_eval.py rung0_arithmetic --model 'cmd:my-local-llm --prompt-stdin'
```

If the `adj-lang-cli` binary lives somewhere non-standard, point `ADJ_LANG_CLI` at it.

## Adding a rung

Drop a new `rungN_<name>/items.json` with the same schema and a mini standard library
the engine imports; reuse `ladder_eval.py` unchanged. Each rung pulls in the next
engine capability from ADJ-REASON-MATH (exact rationals, CAS wiring, dimensional
units, the deduction↔evidence bridge) — see ADJ-LADDER.md §5.
