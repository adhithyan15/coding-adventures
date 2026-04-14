# intel-4004-packager

The final stage of the Nib → Intel 4004 compiler pipeline. Takes compiled binary bytes and wraps them in **Intel HEX** format — the standard used by EPROM programmers since 1973.

## Pipeline Position

```
Nib source text
    → nib-parser           (text → AST)
    → nib-type-checker     (AST → typed AST)
    → nib-ir-compiler      (typed AST → IrProgram)
    → ir-optimizer         (IrProgram → optimized IrProgram)
    → intel-4004-backend   (IrProgram → assembly text)
    → intel-4004-assembler (assembly text → binary bytes)
    → intel-4004-packager  (binary → Intel HEX)  ← this package
```

## Quick Start

### Full pipeline (Nib source → Intel HEX)

```python
from intel_4004_packager import Intel4004Packager

packager = Intel4004Packager()
result = packager.pack_source("""
    fn main() -> u4 {
        let x: u4 = 7
        return x
    }
""")

# Write to file and burn to EPROM
with open("firmware.hex", "w") as f:
    f.write(result.hex_text)

# Or inspect intermediate stages for debugging
print(result.asm_text)      # Intel 4004 assembly
print(result.binary.hex())  # Raw bytes
```

### Lower-level: binary → Intel HEX

```python
from intel_4004_packager import encode_hex, decode_hex

# Encode
binary = bytes([0xD7, 0x01])   # LDM 7, HLT
hex_text = encode_hex(binary)
print(hex_text)
# :020000000000D70126
# :00000001FF

# Decode (for round-trip testing or loading from file)
origin, recovered = decode_hex(hex_text)
assert recovered == binary
```

## Intel HEX Format

Each line is a record:

```
:LLAAAATTDDDDDD...CC
 ││││││││││││││ └── checksum (two's complement of all prior bytes)
 ││││││││└───────── data bytes (LL × 2 hex chars)
 │││││└──────────── record type (00=data, 01=EOF)
 │└───────────────── address (16-bit big-endian)
 └────────────────── byte count
```

The checksum ensures that every byte (including the checksum byte itself) sums to 0 mod 256. EPROM programmers verify this before burning.

Example for two bytes at address 0x000:
```
:020000000000D70126
:00000001FF
```

## End-to-End Testing

The packager tests verify correctness all the way to register state using `Intel4004Simulator`:

```python
from intel4004_simulator import Intel4004Simulator
from intel_4004_packager import Intel4004Packager

packager = Intel4004Packager()
result = packager.pack_source("fn main() -> u4 { return 9 }")

sim = Intel4004Simulator()
exec_result = sim.execute(result.binary, max_steps=10_000)

assert exec_result.ok
assert exec_result.final_state.registers[1] == 9   # return value in R1
```

## API

### `Intel4004Packager`

| Method | Description |
|---|---|
| `pack_source(source: str) -> PackageResult` | Run the full pipeline |

Constructor options:
- `optimize=True` — enable IR optimizer (default: `True`)
- `origin=0x000` — ROM base address for Intel HEX (default: `0x000`)

### `PackageResult`

| Field | Type | Description |
|---|---|---|
| `typed_ast` | `ASTNode` | Type-checked AST |
| `raw_ir` | `IrProgram` | IR before optimization |
| `optimized_ir` | `IrProgram` | IR after optimization |
| `asm_text` | `str` | Intel 4004 assembly text |
| `binary` | `bytes` | Raw machine code |
| `hex_text` | `str` | Intel HEX ROM image |

### `PackageError`

Raised when any pipeline stage fails:

```python
try:
    result = packager.pack_source(source)
except PackageError as e:
    print(e.stage)    # "parse", "typecheck", "ir_compile", etc.
    print(e.message)
    print(e.cause)    # original exception, if any
```
