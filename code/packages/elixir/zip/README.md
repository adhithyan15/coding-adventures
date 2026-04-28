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
| `dos_datetime/6` | MS-DOS timestamp. |
| `dos_epoch/0` | `0x00210000` — 1980-01-01 00:00:00. |

## Running tests

```bash
mix deps.get
mix test --cover
```
