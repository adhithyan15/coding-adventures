## Unreleased - CV2: contract row for `off-by-one-consumed` in the `Ok` form, README caveat for the `Option` row

### Added

- `tests/fixtures/off-by-one-consumed-v1/make_off_by_one_consumed.py`: a refusals row
  `a-non-numeric-second-component-before-the-ok-payload-clash-in-the-ok-form`
  (`op f(ok_payload: U8) -> Result<(U8,Bool),E>`), pinning that the numeric second-component check runs before the
  `ok_payload` clash in the `Ok` form as well as in the bare tuple form. The refusals dump is regenerated;
  `test_off_by_one_consumed.py` asserts the row's source and message.

### Changed

- The fixture README states that the `Option` row (`an-option-result`) reaches the code `fault` only through the
  oracle's bare `fault_row`: a real generator refuses an `Option` in a signature earlier, at lowering, with the level
  `ir.data` ([D23]). No code change, no version bump.
