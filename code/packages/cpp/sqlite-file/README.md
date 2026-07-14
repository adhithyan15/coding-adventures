# sqlite-file (C++)

A **zero-dependency reader for the SQLite on-disk format** — header-only, ISO
C++17. A faithful port of the Rust [`sqlite-file`](../../rust/sqlite-file)
crate, in namespace `ca::sqlite_file`.

## What it does

The record layer also encodes typed values into byte-compatible SQLite records,
providing the Phase F writer groundwork exposed by the Rust source package.

Decodes the subset of the [SQLite file format](https://www.sqlite.org/fileformat2.html)
needed to read table rows straight out of a database's bytes — no external
SQLite library, no FFI, no I/O. You hand it a `std::vector<std::uint8_t>` (e.g.
the `collection.anki2` unpacked from an Anki `.apkg`) and it walks the b-trees.

Layers, leaf-to-root: **varint** → **record** (`Value` = `std::variant` of
Null/Int/Real/Text/Blob) → **header** → **pager** (zero-copy page borrow) →
**btree** (table + index walks, overflow reassembly, cycle + amplification
guards) → **schema** (table-by-name reads).

Every input is untrusted: a corrupt or hostile file throws a `SqliteError`
(never an out-of-bounds read or unbounded loop).

## API

```cpp
#include "sqlite_file.hpp"
namespace sf = ca::sqlite_file;

for (auto& [rowid, columns] : sf::read_table(db_bytes, "notes")) {
    for (const sf::Value& v : columns) {
        if (v.index() == sf::record::VText) { /* std::get<std::string>(v) */ }
    }
}
```

Also exposed: `varint::read`/`write`, `record::encode`, `record::decode` (returns
`std::optional`), `parse_header`, `Pager`, `walk_table`/`walk_index`,
`read_schema`, `table_root_page`, `read_without_rowid_table`. Where the Rust
crate returns `Result`, this port throws a `SqliteError` carrying an `Error`
code.

## Building

```sh
sh BUILD          # POSIX: g++ and/or clang++, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`. Verified clean under ASan + UBSan.
