# sqlite-file (C)

A **zero-dependency reader for the SQLite on-disk format** — pure ISO C17. A
faithful port of the Rust [`sqlite-file`](../../rust/sqlite-file) crate.

## What it does

Decodes the subset of the [SQLite file format](https://www.sqlite.org/fileformat2.html)
needed to read table rows straight out of a database's bytes — no external
SQLite library, no FFI, no I/O. You hand it a byte buffer (e.g. the
`collection.anki2` unpacked from an Anki `.apkg`) and it walks the b-trees.

Layers, leaf-to-root:

1. **varint** — the 1–9 byte big-endian base-128 integer used everywhere.
2. **record** — decode a row's bytes into typed `sf_value_t`s (Null / Int /
   Real / Text / Blob).
3. **header** — parse the 100-byte database header (page size, encoding, …).
4. **pager** — borrow page N's bytes out of the buffer (1-based, zero-copy).
5. **btree** — walk a table/index b-tree, reassembling overflow chains and
   guarding against page cycles and cell-aliasing amplification DoS.
6. **schema** — resolve a table name to its root page and read it.

Every input is untrusted: a corrupt or hostile file yields a clean `sf_error_t`
(never a panic, out-of-bounds read, or unbounded loop).

## API

```c
#include "sqlite_file.h"

sf_named_rows_t rows;
if (sf_read_table(db_bytes, db_len, "notes", &rows) == SF_OK) {
    for (size_t i = 0; i < rows.len; ++i) {
        int64_t rowid = rows.rows[i].rowid;
        sf_row_t *cols = &rows.rows[i].columns;
        /* cols->items[k] is an sf_value_t */
    }
    sf_named_rows_free(&rows);
}
```

Also exposed: `sf_varint_read`/`write`, `sf_record_decode`, `sf_header_parse`,
`sf_pager_open`/`sf_pager_page`, `sf_walk_table`/`sf_walk_index`,
`sf_read_schema`, `sf_table_root_page`, `sf_read_without_rowid_table`. Decoded
results are malloc-owned and freed with the matching `sf_*_free` routine.

## Building

```sh
sh BUILD          # POSIX: gcc and/or clang, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`. Verified clean under ASan + UBSan
and macOS `leaks`.
