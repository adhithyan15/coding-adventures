# intel-8008-packager

Intel HEX ROM image encoder/decoder for Intel 8008 binary programs.

This is the final stage of the Oct → Intel 8008 compiler pipeline:

```
intel-8008-assembler  →  binary bytes
intel-8008-packager   →  Intel HEX file (.hex)
intel8008-simulator   reads the .hex file directly
```

## What is Intel HEX?

Intel HEX is a text-based file format invented in the early 1970s for
loading programs into EPROM chips.  Each line is a "record":

```
:LLAAAATTDD...CC
:  — start code
LL — byte count (2 hex digits)
AAAA — 16-bit address (4 hex digits, big-endian)
TT — record type (00=data, 01=EOF)
DD — data bytes
CC — checksum (two's complement of all preceding bytes)
```

The format is understood by every EPROM programmer on the market and
has survived 50+ years unchanged.

## Intel 8008 Address Space

The Intel 8008 has a 14-bit address space covering 16 KB:

| Range | Content |
|-------|---------|
| 0x0000–0x1FFF | ROM: program code (8 KB) |
| 0x2000–0x3FFF | RAM: static variable data (8 KB) |

This packager handles the full 16 KB range.

## Usage

```python
from intel_8008_packager import encode_hex, decode_hex

# Binary → Intel HEX (ready for EPROM programmer or simulator)
binary = bytes([0x06, 0x00, 0xFF])   # MVI B, 0; HLT
hex_text = encode_hex(binary)
print(hex_text)
# :030000000600FF...
# :00000001FF

# Intel HEX → binary (round-trip / simulator loading)
origin, recovered = decode_hex(hex_text)
assert recovered == binary

# Non-zero origin (e.g. for RAM-region data)
ram_data = bytes([0x00] * 16)
ram_hex = encode_hex(ram_data, origin=0x2000)
```

## Key Differences from intel-4004-packager

| Aspect | 4004 packager | 8008 packager |
|--------|---------------|---------------|
| Address space | 12-bit (4 KB) | 14-bit (16 KB) |
| Max image size | 4 096 bytes | 16 384 bytes |
| Record format | identical | identical |

The Intel HEX format itself is the same — only the address cap differs.

## Tests

```bash
bash BUILD
```

Tests cover:
- Record format (byte count, address, type, data, checksum fields)
- Checksum correctness (sum of record bytes = 0 mod 256)
- Multi-record output (records split at 16-byte boundaries)
- Non-zero origin
- Round-trip encode → decode
- Error cases (empty binary, overflow, bad checksum, unsupported type)
- 8008-specific: addresses in ROM and RAM regions, 16 KB cap
