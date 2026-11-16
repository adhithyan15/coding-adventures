# coding_adventures_hamurabi (Dart)

Deterministic simulation core for the seminal 1968 mainframe resource allocation game Hamurabi (GAME01 / HA01).

See `code/specs/hamurabi.warrant` for the formal verification contract.

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis and programmed by William McKay for an IBM 7090 mainframe at the Board of Cooperative Educational Services (BOCES) in Westchester County, New York.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the Digital Equipment Corporation (DEC) PDP-8 in FOCAL-69.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published by DEC.

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER27`, `VER29`):
- Strictly zero ambient system authority: `"capabilities": []` declared in `required_capabilities.json`.
- Pure state transition: `stepYear(state, order) -> Result<HamurabiState, HamurabiError>`.
- Explicit pseudorandomness: all simulation rolls enter deterministically through explicit PCG32 state stored inside `HamurabiState`.
- Pure integer arithmetic: bounded integer types with zero floating-point operations.
- Zero heap allocation on core simulation step: immutable value objects.
- Closure of CWE-190 and CWE-681: On the native Dart VM, `int` is a signed 64-bit two's-complement integer. PCG32 64-bit unsigned arithmetic (`state` and `inc`) uses `BigInt` with `toUnsigned(64)`, avoiding signed integer overflow on values $\ge 2^{63}$. Resource arithmetic uses strictly checked `int` bounded to `u32` (`[0, 0xFFFFFFFF]`), and guards `-2147483648` before negation.

## API

```dart
import 'package:coding_adventures_hamurabi/coding_adventures_hamurabi.dart';

// Start a new 10-year reign
final state = newGameFromInt(1337);

// Formulate an order for the year
final order = TurnOrder(
  acresDelta: 0,
  bushelsToFeed: 2000,
  acresToPlant: 1000,
);

// Validate order without advancing state
final error = validateOrder(state, order);
if (error == null) {
  // Execute turn transition
  final result = stepYear(state, order);
  if (result is Ok<HamurabiState, HamurabiError>) {
    final nextState = result.value;
    print(formatReport(nextState));
  }
}
```

## Development & Testing

```bash
cd code/packages/dart/hamurabi
dart pub get
dart test
```

Or using repo build scripts:

```bash
sh BUILD
```
