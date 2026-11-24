# coding-adventures-test-bootstrap

Zero-dependency Python test foundation providing deterministic PRNG (`SplitMix64`), property-based testing with shrinking, coverage-guided fuzz harness, and structured TFV/evidence summaries compatible with VER00/VER13 verified shipping policies.

## Features

- **SplitMix64 PRNG**: Exact 64-bit deterministic pseudorandom generator, matching reference vectors (`seed 0 -> 0xe220a8397b1dcdaf`), unbiased rejection sampling in `bounded(n)`, and FNV-1a 64-bit hashing.
- **Property-Based Testing** (VER05 / VER05c): a `Gen` generator algebra (`gen_int`, `gen_bytes`, `gen_list`, `gen_tuple`, `frequency`, `map`, `filter`, `flat_map`, ...) with *integrated* shrinking, so a shrunk `gen_int(5, 10)` value never leaves `[5, 10]`; conforming shrinkers returning `(minimal, steps)`; an honest runner (`check_property`) and the convenience runner `for_all`. Conforms to the shared `pbt-v1` corpus (see below).
- **Fuzz Harness**: Deterministic mutation engine (`fuzz_bytes`, `fuzz_text`) supporting bit/byte flipping, insertions, deletions, replacements, and crossover with crash deduplication.
- **Structured Test Evidence & TFV**: In-memory TFV (TF01 §2 format) parser and serializer, along with deterministic JSON/TFV assurance report emission (VER00/VER13).
- **CWE-400 Hardening**: Strict upper bounds on input sizes (`MAX_INPUT_BYTES`), case counts (`MAX_CASES`), shrink iterations (`MAX_SHRINK_STEPS`), and fuzz iterations (`MAX_ITERATIONS`), with strict input validation on all entrypoints.

## Property testing

```python
from coding_adventures_test_bootstrap import (
    PbtOptions, check_property, for_all, gen_int, gen_list, ci_from_environ,
)
import os

# Convenience runner: returns the raw counterexample, shrunk inside the bounds.
result = for_all(lambda x: x < 10, gen_int(0, 100), count=200, seed=7)
assert not result.passed and result.shrunken == (10,)

# Honest runner: closed failure ids, re-verified witness, CI redaction.
verdict = check_property(
    42,
    gen_list(gen_int(0, 9), 0, 8),
    lambda xs: sum(xs) <= 20,
    PbtOptions(num_tests=200, ci=ci_from_environ(os.environ)),
)
print(verdict.failure_id, verdict.message)
```

A property **fails** when it returns exactly `False`; returning `None` (assertion
style) passes, and an exception is `property-panic`. A run that evaluates zero
cases is `property-exhausted`, never a pass.

`PbtOptions.ci` defaults to `None`, which *redacts* (no witness, no seed in the
message; only the iteration, step count and a `repro-fnv` digest). The package
has zero capabilities, so it cannot read `CI` itself: pass
`ci=ci_from_environ(os.environ)`, or `ci=False` for local verbose output.

### Conformance

The engine reproduces all 25 cases of `code/specs/fixtures/pbt-v1/cases.json`
(generator outputs, shrink minimals **and** step counts, replay, filter
exhaustion, failure-id validation, strict-parser rejection).
`pbt_corpus.run_pbt_corpus(cases)` returns the list of problems (empty means
conformant); `tests/test_pbt_corpus.py` runs it against the committed fixture,
asserts 25 cases were evaluated, and contains negative controls proving a
corrupted expectation, a truncated list or an unknown op is reported. Spec:
`code/specs/VER05c-python-pbt-engine.md`. The independent reference is
`python code/scripts/pbt_conformance.py`.

### Breaking changes in 0.2.0

`shrink_int`/`shrink_list`/`shrink_bytes` now implement the corpus shrinkers and
return `(minimal, steps)`; `shrink_text` and `shrink_value` are gone.
