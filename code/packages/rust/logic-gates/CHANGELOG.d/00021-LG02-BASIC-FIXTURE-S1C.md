## Unreleased — the basic-gates fixture exists; drift guard checks 10-12 are mandatory (LGE-S1c)

- `code/scripts/logic_gates_basic_drift.py`: `FIXTURE_REQUIRED = True`; a pin on a claim the corpus lists as
  `blind_claims` passes only for the two documented `shift-mask` rejection pins (`KNOWN_STAND_IN_BLIND`).
- Tests: the synthetic trees start in the pre-fixture mode; new tests for the exception, its two limits and the
  shipped flag. No change to this package's BUILD or sources.
