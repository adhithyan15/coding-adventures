# coding_adventures_trig

Pure Dart implementation of the PHY00 trigonometry contract. The package
computes its numeric results from first principles and does not delegate to
`dart:math` trigonometric functions.

## What it provides

- `pi`, `twoPi`, and `halfPi` angle constants;
- `sin` and `cos` using 20-term Maclaurin series after range reduction;
- `radians` and `degrees` angle conversion;
- `sqrt` using power-of-four-normalized Newton iteration with explicit
  negative-input rejection, signed-zero preservation, and infinity handling;
- `tan`, `atan`, and four-quadrant `atan2` built on the local primitives.

The precision target is an absolute error no greater than `1e-10` for the
PHY00 conformance cases, including tiny normal and subnormal square roots.
Tangent returns the exact signed finite sentinel `1e308` within `1e-15` of a
pole, matching the cross-language contract.

## Usage

```dart
import 'package:coding_adventures_trig/trig.dart' as trig;

void main() {
  final angle = trig.radians(45.0);
  print(trig.sin(angle));
  print(trig.atan2(1.0, -1.0));
}
```

## How it fits in the stack

This is the dependency-free PHY00 numeric leaf. The PHY01 `wave` package uses
its `pi` constant and `sin` implementation.

## Running the tests

The tests consume
`../../../specs/fixtures/phy00-phy01-v1/cases/trig.json` through test-only
`dart:io`, so the shared cross-language cases do not add runtime authority.

```sh
dart pub get
dart analyze
dart test
```
