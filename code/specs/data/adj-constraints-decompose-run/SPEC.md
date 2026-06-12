# ADJ constraints — live decompose→solve demonstration (track D2)

## The claim

> A **small local model** can do the one job it is good at — turning messy prose
> into a typed, checkable representation — and a **deterministic CPU engine** does
> all the reasoning. The model **decomposes**; it never computes the answer. Every
> answer is produced by `adj-lang-cli` at **zero answer-time model calls**, and is
> cited back to the constraint that determined it.

This is the constraint-solving analogue of the MYCIN derive-once demonstration:
the intelligence lives in the *framework* (the grammar that constrains the
decomposition + the exact solver), not in the weights.

## What this is NOT

- It is **not** a closed-book accuracy benchmark. The model is asked only to
  *transcribe* a word problem into adj-lang; it is not asked to solve it.
- It is **not** a CI test. A live model is non-deterministic and Ollama is not
  available in CI. The reproducible part — that the **engine** solves the
  committed `.adj` programs correctly — is covered by a deterministic golden test
  ([`adj-lang-cli/tests/decompose_run.rs`](../../../packages/rust/adj-lang-cli/tests/decompose_run.rs)).

## Protocol

```
messy prose word problem  (cases/cases.json, with a GOLD answer)
        │  decompose.py  — ONE model call per case (the only model touchpoint)
        │     prompt = the adj-lang constraint grammar + vocabulary rules + the prose
        │     the model emits ONLY an .adj program (symbols/constrain/solve|check|min|max)
        ▼  results/<id>.adj           ← the model's decomposition (committed)
        │  adj-lang-cli results/<id>.adj      — the CPU engine SOLVES it
        ▼  results/<id>.json          ← the engine's decision + provenance (committed)
        │  score.py — compare the engine's value to the case GOLD
        ▼  results/summary.json       ← per-case pass/fail + the call accounting
   answer_time_model_calls == 0  (asserted)
```

- **Model:** a local Ollama model (default `llama3.1:8b`), temperature 0, called
  via the zero-dependency `/api/generate` endpoint (urllib only — no SDK).
- **Decompose-only discipline:** the prompt forbids the model from stating an
  answer; it must emit only the grammar. If the emitted text fails to compile or
  uses a non-grammar construct, that is a *decomposition* failure (a framework
  signal), not a reasoning error — the engine never produces a wrong number.
- **The engine is the reasoner:** the GOLD answer is checked against
  `adj-lang-cli`'s output, never against anything the model said.

## Cases (`cases/cases.json`)

Each exercises a different solver path, with a hand-computed GOLD answer:

| id | prose gist | solver path | GOLD |
|---|---|---|---|
| `relief_allocation` | split a $12k budget across meals/shelter to maximize relief, meals capped at $8k | `maximize` (LP) | 44 at (meals=8, shelter=4) |
| `workshop_break_even` | $2000 fixed + $15/attendee, $65/ticket — break-even headcount | `solve for` (linear eq) | 40 |
| `schedule_feasibility` | design finishes ≥ day 28, build ≥ design+20 and ≤ day 45 — feasible? | `check` (feasibility) | UNSAT (contradiction) |
| `production_cost` | make ≥100 of A ($5) and ≥60 of B ($8), minimize cost | `minimize` (LP) | 980 |

## Files

- `cases/cases.json` — the prose + gold answers + which engine key carries the answer.
- `decompose.py` — the model touchpoint (urllib → Ollama, decompose-only prompt).
- `run.py` — orchestrate decompose → `adj-lang-cli` → score; write `results/`.
- `results/` — the committed real run: `<id>.adj` (model output), `<id>.json`
  (engine output), `summary.json` (scores + call accounting).
- `FINDINGS.md` — what the run showed (honest about decomposition failures).
- `adj-lang-cli/tests/decompose_run.rs` — deterministic golden test: the engine
  solves every committed `.adj` to its gold value (no model involved).

## Reproduce

```
ollama pull llama3.1:8b          # or any local model
cargo build -p adj-lang-cli
python3 run.py                   # re-runs the model; overwrites results/
cargo test -p adj-lang-cli --test decompose_run   # deterministic, no model
```

## Honest limitations

- One model, one prompt, a handful of cases — a *demonstration*, not a powered
  study. The point is the **architecture** (decompose-only + exact solver = 0
  answer-time calls), not a model-quality number.
- A small model sometimes mis-transcribes a problem (wrong coefficient, a missing
  constraint). That surfaces as a *failed decomposition* the engine reports
  honestly (wrong value vs gold, or a compile error) — never a confidently-wrong
  answer the engine invented. The committed `results/` show exactly which cases
  the model got right on the recorded run.
