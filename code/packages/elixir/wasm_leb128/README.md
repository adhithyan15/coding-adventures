# wasm_leb128

LEB128 (Little-Endian Base-128) variable-length integer encoding for the
WebAssembly binary format, implemented in Elixir.

## What is LEB128?

LEB128 packs 7 bits of data into each byte and uses the high bit (bit 7) as a
"more bytes follow" flag. Small numbers fit in one byte; large numbers use more.
This keeps the WASM binary format compact.

```
Byte layout:
  bit 7 (MSB): continuation flag  — 1 = more bytes follow
  bits 0–6   : 7 bits of payload data
```

### Encoding example: 624485 (unsigned)

```
624485 = 0b10011000011101100101
Split into 7-bit groups (LSB first):
  group 0: 1100101 = 0x65  → with flag: 0xE5
  group 1: 0001110 = 0x0E  → with flag: 0x8E
  group 2: 0100110 = 0x26  → last byte: 0x26
Result: <<0xE5, 0x8E, 0x26>>
```

## Where Does This Fit?

This module is part of the coding-adventures monorepo — a ground-up
implementation of the computing stack from transistors to operating systems.
It provides the integer encoding primitives needed by a WASM binary parser.

## API

```elixir
alias CodingAdventures.WasmLeb128

# Decode unsigned (returns {:ok, {value, bytes_consumed}} or {:error, msg})
{:ok, {624485, 3}} = WasmLeb128.decode_unsigned(<<0xE5, 0x8E, 0x26>>)

# Decode signed (sign-extends the final byte for negative numbers)
{:ok, {-2, 1}} = WasmLeb128.decode_signed(<<0x7E>>)

# Decode at a non-zero offset
{:ok, {624485, 3}} = WasmLeb128.decode_unsigned(<<0, 0, 0xE5, 0x8E, 0x26>>, 2)

# Encode unsigned
<<0xE5, 0x8E, 0x26>> = WasmLeb128.encode_unsigned(624485)

# Encode signed
<<0x7E>> = WasmLeb128.encode_signed(-2)
<<0x80, 0x80, 0x80, 0x80, 0x78>> = WasmLeb128.encode_signed(-2147483648)
```

## Error Handling

Functions return `{:error, message}` rather than raising:

```elixir
{:error, msg} = WasmLeb128.decode_unsigned(<<0x80, 0x80>>)
# msg: "unexpected end of data: LEB128 sequence is unterminated"
```

## Development

```bash
# Install deps and run tests
mix deps.get
mix test

# Via build script
bash BUILD
```

## Design Notes

- Uses Elixir binary pattern matching for byte-level decoding — idiomatic and fast.
- Arbitrary-precision integers handle sign extension naturally: `-(1 <<< shift)`
  gives a mask with all high bits set, then we normalize to signed 64-bit.
- `Integer.floor_div/2` provides arithmetic right shift semantics for negative
  integers (equivalent to `>>>` / `asr` in languages with fixed-width integers).
- All code uses Knuth-style literate programming: algorithm walkthroughs and
  examples live inline with the source.
