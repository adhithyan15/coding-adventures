# zip (C++)

A pure ISO **C++17**, header-only implementation of the **ZIP** archive
format (PKZIP, Phil Katz / Gary Conway, 1989), in namespace `ca::zip`. A
faithful port of the Rust `zip` crate (CMP09).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4
/WX` on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard
library only — no third-party dependencies.

## What it is

ZIP bundles one or more files into a single archive, compressing each entry
independently with **DEFLATE** (method 8, CMP05) or storing it verbatim
(method 0). The same format underlies Java JARs, Office Open XML
(`.docx`/`.xlsx`/`.pptx`), Android APKs, Python wheels, `.epub`, and many
more. This package is a straight CONTAINER around the sibling
[`deflate`](../deflate/) package — CMP09 does no entropy coding or
match-finding of its own, it only frames DEFLATE streams with headers, a
Central Directory, and a CRC-32 integrity check.

```
Series:
  CMP00 (LZ77,     1977) — Sliding-window backreferences.
  CMP01 (LZ78,     1978) — Explicit dictionary (trie).
  CMP02 (LZSS,     1982) — LZ77 + flag bits.
  CMP03 (LZW,      1984) — LZ78 + pre-initialized alphabet; GIF.
  CMP04 (Huffman,  1952) — Entropy coding.
  CMP05 (DEFLATE,  1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
  CMP09 (ZIP,      1989) — DEFLATE container; universal archive.  ← this package
```

### Dual-header design

```
┌─────────────────────────────────────────────────────┐
│  [Local File Header + File Data]  ← entry 1          │
│  [Local File Header + File Data]  ← entry 2          │
│  ...                                                 │
│  ══════════ Central Directory ══════════             │
│  [Central Dir Header]  ← entry 1 (has local offset)  │
│  [Central Dir Header]  ← entry 2                     │
│  [End of Central Directory Record]                   │
└─────────────────────────────────────────────────────┘
```

A **Local File Header** precedes each entry's data (streaming-write
friendly); a **Central Directory** at the end lists every entry with its
metadata and a byte offset back to its Local Header (random-access-read
friendly). Reading seeks to the **End of Central Directory** (EOCD) record,
scanned backward from the end of the file, then reads the whole Central
Directory before touching any entry's data.

### DEFLATE inside ZIP

Method 8 stores **raw RFC 1951 DEFLATE** — no zlib wrapper (no CMF/FLG
header, no Adler-32 checksum). `ZipWriter` calls the sibling
`ca::deflate::compress`; `ZipReader` calls `ca::deflate::inflate`, which
decodes **all three** RFC 1951 block types (stored, fixed, dynamic Huffman)
— so it opens archives this package never wrote. `zip`(1), Python's
`zipfile`, and Microsoft Office all emit dynamic Huffman routinely; this
package's test suite includes a real dynamic-Huffman fixture (produced
independently by CPython's `zipfile`) to prove that path actually works.

> **Note on a sibling lesson:** when `dart/zip` was implemented, `dart/deflate`
> turned out to have a private, non-standard wire format, forcing `dart/zip`
> to self-contain its own DEFLATE codec (see `lessons.md`, "dart/deflate").
> That does **not** apply here — `cpp/deflate` was independently verified
> against real `zlib` dynamic-Huffman output, so `cpp/zip` depends on it
> directly, with no reimplementation.

## API

```cpp
#include "zip.hpp"
namespace zip = ca::zip;
using Bytes = std::vector<std::uint8_t>;

// Write
zip::ZipWriter w;
w.add_file("hello.txt", Bytes{'h','i'}, /*compress=*/true);
w.add_directory("mydir/");
Bytes archive = w.finish();                 // never fails

// Convenience write
Bytes archive2 = zip::zip({{"hello.txt", Bytes{'h','i'}}});

// Read
zip::ZipReader reader(archive);             // throws ZipException on malformed input
for (const auto& entry : reader.entries()) {
    Bytes content = reader.read(entry);     // throws on CRC mismatch / bad method / ...
}
Bytes one = reader.read_by_name("hello.txt");

// Convenience read
auto files = zip::unzip(archive);           // vector<pair<name, data>>, directories skipped
```

- `ZipWriter::add_file(name, data, compress=true)` — DEFLATE is attempted;
  the compressed form is kept only if strictly smaller than the original,
  otherwise the entry falls back to Stored (method 0). Throws `ZipException`
  (`NameTooLong` / `DataTooLarge`) if `name` exceeds 65535 bytes or `data`
  exceeds 4 GiB — this format's non-ZIP64 field widths — rather than
  silently truncating into a corrupt archive.
- `ZipWriter::add_directory(name)` — `name` should end with `/`; that
  trailing slash is how readers recognize a directory (there is no separate
  on-wire flag).
- `ZipWriter::finish()` — appends the Central Directory + EOCD and returns
  the full archive.
- `ZipReader(const Bytes&)` — **borrows** the buffer (stored by reference,
  like the Rust reference's `ZipReader<'a>`); the caller must keep it alive
  for the reader's lifetime. Throws `ZipException` if no EOCD is found or
  the archive is structurally malformed.
- `ZipReader::entries()` — all entries (files and directories) in Central
  Directory order.
- `ZipReader::read(entry)` / `read_by_name(name)` — decompress + verify
  CRC-32; throw `ZipException` on CRC mismatch, an encrypted entry, an
  unsupported method, or a malformed DEFLATE stream.
- `zip::zip(entries)` / `zip::unzip(data, max_total_uncompressed_bytes=256MB)`
  — one-shot convenience wrappers.

```cpp
try {
    auto files = zip::unzip(untrusted_bytes);
} catch (const zip::ZipException& e) {
    // e.error() is a ca::zip::ZipError
}
```

## Robustness

- **Bounds-checked reads**: every multi-byte field pulled from untrusted
  archive bytes goes through a checked little-endian reader; nothing indexes
  raw pointers or trusts a length field without validating it against the
  buffer size first. `detail::read_u16`/`read_u32` take a `uint64_t` offset
  (not `size_t`) specifically so that CALLERS computing `base + N` for an
  untrusted, `uint32_t`-derived `base` (e.g. a Central Directory
  `local_offset`) are forced to do that addition in a width that cannot wrap
  on a hypothetical 32-bit `size_t` platform — `ZipReader::read` widens
  `entry.local_offset` to `uint64_t` exactly once and adds every fixed field
  offset against that value, never against a `size_t` copy of it.
- **Overflow-safe offset arithmetic**: combining two untrusted `u32` fields
  (Central Directory offset + size; Local Header data start + compressed
  size) is done in `uint64_t` so the sum cannot silently wrap before the
  bounds check runs.
- **Two-layer decompression-bomb guard**: each single DEFLATE entry is
  capped by `ca::deflate::MAX_INFLATE_OUTPUT` (256 MB) inside `inflate`
  itself; `zip::unzip()` additionally enforces a configurable **aggregate**
  budget (default 256 MB) across every entry it decompresses, plus a hard
  cap of 65535 parsed Central Directory entries. The aggregate check trusts
  the Central Directory's declared `Uncompressed_Size` field for its
  pre-decompression early-out — which is safe against an entry that
  UNDERSTATES its real cost specifically because `ZipReader::read` (see
  next bullet) throws rather than returning a result if the actual
  decompressed size ever disagrees with the declared one.
- **Declared vs. actual size is enforced, not merely trusted**:
  `ZipReader::read` throws `ZipException(DeclaredSizeMismatch)` if the
  ACTUAL decompressed byte count differs from the Central Directory's
  declared `Uncompressed_Size`, rather than silently trimming an oversized
  result down to the declared length. This matters beyond simple
  correctness: `Uncompressed_Size` is an attacker-controlled field, so a
  crafted entry could otherwise declare `Uncompressed_Size = 0` while its
  real DEFLATE stream still cost up to 256 MB of genuine decompression work
  per entry — and a silently-trimmed 0-byte result would make every
  size-based aggregate budget believe nothing had been decompressed at all,
  letting many such entries each smuggle real work past it.
- **Oversized-write rejection**: `ZipWriter::add_file`/`add_directory` throw
  `ZipException` (`NameTooLong` / `DataTooLarge`) instead of silently
  truncating a name over 65535 bytes or data over 4 GiB into the format's
  16-bit/32-bit fields — a silent truncation there would produce a
  structurally-corrupt archive whose declared length no longer matches its
  actual bytes. The same protection applies to two cumulative-archive cases:
  every Local Header offset and the Central Directory's own offset/size are
  narrowed through a checked helper (`ArchiveTooLarge`) rather than an
  unchecked `static_cast<uint32_t>`, so writing enough entries to push the
  total archive past 4 GiB fails loudly instead of wrapping every later
  entry's recorded offset; and `finish()` itself rejects (`TooManyEntries`)
  more than 65535 entries rather than letting the EOCD's 16-bit entry-count
  fields wrap (e.g. a real 65536-entry archive silently declaring 0).
- **Central Directory is authoritative**: sizes and compression method come
  from the CD, not the Local Header (which is only consulted for its own
  name/extra-field lengths, to find the data offset).
- **Encrypted entries rejected**: General-Purpose flag bit 0 set on the
  Local Header → `ZipException(EncryptedEntryUnsupported)`, not an attempt to
  decompress ciphertext.
- **Bounded EOCD scan**: the backward scan for the EOCD signature is capped
  to the last `22 + 65535` bytes (the maximum legal comment length) — it
  never searches an arbitrarily large file unboundedly.
- **Zip-slip / path traversal does not apply**: this package is in-memory
  only — `ZipReader`/`unzip()` never write to disk, so a malicious entry
  name (`"../../etc/passwd"`, `"/etc/passwd"`) is just a `std::string` key in
  the returned data. A caller building disk extraction on top of
  `ZipEntry::name` is responsible for sanitizing it first; this package
  deliberately provides no such function.

## Dependency

The sibling [`deflate`](../deflate/) package (CMP05) supplies
`ca::deflate::compress`/`inflate` for method-8 entries; `deflate` itself
depends on [`lzss`](../lzss/) (CMP02), which is why the build's include path
needs both. See `BUILD`.

## Building & testing

```sh
sh tools/run.sh    # POSIX: compiles + runs the tests under every compiler found
```

Tests cover all 12 mandatory test cases from `code/specs/CMP09-zip.md`
(Stored/DEFLATE round-trip, multi-file, directory entries, CRC-32 corruption
detection, EOCD/random-access reading, incompressible-data→Stored fallback,
empty file, 100 KB compression, CLI interop, Unicode filenames, nested
paths), plus:

- A **real** CLI-interop round-trip against the system `zip`/`unzip` tools
  (shelled out via `std::system`) in both directions — this package writes
  and the system tool reads, and vice versa. Skips gracefully (does not
  fail) when those tools are not on `PATH`.
- A **real** dynamic-Huffman ZIP entry produced independently by CPython's
  `zipfile` module, proving `ZipReader` reads dynamic Huffman it never wrote
  itself — the exact case a fixed-Huffman-only decoder would reject.
- Negative/malformed-input cases: no EOCD, CRC mismatch, encrypted entry,
  unsupported compression method, aggregate decompression-bomb budget.
