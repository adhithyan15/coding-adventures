## [0.2.1] - 2026-04-02

### Fixed
- Added `.PanicOnUnexpected()` to `Parse` so intentional panics (unexpected token, malformed input) propagate correctly instead of being swallowed by the Operations panic-recovery wrapper.

