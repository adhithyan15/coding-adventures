# Hamurabi (Swift)

Deterministic simulation core for the seminal 1968 mainframe resource allocation game Hamurabi (GAME01 / HA01).

See `code/specs/hamurabi.warrant` for the formal verification contract.

## Historical Lineage

- **1964 — The Sumerian Game:** Designed by Mabel Addis and programmed by William McKay for an IBM 7090 mainframe at the Board of Cooperative Educational Services (BOCES) in Westchester County, New York.
- **1968 — FOCAL Hamurabi:** Implemented by Richard Merrill on the Digital Equipment Corporation (DEC) PDP-8 in FOCAL-69.
- **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David H. Ahl and published by DEC.

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER27`, `VER29`):
- Strictly zero ambient system authority: `"capabilities": []` declared in `required_capabilities.json`.
- Pure state transition: `stepYear(state:order:) -> Result<HamurabiState, HamurabiError>`.
- Explicit pseudorandomness: all simulation rolls enter deterministically through explicit PCG32 state stored directly inside `HamurabiState`.
- Pure integer arithmetic: bounded integer types with zero floating-point operations.
- Zero heap allocation on core simulation step: immutable `Sendable` value structs.
- Complete CWE-190 mitigation: Swift traps on integer overflow by default. All arithmetic operations use wrapping arithmetic (`&*`, `&+`), reporting overflow methods (`multipliedReportingOverflow`, `addingReportingOverflow`), or safe widening (`UInt32` to `UInt64`, `Int32.min` negation guard via `Int64`).

## API

```swift
import Hamurabi

// Initialize a new 10-year reign
let state = newGame(seed: 1337)

// Formulate player decisions for the year
let order = TurnOrder(
    acresDelta: 0,
    bushelsToFeed: 2000,
    acresToPlant: 1000
)

// Validate order without advancing state
switch validateOrder(state: state, order: order) {
case .success:
    // Execute turn transition
    let result = stepYear(state: state, order: order)
    if case .success(let nextState) = result {
        print(formatReport(state: nextState))
    }
case .failure(let error):
    print("Invalid order: \(error.message)")
}
```

## Development & Testing

```bash
cd code/packages/swift/hamurabi
swift test --enable-code-coverage --verbose
```

Or using repo build scripts:

```bash
sh BUILD
```
