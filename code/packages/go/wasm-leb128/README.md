# wasm-leb128 (Go)

LEB128 variable-length integer encoding for the WebAssembly binary format.

## What Is LEB128?

LEB128 (Little-Endian Base-128) is a variable-length encoding for integers.
Each byte carries 7 bits of data plus a continuation flag in the high bit:

- High bit = 1: more bytes follow
- High bit = 0: this is the last byte

WebAssembly uses LEB128 for every integer in its binary format — function
indices, type indices, memory sizes, instruction immediates, and so on. A
32-bit value that would always occupy 4 bytes in a fixed-width format may
use only 1 or 2 bytes with LEB128.

## Where It Fits in the Stack

```
wasm-leb128           ← this package (binary encoding primitives)
     ↓
wasm-binary-parser    (future: reads .wasm module sections)
     ↓
wasm-runtime          (future: executes WebAssembly)
```

## API

```go
import wasmleb128 "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128"
```

### `DecodeUnsigned(data []byte, offset int) (uint64, int, error)`

Reads an unsigned LEB128 integer from `data` starting at `offset`.
Returns `(value, bytesConsumed, error)`.

Returns `*LEB128Error` if the byte sequence is unterminated.

### `DecodeSigned(data []byte, offset int) (int64, int, error)`

Reads a signed LEB128 integer (two's complement sign extension).
Returns `(value, bytesConsumed, error)`.

Returns `*LEB128Error` if the byte sequence is unterminated.

### `EncodeUnsigned(value uint64) []byte`

Encodes a non-negative integer as unsigned LEB128.

### `EncodeSigned(value int64) []byte`

Encodes a signed integer as signed LEB128 (any int64 value).

### `LEB128Error`

Error type returned on malformed input.

```go
type LEB128Error struct {
    Message string  // human-readable description
    Offset  int     // byte position where decoding started
}

func (e *LEB128Error) Error() string
```

## Examples

```go
package main

import (
    "fmt"
    wasmleb128 "github.com/adhithyan15/coding-adventures/code/packages/go/wasm-leb128"
)

func main() {
    // Decode a single byte
    value, n, _ := wasmleb128.DecodeUnsigned([]byte{0x03}, 0)
    fmt.Println(value, n)  // 3 1

    // Decode multi-byte
    value, n, _ = wasmleb128.DecodeUnsigned([]byte{0xE5, 0x8E, 0x26}, 0)
    fmt.Println(value, n)  // 624485 3

    // Decode at an offset
    data := []byte{0xFF, 0x03}
    value, n, _ = wasmleb128.DecodeUnsigned(data, 1)
    fmt.Println(value, n)  // 3 1

    // Decode signed negative
    signed, n, _ := wasmleb128.DecodeSigned([]byte{0x7E}, 0)
    fmt.Println(signed, n)  // -2 1

    // Encode/decode round-trip
    encoded := wasmleb128.EncodeUnsigned(624485)
    fmt.Println(encoded)  // [229 142 38]

    // Error handling
    _, _, err := wasmleb128.DecodeUnsigned([]byte{0x80, 0x80}, 0)
    if lErr, ok := err.(*wasmleb128.LEB128Error); ok {
        fmt.Println(lErr.Message, lErr.Offset)
    }
}
```

## Development

```bash
# Run tests
go test ./... -v -cover

# Run vet
go vet ./...
```

## Test Coverage

41 tests, 96.0% statement coverage (exceeds 80% threshold).
