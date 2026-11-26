## [0.6.0]

### Added

- `CodingAdventures::LogicGates::Error` (`lib/CodingAdventures/LogicGates/Error.pm`):
  a blessed exception with read-only `code`, `param`, `value_description` and
  `message`. Codes: `INVALID_BIT`, `INVALID_ARITY`, `INVALID_WIDTH`,
  `INVALID_SHAPE`. Stringifies to the old message (ending in a newline); `bool`
  is true; `eq`/`ne` compare the string.
- `is_invalid_bit($err)` (exported): true only for a blessed object of exactly
  the Error class with code `INVALID_BIT` (`ref`, not `isa`: forged hashrefs and
  subclasses do not count).

### Changed

- Every rejection in `LogicGates.pm` now throws an Error instance instead of a
  bare string, so verification bindings match on `code`, never on message text
  (LG02 sections 7 and 9). Existing `like($@, qr/.../)` checks still pass.
- Transistors' error API is unchanged (possible follow-up).

### Tests

- The bad-value matrix over all 11 gates and every argument position asserts
  class, code, parameter name and `is_invalid_bit`; new tests for arity, width,
  shape, bounded `value_description`, stringification and forged/subclass cases.
