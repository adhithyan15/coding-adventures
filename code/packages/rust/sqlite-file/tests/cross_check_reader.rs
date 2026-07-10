//! Interop gate for the zero-dependency `sqlite-file` reader, measured against
//! the real bundled-C SQLite via `rusqlite` (a **dev-dependency only** — the
//! whole point of this crate is that nothing at runtime links it).
//!
//! The gate builds genuine `.sqlite` files with `rusqlite`, then asserts our
//! from-scratch decoder reads back exactly what SQLite wrote. It grows in
//! lockstep with the reader:
//!
//!   * **now (varint + record codec):** confirm the oracle is wired and that a
//!     real SQLite file matches the format constants the reader is built to
//!     assume (page size, text encoding, schema format). This is what pins the
//!     fixtures every later phase parses.
//!   * **with the b-tree walk (Phase E3):** locate each row's record bytes in
//!     the file and assert `record::decode` yields the same values `rusqlite`
//!     returns over SQL — the full round-trip gate. Staged below as an
//!     `#[ignore]`d placeholder so the intent is visible and testable the moment
//!     the walk lands.

use std::sync::atomic::{AtomicU64, Ordering};

/// Build a real SQLite database by running `statements` through bundled-C
/// SQLite (`rusqlite`) and return its on-disk bytes. This is the reusable
/// oracle fixture-builder the whole cross-check suite is built on.
///
/// The fixture is written inside a freshly-`create_dir`'d per-run subdirectory
/// rather than a predictably-named file placed straight in the shared temp dir:
/// `create_dir` fails if the path already exists (including a planted symlink),
/// which sidesteps the classic `/tmp` symlink-swap hazard without pulling in a
/// `tempfile` dependency (this is a zero-dep crate; even the test oracle stays
/// lean). A per-run counter keeps parallel test threads from colliding.
fn build_sqlite_db(statements: &[&str]) -> Vec<u8> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "sqlite_file_xcheck_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::create_dir(&dir).expect("create fresh fixture dir");
    let path = dir.join("oracle.db");

    {
        let conn = rusqlite::Connection::open(&path).expect("open sqlite db");
        for stmt in statements {
            conn.execute_batch(stmt).expect("run statement");
        }
    } // connection dropped → file flushed and closed

    let bytes = std::fs::read(&path).expect("read db file");
    let _ = std::fs::remove_dir_all(&dir);
    bytes
}

/// A real SQLite file must open with the format's magic string and expose the
/// header fields the reader is built around. Validating them here means every
/// fixture the later phases parse is known-good, and proves the `rusqlite`
/// oracle is available in this environment.
#[test]
fn oracle_produces_wellformed_sqlite_header() {
    let db = build_sqlite_db(&[
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, score REAL, blob BLOB)",
        "INSERT INTO t VALUES (1, 'Ada', 1.5, x'dead'), (2, 'Grace', NULL, NULL)",
    ]);

    // The 100-byte database header lives at the start of page 1.
    assert!(db.len() >= 100, "file too small to hold a header");
    assert_eq!(&db[0..16], b"SQLite format 3\0", "magic string");

    // Offset 16: page size, u16 big-endian. SQLite encodes 65536 as the value
    // 1; every other legal size is a power of two in 512..=32768.
    let raw_page_size = u16::from_be_bytes([db[16], db[17]]);
    let page_size: u32 = if raw_page_size == 1 {
        65536
    } else {
        u32::from(raw_page_size)
    };
    assert!(
        page_size >= 512 && page_size.is_power_of_two(),
        "page size {page_size} must be a power of two ≥ 512"
    );

    // Offset 28: in-header database size in pages (u32-be). Must cover the file.
    let page_count = u32::from_be_bytes([db[28], db[29], db[30], db[31]]);
    assert_eq!(
        u64::from(page_count) * u64::from(page_size),
        db.len() as u64,
        "in-header page count must match the file length"
    );

    // Offset 44: schema format number (1..=4). Offset 56: text encoding
    // (1 = UTF-8) — the reader decodes TEXT as UTF-8, so this must hold.
    let schema_format = u32::from_be_bytes([db[44], db[45], db[46], db[47]]);
    assert!(
        (1..=4).contains(&schema_format),
        "schema format {schema_format} out of range"
    );
    let text_encoding = u32::from_be_bytes([db[56], db[57], db[58], db[59]]);
    assert_eq!(text_encoding, 1, "reader assumes UTF-8 (encoding = 1)");
}

/// Parse a real SQLite file's header with **our** `header` module and confirm
/// every field matches an independent inline read of the same bytes. This makes
/// the reader's header parser — not just the reader's assumptions — the thing
/// under test against genuine `rusqlite` output.
#[test]
fn our_header_matches_a_real_sqlite_file() {
    let db = build_sqlite_db(&[
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)",
        "INSERT INTO t VALUES (1, 'Ada'), (2, 'Grace')",
    ]);

    let header = sqlite_file::Header::parse(&db).expect("our header parses a real db");

    // Page size (offset 16, u16-be; 1 ⇒ 65536).
    let raw = u16::from_be_bytes([db[16], db[17]]);
    let expected_page_size: u32 = if raw == 1 { 65536 } else { u32::from(raw) };
    assert_eq!(header.page_size, expected_page_size);

    // Reserved space (offset 20), page count (offset 28), schema format
    // (offset 44), text encoding (offset 56).
    assert_eq!(header.reserved_space, db[20]);
    assert_eq!(
        header.page_count,
        u32::from_be_bytes([db[28], db[29], db[30], db[31]])
    );
    assert_eq!(
        header.schema_format,
        u32::from_be_bytes([db[44], db[45], db[46], db[47]])
    );
    assert_eq!(header.text_encoding, sqlite_file::TextEncoding::Utf8);

    // The header's page count must describe the actual file, and the pager must
    // agree and be able to hand back page 1 (which carries the header).
    let (h2, pager) = sqlite_file::Pager::open(&db).unwrap();
    assert_eq!(h2, header);
    assert_eq!(
        u64::from(header.page_count) * u64::from(header.page_size),
        db.len() as u64
    );
    assert_eq!(pager.page_count(), header.page_count as usize);
    assert_eq!(&pager.page(1).unwrap()[0..16], sqlite_file::header::MAGIC);
}

/// Walk the **real `sqlite_schema` b-tree** (rooted on page 1) with our reader
/// and confirm the object name + rootpage of every table/index matches what
/// SQLite reports via `SELECT name, rootpage FROM sqlite_schema`. This exercises
/// the whole leaf/interior walk + record-decode pipeline against genuine SQLite
/// output — the first end-to-end row read.
#[test]
fn sqlite_schema_walk_matches_the_oracle() {
    let db = build_sqlite_db(&[
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)",
        "CREATE TABLE u (x INTEGER, y INTEGER)",
        "CREATE INDEX idx_u_x ON u (x)",
        "INSERT INTO t VALUES (1, 'Ada'), (2, 'Grace')",
        "INSERT INTO u VALUES (10, 20), (30, 40)",
    ]);

    // Our reader: walk page 1 (the sqlite_schema root) and decode each 5-column
    // row (type, name, tbl_name, rootpage, sql).
    let (header, pager) = sqlite_file::Pager::open(&db).unwrap();
    let rows = sqlite_file::btree::walk_table(&pager, &header, 1).unwrap();
    let mut ours: Vec<(String, i64)> = Vec::new();
    for (_rowid, record) in &rows {
        let cols = sqlite_file::record::decode(record).expect("schema row decodes");
        assert_eq!(cols.len(), 5, "sqlite_schema has five columns");
        let name = match &cols[1] {
            sqlite_file::SqlValue::Text(s) => s.clone(),
            other => panic!("name column should be TEXT, got {other:?}"),
        };
        // rootpage is 0 for views/triggers; tables and indexes have a real page.
        let rootpage = match &cols[3] {
            sqlite_file::SqlValue::Int(n) => *n,
            sqlite_file::SqlValue::Null => 0,
            other => panic!("rootpage should be INTEGER, got {other:?}"),
        };
        ours.push((name, rootpage));
    }
    ours.sort();

    // The oracle: the same view straight out of bundled-C SQLite.
    let conn = {
        // Reopen the bytes we already built by writing them to a fresh temp file
        // and querying — simplest independent path to the same rows.
        static COUNTER2: AtomicU64 = AtomicU64::new(1_000_000);
        let unique = COUNTER2.fetch_add(1, Ordering::Relaxed);
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "sqlite_file_schema_{}_{}",
            std::process::id(),
            unique
        ));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("oracle.db");
        std::fs::write(&path, &db).unwrap();
        let conn = rusqlite::Connection::open(&path).unwrap();
        // Leak the tempdir cleanup to the OS reboot is fine, but tidy up anyway
        // after reading.
        (conn, dir)
    };
    let mut theirs: Vec<(String, i64)> = conn
        .0
        .prepare("SELECT name, rootpage FROM sqlite_schema")
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    theirs.sort();
    drop(conn.0);
    let _ = std::fs::remove_dir_all(&conn.1);

    assert_eq!(ours, theirs, "our sqlite_schema walk must match SQLite's");
    assert!(!ours.is_empty(), "expected some schema objects");
}

/// The full round-trip gate: decode every row straight out of the file bytes
/// and confirm it equals what SQLite reports over SQL. It needs the table
/// b-tree walk to locate record bytes, which lands in Phase E3 — this staged
/// placeholder documents the gate and turns green the moment `read_table`
/// exists.
#[test]
#[ignore = "activates in Phase E3 once the b-tree walk can locate row records"]
fn rows_round_trip_through_our_reader() {
    let _db = build_sqlite_db(&[
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)",
        "INSERT INTO t VALUES (1, 'Ada'), (2, 'Grace')",
    ]);
    // TODO(E3): let rows = sqlite_file::read_table(&_db, "t").unwrap();
    //           assert_eq!(rows, [(1, [Int(1), Text("Ada")]), (2, [Int(2), Text("Grace")])]);
}
