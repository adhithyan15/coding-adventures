# coding_adventures_clock (Dart)

Square-wave clock generator with edges, dividers, and multi-phase clocks.
The Dart lane of the Rust `clock` crate: the heartbeat for simulated
sequential logic.

```dart
import 'package:coding_adventures_clock/coding_adventures_clock.dart';

final clk = Clock(1000000); // 1 MHz
clk.registerListener((edge) {
  if (edge.isRising) print('cycle ${edge.cycle}');
});
clk.run(3); // 6 ticks, 3 rising edges

final divider = ClockDivider(1000000000, 4); // 1 GHz -> 250 MHz
final phases = MultiPhaseClock(4); // 4 non-overlapping phases
```

## API

- `Clock(frequencyHz)` — toggles 0 → 1 → 0. `tick()` advances one
  half-cycle and returns the [ClockEdge] (cycle, value, direction);
  `fullCycle()` runs two ticks; `run(n)` runs n cycles; `reset()` restores
  rest state but keeps listeners; `periodNs()`, `cycle`, `value`,
  `totalTicks` observe state. Throws [ArgumentError] for non-positive
  frequencies.
- `ClockDivider(sourceHz, divisor)` — counts source rising edges via
  `onEdge`; every `divisor`-th one runs a full cycle on `output` (whose
  frequency is `source ~/ divisor`). Divisor must be ≥ 2.
- `MultiPhaseClock(phases)` — `onEdge` rotates the single active phase on
  rising edges (falling edges ignored); `getPhase(i)` reads 0/1 and throws
  [RangeError] out of range. Phase count must be ≥ 2.
- `ClockEdge` — immutable value record with structural equality.

One mapping note: the reference validates constructor arguments with
`assert!` (panics); the Dart lane throws [ArgumentError] with the same
messages, since Dart asserts are debug-only. Validation order is
preserved: the divisor/phase count is checked before any member the
check guards is constructed.

## Validation

- **Unit** (`test/clock_test.dart`): mirrors the reference suite, plus
  listener ordering and reset survival, divider ratios/output wiring,
  multiphase rotation/bounds, edge value semantics, and exact validation
  messages. 25 tests.
- **Property** (`test/properties_test.dart`): tick law, edge-stream shape,
  run/reset restoration, divider ratio and frequency (including the
  zero-quotient rejection), multiphase rotation, listener delivery, period
  formula — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration script-driven
  differential campaign (ticks, cycles, runs, resets, synthetic edges)
  against a shadow model after every op. Ad-hoc 60,000-iteration runs over
  three more seeds: zero divergences.
- **Coverage**: 100% lines, branches, and functions.
