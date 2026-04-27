# CodingAdventures::Deflate (Perl)

**CMP05 — DEFLATE lossless compression (1996)**

## Usage

```perl
use CodingAdventures::Deflate qw(compress decompress);

my $data       = "hello hello hello world";
my $compressed = compress($data);
my $original   = decompress($compressed);
```

## Wire Format

```
[4B] original_length    big-endian uint32
[2B] ll_entry_count     big-endian uint16
[2B] dist_entry_count   big-endian uint16
[ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8)
[dist_entry_count × 3B] same format
[remaining bytes]       LSB-first packed bit stream
```
