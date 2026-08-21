# coding-adventures-zip

ZIP archive format (PKZIP 1989) implemented from scratch in Lua 5.4 — **CMP09** in the compression series.

## What it does

Creates and reads `.zip` files byte-compatible with standard ZIP tools (macOS Archive Utility, Info-ZIP, Python's `zipfile`, etc.). Each entry is compressed with RFC 1951 DEFLATE (method 8) or stored verbatim (method 0) if compression doesn't help.

## Where it fits

```
CMP02 (LZSS,    1982) — LZ77 + flag bits       ← dependency
CMP05 (DEFLATE, 1996) — LZ77 + Huffman         ← inlined here (raw RFC 1951)
CMP09 (ZIP,     1989) — DEFLATE container      ← this package
```

## Installation

```lua
-- via LuaRocks (local path install):
-- luarocks make --local coding-adventures-zip-0.2.0-1.rockspec
```

## Usage

### Create an archive

```lua
local zip = require("coding_adventures.zip")

-- One-shot
local archive = zip.zip({
    {"hello.txt", "Hello, ZIP!"},
    {"data.bin",  "\1\2\3"},
})

-- Full control
local w = zip.new_writer()
zip.add_directory(w, "docs/")
zip.add_file(w, "docs/readme.txt", "Read me")
local bytes = zip.finish(w)
```

### Read an archive

```lua
-- One-shot
local files = zip.unzip(archive)
print(files["hello.txt"])  -- => Hello, ZIP!

-- Fine-grained
local reader = zip.new_reader(archive)
for _, e in ipairs(zip.reader_entries(reader)) do
    print(e.name, e.size)
end

local data = zip.read_by_name(reader, "hello.txt")
```

### CRC-32

```lua
zip.crc32("hello world")  -- => 0x0D4A1185
```

### Raw RFC 1951

```lua
local encoded = zip.raw_deflate("hello hello")
local decoded = assert(zip.raw_inflate(encoded))
local counted = assert(zip.raw_inflate_counted(encoded, 1024))
print(counted.output, counted.bytes_consumed)
```

The decoder accepts stored, fixed-Huffman, dynamic-Huffman, and multi-block
streams. `raw_inflate_counted` reports the exact number of compressed bytes
consumed, allowing ZIP readers to reject hidden suffix cavities. Completed
output is retained in compact byte-string chunks with only one 4 KiB numeric
tail, avoiding boxed-number amplification at the public output ceiling.

## API

| Function | Description |
|----------|-------------|
| `new_writer()` | Creates a new ZipWriter table. |
| `add_file(w, name, data, compress)` | Add a file entry. `compress` defaults to `true`. |
| `add_directory(w, name)` | Add a directory entry. |
| `finish(w)` | Return completed archive as a binary string. |
| `new_reader(data)` | Parse a ZIP archive binary string. |
| `reader_entries(r)` | List all entry tables. |
| `reader_read(r, entry)` | Decompress and CRC-validate an entry. |
| `read_by_name(r, name)` | Convenience wrapper. |
| `zip(entries, compress)` | One-shot compress. |
| `unzip(data)` | One-shot decompress → table of name → data. |
| `crc32(data, initial)` | CRC-32 (polynomial 0xEDB88320). |
| `raw_deflate(data)` | Encode a raw RFC 1951 stream. |
| `raw_inflate(data, max_output)` | Decode a raw stream with a caller-lowerable output cap. |
| `raw_inflate_counted(data, max_output)` | Decode and return `output` plus exact `bytes_consumed`. |
| `RAW_INFLATE_MAX_OUTPUT` | Hard and default 256 MiB inflate ceiling. |
| `RAW_INFLATE_ERROR_CODES` | Stable payload-blind decoder error identifiers. |
| `dos_datetime(y,m,d,h,min,s)` | MS-DOS timestamp encoder. |
| `DOS_EPOCH` | `0x00210000` — 1980-01-01 00:00:00. |

## Security boundary

The package is a pure in-memory codec. It has no filesystem, network, process,
environment, clock, entropy, console, or credential authority. DEFLATE output
is capped at 256 MiB and ZIP method-8 reads lower that cap to the declared entry
size. Every stored byte, literal, and back-reference is checked before append;
errors expose no partial output or attacker-controlled values. Counted decoding
and exact declared-size checks fail closed before CRC validation. CRC-32 detects
accidental corruption only; it is not authentication.

## Running tests

```bash
luarocks make --local coding-adventures-zip-0.2.0-1.rockspec
cd tests && LUA_PATH="../src/?.lua;../src/?/init.lua;../../lzss/src/?.lua;../../lzss/src/?/init.lua;;" busted . --verbose --pattern=test_
```
