# CodingAdventures::Zip

ZIP archive format (PKZIP 1989) implemented from scratch in Perl 5.26+ — **CMP09** in the compression series.

## What it does

Creates and reads `.zip` files byte-compatible with standard ZIP tools (macOS Archive Utility, Info-ZIP, Python's `zipfile`, etc.). Each entry is compressed with RFC 1951 DEFLATE (method 8) or stored verbatim (method 0) if compression doesn't help.

The package also exposes its ZIP-owned raw RFC 1951 codec directly. The strict
decoder accepts stored, fixed-Huffman, dynamic-Huffman, and multi-block streams,
reports exact byte consumption, enforces a caller-lowerable 256 MiB output cap,
and returns one of 14 stable payload-blind error codes without partial output.

## Where it fits

```
CMP02 (LZSS,    1982) — LZ77 + flag bits       ← dependency
CMP05 (DEFLATE, 1996) — LZ77 + Huffman         ← inlined here (raw RFC 1951)
CMP09 (ZIP,     1989) — DEFLATE container      ← this package
```

## Installation

```bash
cpanm --installdeps .
```

## Usage

### Create an archive

```perl
use CodingAdventures::Zip qw(zip new_writer add_file add_directory finish);

# One-shot
my $archive = zip([ ["hello.txt", "Hello, ZIP!"], ["data.bin", "\x01\x02\x03"] ]);

# Full control
my $w = new_writer();
add_directory($w, "docs/");
add_file($w, "docs/readme.txt", "Read me");
my $bytes = finish($w);
```

### Read an archive

```perl
use CodingAdventures::Zip qw(unzip new_reader reader_entries read_by_name);

# One-shot
my $files = unzip($archive);
print $files->{"hello.txt"};  # Hello, ZIP!

# Fine-grained
my $reader = new_reader($archive);
for my $e (@{reader_entries($reader)}) {
    printf "%s %d\n", $e->{name}, $e->{size};
}
my $data = read_by_name($reader, "hello.txt");
```

### CRC-32

```perl
use CodingAdventures::Zip qw(crc32);
printf "%08X\n", crc32("hello world");  # 0D4A1185
```

### Raw RFC 1951

```perl
use CodingAdventures::Zip qw(
    raw_deflate raw_inflate raw_inflate_counted RAW_INFLATE_MAX_OUTPUT
);

my $compressed = raw_deflate("hello" x 100);
my $plain = raw_inflate($compressed, 4096);
my $result = raw_inflate_counted($compressed, RAW_INFLATE_MAX_OUTPUT);
print $result->bytes_consumed;
```

These bytes have no ZIP, zlib, or gzip framing. `raw_inflate_counted` counts
through the partially consumed final byte and excludes whole trailing bytes,
allowing ZIP readers to reject covert payload suffixes. CRC-32 detects accidental
corruption; it is not authentication or a cryptographic integrity check.

## API

| Function | Description |
|----------|-------------|
| `new_writer()` | Creates a new ZipWriter hashref. |
| `add_file($w, $name, $data, $compress)` | Add a file entry. `$compress` defaults to 1. |
| `add_directory($w, $name)` | Add a directory entry. |
| `finish($w)` | Return completed archive as a binary string. |
| `new_reader($data)` | Parse a ZIP archive binary string. Dies on error. |
| `reader_entries($r)` | Return arrayref of entry hashrefs. |
| `reader_read($r, $entry)` | Decompress and CRC-validate an entry. Dies on error. |
| `read_by_name($r, $name)` | Convenience wrapper. Dies if not found. |
| `zip($entries, $compress)` | One-shot compress. |
| `unzip($data)` | One-shot decompress → hashref of name → data. |
| `crc32($data, $initial)` | CRC-32 (polynomial 0xEDB88320). |
| `raw_deflate($data)` | Encode raw RFC 1951 bytes using stored/fixed blocks. |
| `raw_inflate($data, $max_output)` | Strictly decode raw RFC 1951 bytes. |
| `raw_inflate_counted($data, $max_output)` | Decode and return output plus exact bytes consumed. |
| `RAW_INFLATE_MAX_OUTPUT` | Hard 256 MiB decoder ceiling. |
| `dos_datetime($y,$m,$d,$h,$min,$s)` | MS-DOS timestamp encoder. |
| `dos_epoch()` | Returns `0x00210000` — 1980-01-01 00:00:00. |

## Running tests

```bash
PERL5LIB=$(cd ../lzss && pwd)/lib:${PERL5LIB:-} prove -l -v t/
```
