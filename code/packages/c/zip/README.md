# zip (C)

A pure ISO **C17** implementation of the **ZIP** archive format (PKZIP, Phil
Katz 1989; CMP09). It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). Standard
library only.

## What it is

ZIP bundles one or more files into a single archive, compressing each entry
INDEPENDENTLY with DEFLATE (method 8, RFC 1951) or storing it verbatim
(method 0). The same format underlies Java JARs, Office Open XML
(`.docx`/`.xlsx`/`.pptx`), Android APKs, Python wheels, and many more.

```
Series:
  CMP02 (LZSS,    1982) — LZ77 + flag bits.
  CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
  CMP09 (ZIP,     1989) — DEFLATE container; universal archive.  ← this package
```

A ZIP file has a dual-header design: **Local File Headers** are written
inline, one immediately before each entry's data, so a writer can stream
sequentially; a **Central Directory** at the end lists every entry with its
authoritative size/method/offset, so a reader can seek to the **End of
Central Directory Record (EOCD)** and jump straight to any entry without
scanning the whole file.

```
+---------------------------------------------------------+
|  [Local File Header + File Data]  <- entry 1             |
|  [Local File Header + File Data]  <- entry 2             |
|  ...                                                     |
|  ========== Central Directory ==========                 |
|  [Central Dir Header]  <- entry 1 (carries local offset)  |
|  [Central Dir Header]  <- entry 2                         |
|  [End of Central Directory Record (EOCD)]                 |
+---------------------------------------------------------+
```

## Dependency on `c/deflate` (a deliberate divergence from the other ports)

`code/specs/CMP09-zip.md` documents that most language ports of `zip`
implement RFC 1951 DEFLATE framing directly inside the `zip` package rather
than depending on that language's `deflate` package — several of those
`deflate` packages use a private, self-designed wire format for their own
round-tripping (see `dart/deflate`, and `lessons.md` Lesson 98), which would
produce archives no real `unzip` could open.

`c/deflate` is the exception: it was purpose-built and specifically verified
to be genuine RFC 1951 (including decoding a real `zlib`-produced
dynamic-Huffman stream), so `c/zip` depends on it directly —
`deflate_compress` for writing, `deflate_decompress` (all three RFC 1951
block types) for reading. This package never touches `c/lzss` directly.

## API

```c
#include "zip.h"

/* One-shot */
ZipFile entry = { "hello.txt", 9, (unsigned char *)"hello, world", 12 };
unsigned char *archive; size_t archive_len;
zip_bytes(&entry, 1, &archive, &archive_len);

ZipFile *files; size_t count;
zip_unzip(archive, archive_len, &files, &count);   /* files[0].data == "hello, world" */
zip_files_free(files, count);
free(archive);

/* Incremental writer */
ZipWriter *w;
zip_writer_new(&w);
zip_writer_add_file(w, "hello.txt", data, len, 1 /* try DEFLATE */);
zip_writer_add_directory(w, "mydir/");
unsigned char *out; size_t out_len;
zip_writer_finish(w, &out, &out_len);
zip_writer_free(w);

/* Random-access reader */
ZipReader *r;
zip_reader_new(archive, archive_len, &r);   /* BORROWS archive */
for (size_t i = 0; i < zip_reader_entry_count(r); i++) {
    const ZipEntry *e = zip_reader_entry(r, i);
    unsigned char *data; size_t data_len;
    zip_reader_read(r, e, &data, &data_len);  /* verifies CRC-32 */
    free(data);
}
zip_reader_free(r);
```

- All entry points return `ZipStatus` (`ZIP_OK` or one of seven error codes);
  every out-parameter buffer is malloc'd (free with `free()` or the matching
  `*_free` helper).
- `zip_writer_add_file(..., compress)`: DEFLATE is used only when it is
  strictly smaller than the original; otherwise the entry falls back to
  Stored (method 0) automatically.
- `ZipReader` **borrows** the archive buffer passed to `zip_reader_new` — it
  must stay valid and unmodified for the reader's whole lifetime.

## Security

This package treats reader input as untrusted, per
`code/specs/CMP09-zip.md`'s "Security Considerations":

- EOCD search is bounded to the last `22 + 65535` bytes (max comment
  length), never an unbounded scan.
- Every multi-byte field read is bounds-checked before use.
- Central Directory offset/size arithmetic is done in a 64-bit intermediate
  before narrowing to `size_t`, so an adversarial pair of 32-bit fields
  cannot wrap around and defeat a bounds check on a 32-bit `size_t` platform.
- Parsed Central Directory entries are hard-capped at `ZIP_MAX_ENTRIES`
  (65535, the largest count the non-ZIP64 format can declare).
- Per-entry decompressed size is bounded by `c/deflate`'s own 256 MiB cap
  (`DEFLATE_MAX_OUTPUT`). **In addition**, `ZipReader` tracks an *aggregate*
  decompressed-bytes budget across every `zip_reader_read` call made through
  it (default 256 MiB, `zip_reader_new_with_budget` to configure), so many
  small entries that individually look fine but sum to gigabytes are
  rejected too — not just a single oversized entry.
- Encrypted entries (General Purpose Bit Flag bit 0) are rejected with
  `ZIP_ERR_ENCRYPTED`; compression methods other than Stored/DEFLATE are
  rejected with `ZIP_ERR_UNSUPPORTED_METHOD`.
- **Zip slip / path traversal**: this package is purely in-memory and never
  writes to the filesystem, so path traversal cannot happen inside it.
  `ZipEntry`/`ZipFile` names are returned as plain byte strings; any caller
  that builds a "write entry to disk" helper on top of `zip_reader_read`
  must independently reject or sanitise names containing `..` components or
  a leading `/` before joining them to a destination directory.

## Building & testing

```sh
sh tools/run.sh    # POSIX: compiles + runs the tests under every compiler found
```

Tests (`tests/zip_test.c`) cover every TC-1..TC-12 from
`code/specs/CMP09-zip.md`, CRC-32 known values, `zip_dos_datetime`, a
real-world dynamic-Huffman fixture (proving the `c/deflate` dependency's
dynamic-Huffman decode path works through this package), and targeted
robustness tests for the security hardening above. TC-10 (CLI interoperability
with the system `zip`/`unzip`, in both directions) shells out via the
standard ISO C `system()` — not `popen`/`fork`+`exec` — so it stays pure
ISO C under the harness's `-pedantic-errors` on every platform including
MSVC; it skips gracefully (prints a `SKIP` line, no failed checks) when
Info-ZIP isn't on `PATH`.
