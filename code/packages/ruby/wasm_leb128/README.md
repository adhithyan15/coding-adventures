# coding_adventures_wasm_leb128

LEB128 (Little-Endian Base 128) variable-length integer encoding for the
WebAssembly binary format. Part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo — a ground-up implementation of the computing stack from transistors
to operating systems.

## What is LEB128?

WebAssembly uses LEB128 encoding for every integer value in its binary
(`.wasm`) format: function indices, type indices, memory sizes, instruction
immediates, section lengths, and more.

The idea: instead of always storing a 32-bit integer in 4 bytes, use a
variable-length encoding where small values take 1 byte and large values take
up to 5 bytes. Each byte contributes 7 bits of data; the 8th (high) bit is
a continuation flag:

```
bit 7 = 1  →  more bytes follow
bit 7 = 0  →  this is the last byte
```

### Example: encoding 624485

```
624485 = 0b0001_0011_0000_0111_0110_0101

7-bit groups (little-endian):
  bits 0–6:   0b110_0101 = 0x65 → byte 0: 0xE5  (continuation bit set)
  bits 7–13:  0b000_1110 = 0x0E → byte 1: 0x8E  (continuation bit set)
  bits 14–19: 0b010_0110 = 0x26 → byte 2: 0x26  (last byte, no continuation)

Encoded: "\xE5\x8E\x26"
```

## Installation

Add to your `Gemfile`:

```ruby
gem "coding_adventures_wasm_leb128"
```

Or install directly:

```bash
gem install coding_adventures_wasm_leb128
```

## API

```ruby
require "coding_adventures_wasm_leb128"

LEB128 = CodingAdventures::WasmLeb128
```

### `decode_unsigned(data, offset=0)`

Decode a ULEB128-encoded unsigned integer. Accepts a binary String or Array
of byte integers. Returns `[value, bytes_consumed]`.

```ruby
LEB128.decode_unsigned("\xE5\x8E\x26".b)         # => [624485, 3]
LEB128.decode_unsigned([0xE5, 0x8E, 0x26])        # => [624485, 3]

# With offset — read from position 1:
LEB128.decode_unsigned("\xAA\xE5\x8E\x26".b, 1)  # => [624485, 3]
```

### `decode_signed(data, offset=0)`

Decode a SLEB128-encoded signed integer (two's complement sign extension).

```ruby
LEB128.decode_signed("\x7E".b)  # => [-2, 1]
LEB128.decode_signed("\x7F".b)  # => [-1, 1]
```

### `encode_unsigned(value)`

Encode a non-negative integer (u32 range) as ULEB128, returning a binary String.

```ruby
LEB128.encode_unsigned(624485)     # => "\xE5\x8E\x26"
LEB128.encode_unsigned(0)          # => "\x00"
LEB128.encode_unsigned(4294967295) # => "\xFF\xFF\xFF\xFF\x0F"
```

### `encode_signed(value)`

Encode a signed integer (i32 range) as SLEB128, returning a binary String.

```ruby
LEB128.encode_signed(-2)          # => "\x7E"
LEB128.encode_signed(2147483647)  # => "\xFF\xFF\xFF\xFF\x07"
LEB128.encode_signed(-2147483648) # => "\x80\x80\x80\x80\x78"
```

### `LEB128Error`

Raised when decoding encounters:
- An unterminated sequence (all bytes have continuation bit = 1)
- A value exceeding the 32-bit range (more than 5 bytes)

```ruby
begin
  LEB128.decode_unsigned("\x80\x80".b)  # unterminated!
rescue CodingAdventures::WasmLeb128::LEB128Error => e
  puts "Invalid LEB128: #{e.message}"
end
```

## Limitations

This gem handles **u32 and i32 ranges only** (values encoded in at most 5
LEB128 bytes). WebAssembly also uses i64/u64 values in some contexts. Ruby's
arbitrary-precision integers handle those naturally, but this package does not
implement them to stay focused on WASM 1.0's primary use cases.

## Where LEB128 fits in the stack

```
WebAssembly Binary Format (.wasm)
  └─ Sections: type, import, function, code, data, ...
       └─ All integer values encoded as LEB128
            └─ This gem ← you are here
```

## Development

```bash
bundle install
bundle exec rake test
```

```bash
# Run tests
bash BUILD
```
