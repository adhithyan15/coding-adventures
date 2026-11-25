## [0.2.0]

### Added

- `InvalidBitException` (subclass of `ArgumentOutOfRangeException`) with
  `ParameterName`/`ParamName` and `Value`, thrown for any non-0/1 input.
- `LogicGatesExhaustiveTests`: literal truth tables for all 11 gates,
  NAND-built == direct, and every int in [-1024, 1024] plus int extremes
  rejected in every argument position with the correct name and value;
  char/byte/short arguments rejected at runtime.

### Changed

- Every gate (Nand, Nor, Xnor and the NAND-built gates included) validates all
  of its own arguments up front, so the first invalid parameter is reported
  and never an inner helper's name. AndN/OrN validate every input first.
- Existing rejection tests use `ThrowsAny` because the thrown type is now a
  subclass; `catch (ArgumentOutOfRangeException)` callers are unaffected.
- Coverage threshold raised to 95.
