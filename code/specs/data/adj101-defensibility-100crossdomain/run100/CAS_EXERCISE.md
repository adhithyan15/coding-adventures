# ADJ101 — exercising the CAS: rulebook → program library → linked into the input-as-program

Concrete demonstration of the derive-once / reuse-indefinitely loop (paper-2 MYCIN), built on a
**verified rulebook from the 100-run** (TAX-1, the federal-filing-threshold rule). `cas_exercise.py`.

## The three steps

**1. CAS WRITE.** The TAX-1 rulebook (2 rules) is committed only after the **byte-accounting gate**:
every rule's `source_span` is verified verbatim in the policy (the invariant the 100-run held 100/100).
It is then canonicalized and **content-addressed** → object `ff22eace640aba13` under `cas/objects/`.

**2. COMPILE.** The cached rulebook is compiled into a **self-contained program library**
`cas/lib/rulelib_ff22eace640aba13.py` — the rules baked in as data + a vendored deterministic evaluator
+ `decide(facts) -> {verdict, answer, proof}`. **No model, no repo imports.** Pure CPU.

**3. LINK + RUN.** A held-out input is decomposed into typed-IR facts; that IR is emitted **as a small
program** that `import`s the compiled library and calls `decide(facts)`. Three held-out cases:

| input (IR → program) | facts | verdict | proof |
|---|---|---|---|
| TAX-1 (original) | `gross_income=18000` | **REQUIRED_TO_FILE** | rule `must-file` fired on income=18000, cites policy bytes |
| held-out A | `gross_income=9500` | **NOT_REQUIRED_TO_FILE** | rule `no-return-below-threshold` fired |
| held-out B (threshold) | `gross_income=14600` | **REQUIRED_TO_FILE** | rule `must-file` fired (`>= 14600`) |

**Answer-time model calls = 0.** The rulebook was derived + verified **once**; every subsequent case —
including ones never seen at derivation time — is decided by executing the linked library on CPU, and
**every verdict carries a proof tracing rule → fact → cited policy bytes.**

## Why this matters (the thesis, operationalized)
- **Derive once, reuse indefinitely:** the expensive, model-driven step (derive + byte-verify the
  rulebook) is paid once; reuse is CPU-bound and free of the model.
- **Reasoning is CPU-bound:** the LLM was the *translator* (policy → rules, input → facts); the
  *reasoner* is the compiled library — deterministic, reproducible, same input → same proof byte-for-byte.
- **Trust-free reuse rests on the write-time gate:** the library is trustworthy because what entered the
  CAS was byte-verified. The full gate ([`FORWARD-DESIGN.md`](FORWARD-DESIGN.md)) adds N adversarial
  readers × byte-stability × blind-judge concurrence before commit; here we exercised the byte-accounting
  leg the 100-run already validated.
- **Everything is a program:** the input (typed IR → program) and the rulebook (CAS → program library)
  are both programs; deciding a case is **linking** them and running — the unification the design targets.

## Reproduce
`python3 cas_exercise.py` → writes `cas/objects/<hash>.json`, `cas/lib/rulelib_<hash>.py`,
`cas/programs/input_*.py`, and `cas_exercise_results.json`.
