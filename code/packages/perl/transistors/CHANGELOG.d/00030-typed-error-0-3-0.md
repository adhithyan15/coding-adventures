## [0.03]

### Added

- `CodingAdventures::Transistors::Error` (`lib/CodingAdventures/Transistors/Error.pm`):
  a blessed exception with read-only `code`, `param`, `value_description` and
  `message`. Codes: `INVALID_BIT`, `INVALID_ARITY`, `INVALID_NUMBER`.
  Stringifies to the old message (ending in a newline); `bool` is true;
  `eq`/`ne`/`cmp`/`.`/`x` use the string; numeric operators fail loudly.
- `is_invalid_bit($err)` (exported): true only for a blessed object of exactly
  the Error class with code `INVALID_BIT` (`ref`, not `isa`: forged hashrefs and
  subclasses do not count).

### Changed

- Every rejection (`_check_bit`, `_num`, `_args`) now throws an Error instance
  instead of a bare string. Messages are unchanged, so existing `like($@, ...)`
  checks still pass. `CodingAdventures::LogicGates` only calls `cmos_*` with
  already-validated bits, so it never sees these errors.

### Tests

- `t/03-error-class.t`: bad-input matrix over every public function (bit,
  number and arity) asserting class, code and param; overload behaviour
  (concatenation, warn, die wrapping, numeric operators failing); forged
  hashref, foreign class and subclass rejected by the predicate.
