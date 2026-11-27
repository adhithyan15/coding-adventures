## 0.4.0 -- hardening (LGE-EX-HARDEN)

### Changed (BREAKING)

- Size caps, checked before any work: select bits <= 16 (mux_n/demux/decoder),
  inputs/outputs <= 65536 (and_n, or_n, mux_n, demux, encoder,
  priority_encoder), register / shift-register width <= 65536, counter
  width <= 64. Over-cap input raises `ArgumentError`.
- List arguments must be plain proper lists; improper lists, structs, streams
  and maps raise `ArgumentError` without being enumerated. No caller protocol
  implementation (Inspect, String.Chars, Enumerable) is ever invoked.
- `demux` now requires `n_outputs` to be an integer, a power of two, >= 2 and
  <= 65536 (previously `demux(1, [], 0)` returned `[1, 1]`, negatives returned
  longer lists, and a huge bignum cost O(bits^2)). `n_outputs == 1` is rejected.
- `mux_n` requires a power-of-two input count (previously MatchError).
- Flip-flop states are validated (plain map with four bit fields); bad
  shapes raise `ArgumentError`, bad bits `InvalidBitError`. `counter` validates
  state even on reset. `shift_register` accepts only `direction: :left | :right`
  in a keyword list.
- All error messages are fixed text; `mux4` no longer uses `inspect/1`.
- `and_n(5)`, `mux_n/2` shape mismatches, unknown shift direction and similar
  no longer escape as FunctionClauseError/MatchError/CaseClauseError/KeyError.

### Fixed

- Shift functions are O(n) (prepend + reverse) instead of quadratic appends.
- Removed descending-range default in `demux` (`0..(n-1)//1`).

### Build

- BUILD/BUILD_windows compile with `--warnings-as-errors`; coverage gate
  raised from 80 to 95 (measured 95.8%).

### Added

- `Gates.max_select_bits/0`, `max_inputs/0`, `max_width/0`,
  `max_counter_width/0`; `test/coding_adventures/hardening_test.exs`.
