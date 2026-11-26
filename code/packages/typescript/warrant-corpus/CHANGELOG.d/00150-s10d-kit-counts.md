## Unreleased - CV2 S10d: the kit grew by 461 cases at `ir.fixed` and `ir.tuple` (tests only)

`code/specs/fixtures/warrant-corpus-conformance-v1` holds 1,419 v2 cases (S10d added 461). This lane runs 843 (782 before, plus the 61 new cases
that need `ir.core` alone) and skips the other 400 by their `requires`: 0 failed. `tests/v2/kit.test.ts` and `tests/v2/review-fixes.test.ts` pin the
new run and skipped counts and nothing else moved: no runner source, no manifest.
