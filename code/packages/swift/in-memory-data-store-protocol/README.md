# InMemoryDataStoreProtocol (Swift)

This package is the dependency-free command/response value boundary for the
Swift in-memory data-store stack. It sits between a RESP/TCP facade and the
command engine without importing either layer.

## Contract

- `CommandFrame` accepts visible ASCII commands and normalizes only `a`–`z`.
- Ordered argument bytes are retained as Swift values and may contain NUL or
  invalid UTF-8.
- `EngineResponse` distinguishes simple strings, errors, signed 64-bit
  integers, nullable bulk bytes, and nullable ordered response arrays.
- Null bulk, null array, empty bulk, and empty array remain distinct.
- Tighten-only command and response budgets fail closed below hard package
  ceilings.
- Iterative response validation uses depth-bounded traversal storage and
  reports element, depth, and exact payload-byte metrics.
- Simple/error strings reject CR and LF and compare by exact UTF-8 identity;
  integers contribute eight payload bytes.
- The package has no RESP, socket, clock, persistence, or host-authority
  dependency.

## Example

```swift
import InMemoryDataStoreProtocol

let frame = try CommandFrame(parts: [
  Array("set".utf8),
  [0x6B, 0x65, 0x79],
  [0x00, 0xFF],
])

frame.command   // "SET"
frame.arguments // binary-safe key and value

let response = EngineResponse.array([
  .ok,
  .bulkString([0x00, 0xFF]),
  .null,
])
let metrics = try response.validate()
```

Wire adapters own RESP conversion. Engines own command semantics. Keeping this
package as a small immutable IR prevents either concern from leaking into the
other.
