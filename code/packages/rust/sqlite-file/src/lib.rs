//! # sqlite-file — a zero-dependency reader for the SQLite on-disk format
//!
//! Engram imports Anki `.apkg` decks. Inside an `.apkg` is a `collection.anki2`
//! (or a serialized `.anki21` blob) — a **real SQLite database** — and Engram
//! needs to read a handful of tables out of it (`col`, `notes`, `cards`,
//! `revlog`, `graves`). Historically that meant linking the third-party
//! `rusqlite` crate, which bundles the entire C SQLite library and an `unsafe`
//! FFI boundary.
//!
//! This crate replaces that read path with a small, pure-Rust, **zero-dependency**
//! decoder for exactly the subset of the [SQLite file format][fmt] Engram reads:
//! it parses the bytes directly. It is part of the Engram zero-dependency
//! program (`code/specs/engram-zero-dep-plan.md`, Phase E); the byte layout it
//! tracks is documented in `code/specs/storage-sqlite.md`.
//!
//! [fmt]: https://www.sqlite.org/fileformat2.html
//!
//! ## Scope
//!
//! Read-only, and only the format features Engram's collections actually use:
//! table b-trees (no index b-trees), overflow chains, the standard 4 KiB-and-up
//! page sizes, UTF-8 text. Writing a database is a separate, larger effort
//! (Phase F) and lives elsewhere.
//!
//! ## Build order (this crate grows leaf-to-root)
//!
//! 1. **`varint`** — the 1–9 byte integer encoding used everywhere. *(done)*
//! 2. **`record`** — decode a row's bytes into typed [`SqlValue`]s. *(done)*
//! 3. **`header` + `pager`** — parse the 100-byte DB header; borrow pages from
//!    `&[u8]` (read-only, zero-copy, no journal/cache). *(done)*
//! 4. **`btree`** — walk a table b-tree (leaf + interior pages) → `(rowid, record
//!    bytes)`, reassembling overflow chains for records too big for one page.
//!    *(done)*
//! 5. **`sqlite_schema` + `read_table(bytes, name)`** - the public read API.
//!    *(done)*
//!
//! Each layer is cross-checked against the real `rusqlite`/C-SQLite as an
//! independent oracle (a dev-dependency, never a runtime one) before the Anki
//! importer is cut over to this crate.
//!
//! ## Example
//!
//! ```
//! use sqlite_file::record::{decode, SqlValue};
//!
//! // The on-disk bytes of the row `(NULL, 42, "hi")`.
//! let row = [0x04, 0x00, 0x01, 0x11, 0x2a, 0x68, 0x69];
//! assert_eq!(
//!     decode(&row).unwrap(),
//!     vec![SqlValue::Null, SqlValue::Int(42), SqlValue::Text("hi".into())],
//! );
//! ```

#![forbid(unsafe_code)]

pub mod btree;
pub mod error;
pub mod header;
pub mod pager;
pub mod record;
pub mod schema;
pub mod varint;

pub use error::SqliteError;
pub use header::{Header, TextEncoding};
pub use pager::Pager;
pub use record::SqlValue;
pub use schema::{read_schema, read_table, table_root_page, SchemaEntry};
