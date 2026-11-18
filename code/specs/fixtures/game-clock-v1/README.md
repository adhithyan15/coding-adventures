# GAME10 Portable Clock Vectors

This directory contains the first language-neutral vectors for the `game-clock` integer API
specified in [`GAME10-fixed-step-clock-contract.md`](../../GAME10-fixed-step-clock-contract.md).

## Files

- `cases.json` contains 23 expected examples for configuration construction, state validation, and
  frame advancement. Nine cases succeed and fourteen return a specific `ClockError`.
- `validate.py` is a standard-library Python validator. It checks canonical ASCII decimal strings
  for every `u64`, rejects duplicate JSON keys, and evaluates each case with an independent
  integer reference model.

Run it from the repository root:

```sh
python code/specs/fixtures/game-clock-v1/validate.py
```

The vectors cover the differing 60 Hz constructor values, validation precedence, accepted
hand-authored counters, exact step-limit behavior, both sources of dropped time, integer alpha
floors, `u64::MAX` input, and cumulative-counter overflow. The reference model executes at most
`max_frame_steps` ticks by subtraction, then drains any remaining whole ticks with integer division.

These vectors define package-specific conformance. The Rust `game-clock` package test consumes
every case through the public API, checks every output field or exact `ClockError`, and checks that
failed advances preserve their input state. This is finite corpus evidence; it is not a formal proof
or exhaustive evidence for the full input domain. The fixture has no generated Warrant or
source-level proof attached.
