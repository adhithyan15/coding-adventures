## [Unreleased]

### Security / Hardening

- LGE-SWIFT-HARDEN (builds on LGE-SWIFT-CAPS; its BREAKING notes -- `inputs[i]`
  element names and the 65536 `andN`/`orN`/`shiftRegister` caps -- stand).
- Tests: wide-integer rejection matrix (2^31, 2^31+1, 2^32, 2^32+1, 2^53+1,
  Int.max, Int.min, -1) on all 11 gates in both argument positions, no
  short-circuit past a bad second argument, andN/orN element names; every
  block's rejection now asserts the exact `LogicGateError` payload instead of
  a bare `XCTAssertThrowsError`. 32-bit-Int targets are noted in the tests.
- `validateBit` takes its `name` as an `@autoclosure`: per-element labels such
  as `inputs[i]` are built only on the failure path, never on valid input.
- Removed the two force unwraps (`sel.last!` in `muxN`, `firstIndex(of: 1)!`
  in `encoder`); replaced by guard-let that throws a package error.
- `LatchState` / `FlipFlopState` are `Sendable`; the target builds with
  `StrictConcurrency` (complete checking), zero warnings.
- BUILD now also runs `swift test -c release` (`@testable` works in release
  under SwiftPM) and a 95% line-coverage gate (`check_coverage.sh`, currently
  97.7%). BUILD_windows runs the release tests only; the coverage script needs
  `sh` + `python3`, which are not assumed on Windows.
- No behaviour change for valid input; no manifest version bump (changelog-only
  versioning).
