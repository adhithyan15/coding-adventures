# GAME10 - Portable Fixed-Step Clock Contract

## Status and scope

This specification records the current public integer behavior of the Rust `game-clock` package
for language-neutral conformance. The owning implementation is
`code/packages/rust/game-clock/src/lib.rs`. The public operations covered here are `new_clock`,
`validate_state`, and `advance_frame`. The configuration constructors `standard_60hz` and
`from_hz` are included because their 60 Hz rounding differs.

The contract describes checked-in implementation behavior. Host timing, rendering, and frame
pacing policy remain separate. The portable fixture carries integer alpha values. `alpha_f64()`
remains a host convenience.

## Types and wire values

`GameClockConfig` contains `fixed_dt_ns: u64`, `max_frame_steps: u32`, and
`max_frame_delta_ns: u64`.

`GameClockState` contains `tick_count`, `frame_count`, `elapsed_sim_ns`, `elapsed_wall_ns`,
`accumulator_ns`, `fixed_dt_ns`, `max_frame_delta_ns`, `total_dropped_ns`, and
`spiral_clamped_frames` as `u64`. `max_frame_steps` is `u32`.

`ClockStepResult` contains the next state, `steps_to_run: u32`, `alpha_milli: u32`,
`alpha_q16: u32`, `spiral_of_death_clamped: bool`, and `dropped_ns: u64`.

In `game-clock-v1/cases.json`, every `u64` value is a canonical ASCII base-10 JSON string. The
strings contain digits only, have no leading zero except `"0"`, and represent values from zero
through `18446744073709551615`. Every `u32` value is a JSON integer in range. JSON booleans remain
booleans and are rejected in integer fields. This representation preserves the full `u64` range
across runtimes with narrower JSON number types.

## Configuration construction and validation

`standard_60hz()` returns `fixed_dt_ns = 16666667`, `max_frame_steps = 8`, and
`max_frame_delta_ns = 250000000`. `from_hz(60, 8)` uses integer division and returns
`fixed_dt_ns = 16666666`. The fixture pins this one-nanosecond difference.

`new_clock(config)` validates in this order:

1. `fixed_dt_ns == 0` returns `ZeroFixedTimestep`.
2. `fixed_dt_ns > 10000000000` returns `InvalidMaxFrameDelta`.
3. `max_frame_steps` outside `1..=1024` returns `InvalidMaxFrameSteps`.
4. `max_frame_delta_ns < fixed_dt_ns` or `max_frame_delta_ns > 10000000000` returns
   `InvalidMaxFrameDelta`.

A successful call returns a state with zeroed counters and accumulator, with the validated
configuration copied into the state.

`from_hz(hz, max_frame_steps)` checks zero Hz first, then the step range. It computes
`fixed_dt_ns = floor(1000000000 / hz)`. A zero result returns `ZeroFixedTimestep`. It sets
`max_frame_delta_ns` to the greater of 250000000 and `fixed_dt_ns`, then validates the
configuration.

## State validation

`validate_state` checks fields in this order:

1. `fixed_dt_ns == 0` returns `ZeroFixedTimestep`.
2. `fixed_dt_ns > 10000000000` returns `CorruptedStateInvariant`.
3. `max_frame_steps` outside `1..=1024` returns `CorruptedStateInvariant`.
4. `max_frame_delta_ns` outside `fixed_dt_ns..=10000000000` returns
   `CorruptedStateInvariant`.
5. `accumulator_ns >= fixed_dt_ns` returns `CorruptedStateInvariant`.
6. Overflow in `tick_count * fixed_dt_ns` returns `TimestampOverflow`.
7. A value of `elapsed_sim_ns` different from that product returns `CorruptedStateInvariant`.

The validator accepts any `frame_count`, `elapsed_wall_ns`, `total_dropped_ns`, and
`spiral_clamped_frames` values when the checks above pass. Accepted-state coverage includes these
counters at `u64::MAX` and hand-authored states.

## Frame advance

`advance_frame(state, raw_frame_delta_ns)` validates the input state before processing the frame.
It returns errors according to the validation and checked-arithmetic rules above. The function
takes an immutable state reference. Errors leave the caller's state unchanged.

For a valid state, apply these steps using exact unsigned integer arithmetic:

1. Clamp the raw delta to `max_frame_delta_ns`. The difference
   `raw_frame_delta_ns - clamped_delta_ns` is wall-delta drop.
2. Add the clamped delta to the prior accumulator.
3. Count whole ticks. If the count is greater than `max_frame_steps`, run exactly the limit, set
   `spiral_of_death_clamped` to true, and drop the excess whole ticks. Preserve the sub-tick
   remainder. A count equal to the limit leaves the clamp flag false.
4. Set `dropped_ns` to wall-delta drop plus excess whole-tick time. Add it to
   `total_dropped_ns`. Increase `elapsed_wall_ns` by the clamped delta, `elapsed_sim_ns` by
   `steps_to_run * fixed_dt_ns`, `tick_count` by `steps_to_run`, and `frame_count` by one.
   Increase `spiral_clamped_frames` by one when the strict step limit is exceeded.
5. Compute `alpha_milli = floor(accumulator_ns * 1000 / fixed_dt_ns)` and
   `alpha_q16 = floor(accumulator_ns * 65536 / fixed_dt_ns)` from the resulting accumulator.
   Both values use integer floor division.

Every arithmetic update to a cumulative `u64` counter is checked. An overflowing update returns
`TimestampOverflow`. A zero delta still advances `frame_count`. Counters whose increment is zero
remain unchanged, including counters already at `u64::MAX`.

For every successful frame, the per-frame conservation equation is:

```text
previous_accumulator_ns + raw_frame_delta_ns
  = steps_to_run * fixed_dt_ns + dropped_ns + next_accumulator_ns
```

Across a sequence that starts with `new_clock` and whose frames all succeed, cumulative
conservation is:

```text
elapsed_sim_ns + accumulator_ns + total_dropped_ns
  = sum(raw_frame_delta_ns)
```

`validate_state` checks each supplied state using the listed invariants. Cumulative conservation
describes sequences initialized with `new_clock`. `elapsed_wall_ns` accumulates clamped deltas.

## Fixture contract

The language-neutral corpus is `code/specs/fixtures/game-clock-v1/cases.json`. It follows the
repository convention of a version, a suite name, and a `cases` array. Each case has a unique
`id`, an operation, its input, and one expected outcome. Successful outcomes contain the complete
config, state, or step result needed by the operation. Rejected outcomes contain the exact
`ClockError` variant.

The standard-library Python validator beside the corpus checks the wire representation and case
shape, then computes each expectation independently from this contract. It uses Python integers
for intermediate arithmetic. The frame model runs bounded ticks up to `max_frame_steps`, then
accounts for any remaining whole ticks as dropped time. Its reference model operates only on JSON
inputs and its own arithmetic.

Include successful vectors so an implementation that always returns an error fails. Include
rejected configuration and state inputs, validation-precedence cases, exact step-limit behavior,
both drop sources, integer alpha floors, `u64::MAX` raw delta, and isolated counter overflows.
Include valid states with arbitrary counters to pin the current acceptance boundary.

A failed `advance_frame` case also verifies that its supplied state remains unchanged in any
stateful adapter.

## Evidence boundary

This corpus is package-specific conformance evidence. During review, a temporary Rust harness
checked every committed vector against the public API. Checked-in language consumers are follow-up
work.

Each of the three checked-in Warrant claims treats any returned `Err` as satisfying the claim. Its
state invariant covers configuration and accumulator bounds. This contract also specifies the
`elapsed_sim_ns == tick_count * fixed_dt_ns` relation. The assurance report records property case
counts, with empty evidence arrays and null exhaustive, fuzz, and proof entries. Corpus-specific
execution evidence and source-level proof are absent from that report. Later evidence reports should
carry the package-specific conformance classification.
