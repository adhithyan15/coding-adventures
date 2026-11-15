# coding_adventures_zip

ZIP archive format (PKZIP 1989) implemented from scratch in Elixir — **CMP09** in the compression series.

## What it does

Creates and reads `.zip` files byte-compatible with standard ZIP tools (macOS Archive Utility, Info-ZIP, Python's `zipfile`, etc.). Each entry is compressed with RFC 1951 DEFLATE (method 8) or stored verbatim (method 0) if compression doesn't help.

## Where it fits

```
CMP02 (LZSS,    1982) — LZ77 + flag bits       ← dependency
CMP05 (DEFLATE, 1996) — LZ77 + Huffman         ← inlined here (raw RFC 1951)
CMP09 (ZIP,     1989) — DEFLATE container      ← this package
```

## Installation

```elixir
{:coding_adventures_zip, path: "../zip"}
```

## Usage

### Create an archive

```elixir
# One-shot
archive = CodingAdventures.Zip.zip([
  {"hello.txt", "Hello, ZIP!"},
  {"data.bin",  <<1, 2, 3>>}
])

# Full control
w = CodingAdventures.Zip.new_writer()
w = CodingAdventures.Zip.add_directory(w, "docs/")
w = CodingAdventures.Zip.add_file(w, "docs/readme.txt", "Read me")
zip = CodingAdventures.Zip.finish(w)
```

### Read an archive

```elixir
# One-shot
files = CodingAdventures.Zip.unzip(archive)
IO.puts files["hello.txt"]  # => "Hello, ZIP!"

# Fine-grained
reader = CodingAdventures.Zip.new_reader(archive)
CodingAdventures.Zip.reader_entries(reader)
|> Enum.each(fn e -> IO.puts "#{e.name} #{e.size}" end)

data = CodingAdventures.Zip.read_by_name(reader, "hello.txt")
```

### CRC-32

```elixir
CodingAdventures.Zip.crc32("hello world")  # => 0x0D4A1185
```

### Raw RFC 1951

ZIP method 8 carries raw DEFLATE bytes without ZIP, zlib, or gzip framing. The
portable codec surface is available directly for sibling container codecs:

```elixir
compressed = CodingAdventures.Zip.raw_deflate("hello hello hello")
result = CodingAdventures.Zip.raw_inflate_counted(compressed)
result.output         # => "hello hello hello"
result.bytes_consumed # exact bytes through BFINAL
```

`raw_inflate/2` and `raw_inflate_counted/2` accept a caller-lowerable output
limit. The hard and default ceiling is 256 MiB. Decoding covers stored, fixed,
and dynamic Huffman blocks, rejects trailing compressed-payload cavities at
the ZIP boundary, returns no partial output, and exposes one of 14 stable,
payload-blind errors through `CodingAdventures.Zip.RawInflateError`.

### PNG-Scale Hardening (IC18)

The raw codec is engineered to handle the maximum IC18 PNG decompressed scanline ceiling of **134,234,112 bytes** (33,554,432 pixels × 4 bytes/pixel + 16,384 filter bytes) in bounded time and RAM:
- **Chunked Bit Accumulation**: `BitWriter` accumulates bits in 64-bit machine words using binary patterns (`<<word::64-little>>`) and flushes 64 KiB chunks, avoiding linked-list heap expansion.
- **Fast Pattern Matcher**: The DEFLATE encoder utilizes a byte-backed 3-byte hash table paired with Erlang's C-BIF `:binary.longest_common_prefix/1` for ultra-fast LZ77 tokenization.
- **Paged Output Buffer**: `raw_inflate` operates with a 4 KiB paged buffer map and slice-based zero-copy lookups (`:binary.copy/2` for RLE sequences and zero-copy stored block ingestion).
- **Compile-Time Lookup Tables**: Fixed Huffman literal/length, distance, and CRC-32 tables are precomputed at compile time into module constants.

## CVE Resilience Showcase

| Vulnerability Class | Relevant CVE | Mitigation in `CodingAdventures.Zip` |
|:---|:---|:---|
| **Zip-Bomb Amplification & Allocation Limits** | CVE-2002-1363, CVE-2023-4863 | Output limit is enforced eagerly before every stored block, literal, and back-reference copy. Exceeding limit raises `RawInflateError` with `output-limit-exceeded` in bounded memory with zero partial output leakage. |
| **Huffman Oversubscription & Incomplete Trees** | CVE-2016-9840, CVE-2008-1372 | Rejects oversubscribed or incomplete code length tables and dynamic Huffman trees (`huffman-oversubscribed`, `incomplete-code-length-tree`, `incomplete-literal-length-tree`, `incomplete-distance-tree`, `invalid-literal-length-symbol`). |
| **Sliding-Window Underflow & Wrap-Around Overflow** | Historical zlib/Info-ZIP issues | Full 32 KiB history window boundary enforcement: distance exceeding current buffer offset or 32,768 raises `invalid-back-reference`. Overlapping RLE pattern replication is handled safely via `:binary.copy/2`. |
| **Stored-Block NLEN Complement Tampering** | CVE-2005-1849 | RFC 1951 stored block `LEN ^ 0xFFFF == NLEN` strictly verified (`stored-length-mismatch`). Zero-copy payload ingestion bounded by remaining input. |
| **Trailing Payload Cavities & Premature BFINAL** | CVE-2010-1205 | `raw_inflate_counted` returns exact `bytes_consumed` through the BFINAL end-of-block marker. Archive extraction rejects compressed payloads with unconsumed trailing bytes (`zip: compressed payload contains trailing bytes`). |
| **Zip-Slip Directory Traversal & Absolute Paths** | CVE-2018-1002200 | Central directory and local file entry names validated on both write and read: paths cannot contain `..`, leading `/` or `\`, or Windows drive letters (`C:`), raising dedicated payload-blind exceptions. |
| **Payload Data Leakage in Exceptions** | Standing Rule #20 | 100% of error messages and diagnostic strings are isolated in `Messages.ex`. Zero untrusted file names, offsets, or byte snippets are interpolated into error strings. |

## Error Taxonomy

The package defines a closed taxonomy of 14 RFC 1951 error identifiers isolated in `Messages.ex`:

- `invalid-output-limit`
- `unexpected-eof`
- `reserved-block-type`
- `stored-length-mismatch`
- `huffman-oversubscribed`
- `incomplete-code-length-tree`
- `incomplete-literal-length-tree`
- `incomplete-distance-tree`
- `repeat-without-previous`
- `repeat-overrun`
- `invalid-literal-length-symbol`
- `reserved-distance-symbol`
- `invalid-back-reference`
- `output-limit-exceeded`

## API

| Function | Description |
|----------|-------------|
| `new_writer/0` | Creates a new ZipWriter map. |
| `add_file/4` | Add a file entry. `compress: true` by default. |
| `add_directory/2` | Add a directory entry. |
| `finish/1` | Return completed archive as binary. |
| `new_reader/1` | Parse a ZIP archive binary. |
| `reader_entries/1` | List all entry maps. |
| `reader_read/2` | Decompress and CRC-validate an entry. |
| `read_by_name/2` | Convenience wrapper. |
| `zip/2` | One-shot compress. |
| `unzip/1` | One-shot decompress → map. |
| `crc32/2` | CRC-32 (polynomial 0xEDB88320). |
| `raw_deflate/1` | Encode raw RFC 1951 bytes with the ZIP-owned fixed-Huffman encoder. |
| `raw_inflate/2` | Strictly decode stored, fixed, or dynamic raw RFC 1951 bytes. |
| `raw_inflate_counted/2` | Decode and report exact bytes consumed through BFINAL. |
| `raw_inflate_max_output/0` | Return the 256 MiB hard/default output ceiling. |
| `dos_datetime/6` | MS-DOS timestamp. |
| `dos_epoch/0` | `0x00210000` — 1980-01-01 00:00:00. |

## Running tests

```bash
mix deps.get
mix test --cover
```

Production is a pure in-memory transform with an explicit empty capability
profile. Fixture file reads, Jason, and Erlang `:zlib` are test-only. CRC-32
detects accidental corruption; it is not authentication.
