## Unreleased - CV2 S10d: the kit grew by 461 cases at `ir.fixed` and `ir.tuple` (tests only)

`code/specs/fixtures/warrant-corpus-conformance-v1` holds 1,628 v2 cases (S10d added 670). This lane runs 839 (782 before, plus the 57 new cases
that need `ir.core` alone) and skips the other 613 by their `requires`: 0 failed. `tests/test_v2_conformance.py` pins the new run and skipped counts
and nothing else moved: no runner source, no manifest.
