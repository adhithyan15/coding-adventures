# cfb-writer (C)

A **Compound File Binary Format (CFB / OLE2) writer** in pure ISO C17. A faithful
port of the Rust [`cfb-writer`](../../rust/cfb-writer) crate. You hand it named
streams; it produces a byte buffer a conforming CFB reader — and real Office
tooling — accepts. CFB is the container inside legacy `.xls` / `.doc` / `.ppt`
files.

## Mental model

A CFB file is a FAT filesystem crammed into one file: a fixed 512-byte header,
then equal 512-byte sectors. A **File Allocation Table** holds one "next sector"
`u32` per sector, so a multi-sector stream is a linked list ending in
`ENDOFCHAIN`. A **directory** (itself a FAT-stored stream of 128-byte entries)
names the objects. Streams smaller than the 4096-byte cutoff are packed into a
**mini-stream** of 64-byte mini-sectors chained by a parallel **mini-FAT**.

The output is version 3 and fully **deterministic** — CLSIDs and timestamps are
zeroed — so the same input always yields identical bytes.

## API

```c
#include "cfb_writer.h"

/* Builder */
CfbWriter *w = cfb_writer_new();
cfb_writer_add_stream(w, "Workbook", data, data_len);       /* copies data */
size_t len;
uint8_t *bytes = cfb_writer_finish(w, &len);                /* consumes w */
free(bytes);

/* One-shot */
const char *names[] = {"A", "B"};
const uint8_t *datas[] = {a, b};
size_t lens[] = {alen, blen};
uint8_t *out = cfb_write(names, datas, lens, 2, &len);
free(out);
```

- `cfb_writer_new` / `cfb_writer_free` (only if not finished) /
  `cfb_writer_add_stream` (UTF-8 name → UTF-16LE, truncated to 31 units) /
  `cfb_writer_finish` (returns a malloc'd buffer, consumes the writer).
- `cfb_write` — the one-shot convenience.

Every allocation is checked and `size_t`-overflow-guarded; on failure the whole
build unwinds and returns NULL. Verified leak-free under ASan + UBSan, with the
output round-tripped through an in-test CFB reader.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
