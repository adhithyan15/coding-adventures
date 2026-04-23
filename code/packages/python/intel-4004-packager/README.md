# intel-4004-packager

Converts raw binary machine code into the **Intel HEX** format used by
EPROM programmers to burn ROM chips, and parses Intel HEX back to binary
for round-trip verification.

This is a standalone utility package with **no runtime dependencies**.
Pipeline orchestration (Nib source → binary → Intel HEX) lives in
`nib-compiler`.

## Where it fits

```
nib-compiler                    ← full pipeline orchestrator
    ↓  ...all compile stages...
    ↓
intel-4004-assembler            ← assembly text → binary bytes
    ↓
intel-4004-packager  ← YOU ARE HERE: binary bytes → Intel HEX
```

## Intel HEX format

Intel HEX dates back to 1973 and is the industry-standard format for
programming ROM chips.  Each line is a *record*:

```
:LLAAAATTDD...CC
```

| Field | Width | Meaning |
|---|---|---|
| `:` | 1 char | Start code (always colon) |
| `LL` | 1 byte | Byte count for this record |
| `AAAA` | 2 bytes | Load address (big-endian) |
| `TT` | 1 byte | Record type: `00` = data, `01` = EOF |
| `DD...` | LL bytes | Data bytes |
| `CC` | 1 byte | Checksum: `(0x100 - sum(preceding bytes)) & 0xFF` |

Records hold 16 data bytes each.  The final record is always:

```
:00000001FF
```

The checksum invariant: `sum(all bytes in record) % 256 == 0`.

## Usage

```python
from intel_4004_packager import encode_hex, decode_hex

# Convert assembled bytes to Intel HEX
binary = bytes([0xD7, 0x01])          # LDM 7; HLT
hex_text = encode_hex(binary)
# :02000000D70126
# :00000001FF

# Parse Intel HEX back to binary (round-trip)
origin, recovered = decode_hex(hex_text)
assert origin == 0
assert recovered == binary

# ROM at non-zero base address (e.g. second 256-byte page)
hex_page2 = encode_hex(binary, origin=0x100)
```

## API

### `encode_hex(binary: bytes, origin: int = 0) -> str`

Encodes `binary` as a multi-line Intel HEX string.

- `origin`: ROM load address (default 0x000; must fit in 16 bits)
- Returns a string ending with `:00000001FF\n`
- Raises `ValueError` if `origin + len(binary)` exceeds 0xFFFF

### `decode_hex(hex_text: str) -> tuple[int, bytes]`

Parses Intel HEX back to `(origin, binary)`.

- Raises `ValueError` on checksum errors or malformed records

## Running tests

```bash
uv run pytest tests/ -v
```

44 tests, 96% coverage.
