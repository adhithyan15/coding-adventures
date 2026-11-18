# @coding-adventures/game-clock (GAME10)

A pure, deterministic fixed-step clock for simulation code. It accumulates integer nanoseconds,
limits work per rendered frame, accounts for dropped time, and returns interpolation factors without
floating-point drift.

## Public API

The package exports the five GAME10 operations `standard_60hz`, `from_hz`, `new_clock`,
`validate_state`, and `advance_frame`. Config, state, and step-result records use the snake-case
field names in the contract and are readonly. Every `u64` is a JavaScript `bigint`; `max_frame_steps`
and result `u32` fields are bounded integer `number` values. Returned records are fresh and frozen.

Operations with semantic refusals return a discriminated result:

```ts
import {
  advance_frame,
  new_clock,
  standard_60hz,
} from "@coding-adventures/game-clock";

const created = new_clock(standard_60hz());
if (created.ok) {
  const frame = advance_frame(created.value, 16_666_667n);
  if (frame.ok) {
    console.log(frame.value.steps_to_run, frame.value.alpha_q16);
  }
}
```

The closed semantic error variants are `ZeroFixedTimestep`, `InvalidMaxFrameSteps`,
`InvalidMaxFrameDelta`, `CorruptedStateInvariant`, and `TimestampOverflow`. Host binding mistakes
throw `TypeError` for the wrong JavaScript kind and `RangeError` for a value outside its declared
integer width. The package does not coerce `number` values to `bigint`, wrap them, or truncate them.
See [`GAME10-fixed-step-clock-contract.md`](../../../specs/GAME10-fixed-step-clock-contract.md)
for validation order, exact integer behavior, and evidence limits.

## Verification

The package BUILD runs TypeScript compilation and the checked-in Node test suite. Tests validate the
original GAME10 fixture bytes with its Python validator before parsing them in JavaScript, then
dispatch all 23 public API vectors and compare complete values or exact error variants. The suite
also contains non-vacuity controls, input-atomicity and full-width counter cases, a seeded 1,024-case
property campaign, a 256-case bounded seeded mutation campaign, and a 16-input SAT miter with a
mutant witness.

These results are finite package-level evidence over the recorded corpus, seeds, mutation bound,
and SAT encoding. They do not establish a full-domain proof, native source proof, or platform
runtime availability. JavaScript `bigint` arithmetic and returned records allocate; this package
does not establish the native GAME10 zero-allocation, per-step, or constant-time proof targets.
The Rust assurance report and the Warrant `validate_state`/`advance_frame` proof obligations
remain separately tracked.

The package has no production dependencies and declares no ambient filesystem, process, network, or
environment capabilities.
