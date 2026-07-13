# cfb (C)

A **reader for the OLE2 / Compound File Binary Format** ([MS-CFB]) in pure ISO
C17 — the container inside legacy `.xls`, `.doc`, and `.ppt` files. A faithful
port of the Rust [`cfb`](../../rust/cfb) crate, and the read counterpart to the
ported [`cfb-writer`](../cfb-writer).

## Mental model

A CFB file is a FAT filesystem crammed into one file: chopped into fixed-size
sectors (512 or 4096 bytes), with a File Allocation Table chaining multi-sector
streams like a linked list. A directory (itself a FAT-stored stream) names the
objects — a *stream* is a file, a *storage* a folder. Tiny streams live packed
in a mini-stream chained by a parallel mini-FAT.

## Hostile input

CFB files arrive as email attachments, so the reader assumes hostility: every
sector-chain walk is cycle-guarded (bounded by the FAT slot count — a valid
chain never revisits a sector), every offset is bounds-checked with
overflow-safe arithmetic, and assembled output is capped at 256 MiB. Malformed
input yields an error, never an out-of-bounds access or a hang.

## API

```c
#include "cfb.h"

CompoundFile *cf = NULL;
if (cfb_open(bytes, len, &cf) == CFB_OK) {
    for (size_t i = 0; i < cfb_entry_count(cf); i++) {
        const CfbEntry *e = cfb_entry(cf, i); /* e->name, e->size, e->kind */
    }
    uint8_t *data; size_t dlen;
    if (cfb_read_stream(cf, "Workbook", &data, &dlen)) { /* … */ free(data); }
    cfb_free(cf);
}
```

- `cfb_open` (owned `CompoundFile`) / `cfb_free`; `cfb_sector_size`;
  `cfb_entry_count` / `cfb_entry`; `cfb_read_stream` (by name,
  ASCII case-insensitive; caller frees `*out_data`) and
  `cfb_read_stream_by_id`.

Every growable buffer guards `size_t` overflow. Verified clean under ASan +
UBSan, the macOS `leaks` tool (0 leaks), a truncation fuzz (every prefix of a
valid file), and a 200k-iteration random byte-flip fuzz.

### Divergences from the Rust

- Stream/entry names decode UTF-16 LE → UTF-8 into fixed 128-byte buffers.
- `CfbError` drops the sector size that the Rust `UnsupportedSectorSize` variant
  carries. Case-insensitive matching is ASCII-only (CFB names are ASCII).

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`. Tests craft in-memory CFB files (no
external fixture needed).
