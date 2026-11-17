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

These vectors carry a package-specific conformance classification. During review, a temporary Rust harness checked every committed vector against the public API.
Checked-in language consumers are follow-up work. The fixture has no
generated Warrant or source-level proof attached.
