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

### Security (follow-up)

- Validation now canonicalises: `_bit` reads the string form and returns a
  fresh integer 0/1, and every gate, variadic gate, sequential function and
  state constructor computes only on that value. Previously a dualvar such as
  `dualvar(5, "1")` passed the string check and was then computed numerically
  by the transistor layer (`AND(dualvar(5,"1"), 1)` returned 0). A dualvar is
  accepted as the bit its string says; dualvars whose string is not a bit are
  rejected.
- Counter widths are bounded to 1..65536 (CWE-400). The old `$width < 1`
  check numified references to their address (~1e11), so
  `new_counter_state([2])` tried to allocate `(0) x 1e11`. Absurd widths now
  die immediately; a test with an `alarm` proves it.

### Security (round 2)

- `_args` now strips a leading invocant whenever it is exactly the package name (never a valid bit), before the arity check, so `Class->DFlipFlop($d, $c)` and `Class->new_flip_flop_state()` work; a plain call with the package name as data dies.
- `_validate_width` returns a fresh integer built from the validated string form and every caller uses it, so `dualvar(-5, "3")` and `dualvar(70000, "3")` are width 3, never `(0) x -5`.
- `_describe` cuts the value to 32 characters before copying, so a 10 MB input yields a short error message.

- BREAKING (0.5.0): class-method call style removed. Every public function is a plain function and never guesses an invocant; a leading package-name argument is data and is rejected by bit validation or arity (previously `ANDn($pkg,1,0)` silently dropped an input). `$VERSION` is 0.5.0.
