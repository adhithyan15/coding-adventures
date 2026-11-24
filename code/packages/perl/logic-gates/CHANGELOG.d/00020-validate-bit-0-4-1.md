## [0.4.1]

### Security

- Fixed CWE-20 / CWE-843 type confusion in the bit validator. `_validate_bit`
  compared with numeric `!=`, which numifies its argument, so `AND("1abc", 1)`
  returned 1 and `AND("a", 1)`, `AND("", 1)` and `AND(undef, 1)` returned 0 with
  at most a warning. A bit is now accepted only if it is defined, not a
  reference, and its string form is exactly `0` or `1` (`/\A[01]\z/`). Rejected:
  `"1abc"`, `""`, `undef`, `"1.0"`, `" 1"`, `"1\n"`, `"01"`, `"+1"`,
  `"0 but true"`, dualvars with a non-bit string part, all references, blessed
  and overloaded objects. The check does not rely on warnings.
- The same validator now guards every argument of all 11 gates, `ANDn`, `ORn`,
  `XORn`, `SRLatch`, `DLatch`, `DFlipFlop`, `Register`, `ShiftRegister`,
  `Counter` and the state constructors (previously the sequential functions
  carried their own copies of the numeric check). Counter widths must be
  positive decimal integers up to 999999; container arguments are type-checked.

### Changed

- Argument counts are now checked. The old "3 arguments means method call"
  heuristic silently dropped the first argument of `AND(x, y, z)`; a leading
  invocant is now stripped only when it is exactly `CodingAdventures::LogicGates`,
  and any other wrong count dies.
- `$VERSION` brought in line with the changelog (0.4.1).

### Tests

- New `t/02-validation.t`: literal truth tables for all 11 gates, a
  type-confusion matrix over every gate and argument position (with a
  no-warnings assertion), arity, variadic, sequential and state cases, and the
  `AND("1abc", 1)` regression.
