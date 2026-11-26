## 0.2.0 (package version) -- InvalidBitError; exhaustive validation

### Changed (BREAKING)

- Invalid gate inputs now raise `CodingAdventures.LogicGates.InvalidBitError`
  (fields `:param`, `:value_description`) instead of `ArgumentError`. Elixir
  exceptions cannot subclass, so callers rescuing `ArgumentError` for bad
  bits must rescue the new exception. Length errors in `and_n`/`or_n` and the
  structural errors in combinational helpers remain `ArgumentError`.
- A bit is valid iff `is_integer(v) and (v === 0 or v === 1)`; `1.0` is rejected.
- `value_description` is bounded and never calls user protocol
  implementations (no `inspect`/`to_string`); integers beyond 2**64 are
  described by bit length so huge values stay cheap.
- `mix.exs`: version 0.2.0; protocols not consolidated in :test so the test
  suite can prove no Inspect/String.Chars implementation is invoked.

### Added

- `test/coding_adventures/gates_exhaustive_test.exs`: truth tables for all 11
  basic gates, NAND-built equivalence, ~1000 integers plus bignums and a
  type-confusion matrix in every argument position, first-invalid-param
  reporting, and bounded-message checks.
