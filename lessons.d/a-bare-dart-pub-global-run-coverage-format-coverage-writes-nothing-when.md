---
category: Testing & coverage
---

# A bare dart pub global run coverage:format_coverage writes nothing when coverage is not globally activated, so always verify lcov.info freshness or use the BUILD coverage gate

`dart pub global run coverage:format_coverage ...` prints `No active
package coverage.` and exits 0 without writing anything when the
`coverage` package was never `dart pub global activate`d in the
active `PUB_CACHE` — and a stale `coverage/lcov.info` from an
earlier run then masquerades as a fresh gate result. This bit a
post-restart verification: the global invocation "succeeded", awk
read a pre-restart `lcov.info`, and the numbers looked green.

Do one of these instead:

- Run the sanctioned gate from the lane's BUILD file, which needs
  no global activation (coverage rides as a dev dependency):
  `dart run coverage:test_with_coverage --branch-coverage
  --function-coverage --fail-under=95`.
- Or delete `coverage/lcov.info` before the global invocation and
  treat a missing file afterwards as a gate failure, not a pass.

Validate with `python3 code/scripts/lessons.py validate` (this very
lesson file must pass it too).

