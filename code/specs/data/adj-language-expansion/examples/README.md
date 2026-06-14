# Worked adjudication examples (ADJ constraints track D)

Runnable `.adj` programs that exercise the **whole** adj-lang stack end-to-end —
typed values, strict dimensions, `let` arithmetic, predicate-gated verdicts, and
constraint solving — **at zero answer-time model calls**. The model's only job
is to decompose the messy input into these clauses; the engine does all the
math and solving on the CPU, and every answer is cited back to its source.

Run any of them through the CPU reasoner:

```
cargo build -p adj-lang-cli
./target/debug/adj-lang-cli code/specs/data/adj-language-expansion/examples/eligibility.adj
```

| file | demonstrates | expected |
|---|---|---|
| [`eligibility.adj`](eligibility.adj) | a **predicate over a typed value** (`gross_income >= 14600` over `money(18000, usd)`) — a deterministic rule as a saturating LR | `required_to_file` ≈ 1.0, determinate; proof shows the `>= 14600 / observed 18000` comparison + IRS citation |
| [`debt_to_income.adj`](debt_to_income.adj) | a **computed ratio** (`money / money → dimensionless`) driving a rule | `dti = 0.30 <= 0.43` fires → `mortgage_eligible` ≈ 1.0 |
| [`proration.adj`](proration.adj) | **`let` arithmetic** (`annual_bonus * months_worked / 12`) feeding a predicate | `prorated = 9000 >= 8000` fires → `senior_tier` ≈ 1.0 |
| [`break_even.adj`](break_even.adj) | **solving for an unknown** (`p * 1000 = 5000 + 3 * 1000`) | `solve → { p = 8 }`, cited to constraint `[0]` |
| [`grant_allocation.adj`](grant_allocation.adj) | **linear optimization** — `maximize 3·outreach + 2·training` under a budget + capacity (an LP) | `optimize → optimal 26` at `{ outreach = 6, training = 4 }`, binding constraints `[0, 1]` |

These are covered by golden tests in
[`adj-lang-cli/tests/worked_examples.rs`](../../../../packages/rust/adj-lang-cli/tests/worked_examples.rs),
so they stay runnable as the language evolves.

See [`ADJ-CONSTRAINTS-DESIGN.md`](../ADJ-CONSTRAINTS-DESIGN.md) for the arc and
[`DESIGN.md`](../DESIGN.md) for the computation layer these build on.
