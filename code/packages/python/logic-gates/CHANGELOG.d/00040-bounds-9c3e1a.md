## Security (unversioned; PR #19933 owns the version bump and the CHANGELOG.d conversion)

- `decoder`, `demux`, `mux_n`: reject oversize requests with `ValueError` before
  any allocation. New constants in `combinational.py`:
  `MAX_SELECT_BITS = 16`, `MAX_WIDTH = 1 << 16`, `MAX_COUNTER_WIDTH = 64`.
- `encoder`, `priority_encoder`, `decoder`, `mux_n`, `demux`: require `type(x) is list`
  (`TypeError` for dict/tuple/set/str/subclasses), validate every element, cap length.
- `demux`: `n_outputs` must be an exact `int`.
- `counter`: validates `state` (dict shape, `value` elements and length == width,
  `ff_state`) on every path (hold, reset, increment); `width` is an int in 1..64.
- `register`, `shift_register`: flip-flop state dicts are validated (exact keys, bit
  values) with the offending parameter named; width capped at `MAX_WIDTH`.
