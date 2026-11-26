## 0.3.0

Breaking:

- `FlipFlopState` and `CounterState` are now `final class` (no caller subclasses,
  so getters cannot lie between validation and use).
- Caps (Rust parity), checked before any shift or allocation: select bits and
  `decoder` inputs <= 16, N-ary inputs and register / shift-register widths
  <= 65536, counter width <= 64. Over-cap input throws `ArgumentError`
  (previously 64 zero bits to `decoder` returned `[]`, 63 threw `RangeError`,
  huge widths hit OutOfMemoryError).
- `validateBit` / `InvalidBitError` accept `Object?`; non-int values yield a
  fixed `'non-integer value'` description (no caller `toString`/`==`).
- Shape errors no longer embed the caller list.

Fixes:

- HIGH: `muxN`, `xorN`, `andN`, `orN`, `encoder`, `priorityEncoder`, `decoder`,
  `demux`, `mux4/mux8`, `register` snapshot caller lists once (length read once,
  each index read once) and compute only from the copy; a lying `ListBase` can
  no longer make `muxN`/`xorN` return a non-bit.
- Add `analysis_options.yaml` (strict-casts/inference/raw-types,
  avoid_dynamic_calls); `dart analyze --fatal-infos` clean.
- Add `hardening_test.dart`; suite runs on VM and `dart test -p node`.

Documented limitation: on dart2js `1.0` / `-0.0` pass as bits (indistinguishable
from ints at runtime); `InvalidBitError` is an `Error`, not an `Exception`.
