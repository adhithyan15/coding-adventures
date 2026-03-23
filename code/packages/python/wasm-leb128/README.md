# wasm-leb128 (Python)

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

```python
from wasm_leb128 import (
    decode_unsigned,
    decode_signed,
    encode_unsigned,
    encode_signed,
    LEB128Error,
)
```

### `decode_unsigned(data, offset=0) → (value, bytes_consumed)`

Reads an unsigned LEB128 integer from `data` starting at `offset`.
Returns `(value, bytes_consumed)`.

Raises `LEB128Error` if the byte sequence is unterminated.

### `decode_signed(data, offset=0) → (value, bytes_consumed)`

Reads a signed LEB128 integer (two's complement sign extension).
Returns `(value, bytes_consumed)`.

Raises `LEB128Error` if the byte sequence is unterminated.

### `encode_unsigned(value) → bytes`

Encodes a non-negative integer as unsigned LEB128.
Raises `ValueError` for negative input.

### `encode_signed(value) → bytes`

Encodes a signed integer as signed LEB128 (any integer, positive or negative).

### `LEB128Error`

Exception raised on malformed input.

```python
class LEB128Error(Exception):
    message: str   # human-readable description
    offset: int    # byte position where decoding started
```

## Examples

```python
from wasm_leb128 import decode_unsigned, encode_unsigned, decode_signed, encode_signed

# Decode a single byte
value, n = decode_unsigned(bytes([0x03]))
assert value == 3 and n == 1

# Decode multi-byte
value, n = decode_unsigned(bytes([0xE5, 0x8E, 0x26]))
assert value == 624485 and n == 3

# Decode at an offset in a larger buffer
data = bytes([0xFF, 0x03])
value, n = decode_unsigned(data, offset=1)
assert value == 3 and n == 1

# Decode signed negative
value, n = decode_signed(bytes([0x7E]))
assert value == -2 and n == 1

# Encode/decode round-trip
for v in [0, 1, 127, 128, 4294967295]:
    assert decode_unsigned(encode_unsigned(v))[0] == v

for v in [0, -1, -64, -2147483648, 2147483647]:
    assert decode_signed(encode_signed(v))[0] == v

# Error handling
from wasm_leb128 import LEB128Error
try:
    decode_unsigned(bytes([0x80, 0x80]))   # unterminated
except LEB128Error as e:
    print(e.message)  # "unterminated LEB128 at offset 0: ..."
    print(e.offset)   # 0
```

## Development

```bash
# Run tests
bash BUILD           # Linux/macOS
bash BUILD_windows   # Windows

# Run linter
uv run python -m ruff check src/
```

## Test Coverage

95 tests, 96.88% statement coverage (well above the 80% threshold).
