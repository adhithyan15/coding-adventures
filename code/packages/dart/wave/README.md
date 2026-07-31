# coding_adventures_wave

Pure Dart implementation of the PHY01 simple-harmonic wave contract:

```text
y(t) = amplitude * sin(2 * pi * frequency * t + phase)
```

## What it provides

`Wave` stores a finite non-negative amplitude, a finite positive frequency,
and a finite optional phase offset. It exposes:

- `period()` for `1 / frequency`;
- `angularFrequency()` for `2 * pi * frequency`;
- `evaluate(time)` for the sinusoidal displacement at a point in time.

Construction rejects non-finite parameters, negative amplitudes, non-positive
frequencies, and frequencies whose angular frequency would overflow.
Evaluation rejects non-finite time, reduces finite time to one period before
multiplication, and bounds amplitude scaling. A zero amplitude is valid and
short-circuits to an exactly flat wave even for extreme finite inputs.

## Usage

```dart
import 'package:coding_adventures_wave/wave.dart';

void main() {
  final wave = Wave(1.0, 1.0);
  print(wave.evaluate(0.25)); // approximately 1.0
}
```

## How it fits in the stack

This PHY01 package depends directly on PHY00 `coding_adventures_trig`. It uses
the repository's first-principles `pi` and `sin` rather than `dart:math`.

## Running the tests

The tests consume
`../../../specs/fixtures/phy00-phy01-v1/cases/wave.json` through test-only
`dart:io`, so the shared cross-language cases do not add runtime authority.

```sh
dart pub get
dart analyze
dart test
```
