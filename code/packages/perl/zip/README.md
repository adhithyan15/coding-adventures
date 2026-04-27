# CodingAdventures::Zip

ZIP archive format (PKZIP 1989) implemented from scratch in Perl 5.26+ — **CMP09** in the compression series.

## What it does

Creates and reads `.zip` files byte-compatible with standard ZIP tools (macOS Archive Utility, Info-ZIP, Python's `zipfile`, etc.). Each entry is compressed with RFC 1951 DEFLATE (method 8) or stored verbatim (method 0) if compression doesn't help.

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
| `dos_datetime($y,$m,$d,$h,$min,$s)` | MS-DOS timestamp encoder. |
| `dos_epoch()` | Returns `0x00210000` — 1980-01-01 00:00:00. |

## Running tests

```bash
PERL5LIB=$(cd ../lzss && pwd)/lib:${PERL5LIB:-} prove -l -v t/
```
