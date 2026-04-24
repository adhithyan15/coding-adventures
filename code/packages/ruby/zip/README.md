# coding_adventures_zip

ZIP archive format (PKZIP 1989) implemented from scratch in Ruby — **CMP09** in the compression series.

## What it does

Creates and reads `.zip` files byte-compatible with standard ZIP tools (macOS Archive Utility, Info-ZIP, Python's `zipfile`, etc.). Each entry is compressed with RFC 1951 DEFLATE (method 8) or stored verbatim (method 0) if compression doesn't help.

## Where it fits

```
CMP02 (LZSS,    1982) — LZ77 + flag bits       ← dependency
CMP05 (DEFLATE, 1996) — LZ77 + Huffman         ← inlined here (raw RFC 1951)
CMP09 (ZIP,     1989) — DEFLATE container      ← this gem
```

## Installation

```ruby
gem "coding_adventures_zip"
```

## Usage

### Create an archive

```ruby
require "coding_adventures_zip"

# One-shot
archive = CodingAdventures::Zip.zip([
  ["hello.txt", "Hello, ZIP!"],
  ["data.bin",  "\x01\x02\x03"],
])

# Full control
w = CodingAdventures::Zip::ZipWriter.new
w.add_directory("docs/")
w.add_file("docs/readme.txt", "Read me")
zip = w.finish
```

### Read an archive

```ruby
# One-shot
files = CodingAdventures::Zip.unzip(archive)
puts files["hello.txt"]   # => "Hello, ZIP!"

# Fine-grained
reader = CodingAdventures::Zip::ZipReader.new(archive)
reader.entries.each { |e| puts "#{e.name} #{e.size}" }
data = reader.read_by_name("hello.txt")
```

### CRC-32

```ruby
CodingAdventures::Zip.crc32("hello world")  # => 0x0D4A1185
```

## API

| Symbol | Description |
|--------|-------------|
| `ZipWriter` | Builds archives in memory. |
| `ZipWriter#add_file(name, data, compress: true)` | Add a file entry. |
| `ZipWriter#add_directory(name)` | Add a directory entry. |
| `ZipWriter#finish` | Return completed archive as binary String. |
| `ZipReader` | Parses ZIP archives. |
| `ZipReader#entries` | List all `ZipEntry` structs. |
| `ZipReader#read(entry)` | Decompress and return entry data. |
| `ZipReader#read_by_name(name)` | Convenience wrapper. |
| `Zip.zip(entries, compress: true)` | One-shot compress. |
| `Zip.unzip(data)` | One-shot decompress → Hash. |
| `Zip.crc32(data, initial: 0)` | CRC-32 (polynomial 0xEDB88320). |
| `Zip.dos_datetime(...)` | MS-DOS timestamp. |
| `Zip::DOS_EPOCH` | `0x00210000` — 1980-01-01 00:00:00. |

## Running tests

```bash
bundle install
bundle exec rake test
```
