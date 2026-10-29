# RespProtocol (Swift)

This package is the authority-free Swift implementation of the DT23 RESP2
wire layer. It turns owned bytes into typed values and back; sockets, command
semantics, persistence, RESP3 negotiation, and inline commands belong to other
layers.

## Contract

- All seven RESP2 wire variants are distinct. Null bulk, null array, empty
  bulk, and empty array never collapse into one representation.
- Bulk strings retain arbitrary bytes, including NUL, invalid UTF-8, and CRLF.
- Simple and error strings are strict UTF-8 lines without embedded CR or LF.
  Their equality uses exact UTF-8 identity, not Unicode normalization.
- Integers use canonical signed decimal syntax and the full `Int64` domain.
  Lengths accept only canonical non-negative decimal or exactly `-1`.
- Decoding is failure-atomic: an incomplete nested value consumes nothing;
  malformed batches throw without returning a partial result.
- Tighten-only limits bound line/header bytes, bulk bytes, array width, nesting,
  total values, frame/output/buffer bytes, and repeated fragmented work before
  allocation or output growth.
- `RespDecoder` owns its retained tail, preserves complete-frame order, and
  commits no state when a candidate fragment is malformed.

The default profile deliberately admits frames up to 8 KiB. It is an
educational portable core, not a drop-in replacement for Redis deployments
that negotiate much larger application-specific payloads.

## Example

```swift
import RespProtocol

let command: RespValue = .array([
  .bulkString(Array("SET".utf8)),
  .bulkString(Array("key".utf8)),
  .bulkString([0x00, 0xFF]),
])

let wire = try RESP.encode(command)
let result = try RESP.decode(wire)

var decoder = RespDecoder()
for byte in wire {
  for value in try decoder.feed([byte]) {
    print(value)
  }
}
try decoder.finish()
```

The test target dynamically enumerates a closed package-local DT23 case roster
covering every wire kind plus malformed and incomplete frames. A shared
cross-language RESP corpus does not yet exist in `code/specs/fixtures`; that
repository-level reconciliation remains separate from this package.
