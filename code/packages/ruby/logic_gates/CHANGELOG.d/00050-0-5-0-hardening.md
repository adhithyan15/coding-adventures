## 0.5.0

Security hardening (LGE-RB-HARDEN). Behaviour changes:

- **Breaking:** `Combinational` and `Sequential` bit validators now use the
  gates' exact validator and raise `InvalidBitError` (an `ArgumentError`)
  naming the block's own parameter; they no longer raise `TypeError`, and
  no longer call `value.class` / `value.to_s` on the caller's object.
- Class names in error messages are read via the unbound `Module#name`, must
  be an exact String, have control and non-ASCII characters escaped
  (`\xNN`, `\u{...}`), and are capped at 64 characters on an escape boundary
  (CWE-117/150, e.g. a name set with `set_temporary_name`).
- `and_n` / `or_n`: new frozen-integer cap `MAX_INPUTS = 65_536` (checked
  before any work), every element validated up front (so `and_n(0, bad)`
  raises) with `InvalidBitError#param` of `inputs[i]`. Inputs are the
  positional splat (an exact, private Array); a lazy enumerator or other
  container passed positionally is a single non-bit element and is rejected.
- New resource caps checked before allocation: `Combinational::MAX_SELECT_BITS`
  = 16 (decoder inputs, demux/mux_n/encoder sizes up to 2**16),
  `Sequential::MAX_WIDTH` = 65_536 (register, shift_register),
  `Sequential::MAX_COUNTER_WIDTH` = 64. Over-cap raises `ArgumentError`.
  `demux` now requires an Integer `n_outputs`; encoders/`mux_n` require an Array.
- Caller `state:` is validated and snapshotted: a non-Hash, wrong or extra
  keys, wrong lengths raise `ArgumentError` (never `NoMethodError`); bad bit
  values raise `InvalidBitError`.
- The seven `CMOS_*` constants are deep-frozen and Ractor-shareable; gates run
  inside a non-main Ractor.
- Tests: LG02 section 9 matrix rows on all gates in both positions including
  short-circuit rows, probe-object spy counters, name-escaping/cap tests,
  state and cap tests. SimpleCov minimum raised to 95. BUILD and
  BUILD_windows now run `standardrb`.
- Out of scope (cannot be closed inside the package): in-process monkeypatching
  of core classes such as redefining `Integer#==`.

Review follow-ups (same release):

- Container arguments (`inputs`, `sel`, `data`, `state` and nested
  `ff_states` / `bits`) must be exact `Array` / `Hash`; subclasses and duck
  types are rejected with `ArgumentError` without any method being called on
  them, and the validated copy is the only thing used afterwards.
- Non-Array / wrong-size `sel` in `mux4`/`mux_n`/`demux`, a non-Array `inputs`,
  and a non-Integer `demux` `n_outputs` ("n_outputs must be an Integer") raise
  `ArgumentError`, not `InvalidBitError`.
- Class-name escaping: backslash is doubled, every non-ASCII code point is
  `\u{HEX}`, and `\xNN` is used only for raw bytes of an invalid-UTF-8 name
  (which no longer raises from `ord`).
- Messages no longer use `inspect`; `CMOS_*` mutation raises `FrozenError`.
- RBS updated for the `MAX_*` constants.
- State hashes are copied without hashing or comparing any caller key: bound `Hash#keys`, every key must be a Symbol in the allowed frozen list, values read only for those keys via bound `Hash#[]` (default procs never fire). `shift_register` describes a bad `direction` via the safe class-name helper instead of `inspect`.
- Round 3: `NilClass === state` replaces `state.nil?` (no caller dispatch); error messages echo bignums only through `describe_value` (bit length); oversize caller Arrays are refused by bound `Array#length` before copying, oversize Hashes by bound `Hash#size` before `keys`; class names are byte-truncated before scanning.
- Round 4 (behaviour change): error messages no longer include the caller's
  class name. `value_description` is the Integer itself (bit length beyond 64
  bits) or one of the fixed texts "nil", "a boolean", "a Float", "a String",
  "a Symbol", "an Array", "a Hash", "a Rational", "a Complex",
  "a value of another type", chosen by exact class identity. The class-name
  escaping code was removed. `sr_latch`, `d_latch`, `d_flip_flop`, `register`,
  `shift_register` and `counter` reject unknown keywords with the fixed text
  "unknown keyword". Unreachable duplicate cap re-checks were removed.
- Round 5: identity checks use unbound `BasicObject#equal?` with OUR class as
  the receiver, so a caller class with a singleton `equal?` is never invoked.
  Note: Ruby itself hashes non-Symbol keyword keys at the call site when
  building `**rest`; that is outside the package's control.
