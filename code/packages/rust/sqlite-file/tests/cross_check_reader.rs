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
//!   * **with the b-tree walk (Phase E3a):** walk the real `sqlite_schema`
//!     b-tree and match every object's `(name, rootpage)` against SQLite.
//!   * **with overflow chains (Phase E3b):** walk a real user table — including a
//!     row whose TEXT is far larger than one page — and assert every decoded
//!     column equals what `rusqlite` returns over SQL. This is the full
//!     round-trip gate, and it exercises overflow-chain reassembly end to end.

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

/// The full round-trip gate (Phase E3b): decode every row of a real table
/// straight out of the file bytes and confirm it equals what SQLite reports over
/// SQL — *including a row whose TEXT is far too large for one page*, which forces
/// the reader through the overflow-chain reassembly path.
///
/// To read table `t` we must first find its root page: walk `sqlite_schema`
/// (page 1), find the `('table', 't')` row, take its `rootpage` (column 3), then
/// `walk_table` that page and decode each record. Because `id` is declared
/// `INTEGER PRIMARY KEY`, it is an alias for the rowid and stored as `NULL` in the
/// record itself — so the id we compare is the walk's `rowid`, and column 1 is the
/// `body` TEXT.
#[test]
fn rows_round_trip_through_our_reader() {
    // ~6.5 KB of text — larger than the default 4 KiB page, so this row cannot
    // fit inline and must spill into an overflow chain.
    let big = "Ada Lovelace ".repeat(500);
    assert!(big.len() > 5000, "the large row must exceed one page");
    let create = "CREATE TABLE t (id INTEGER PRIMARY KEY, body TEXT)";
    let ins1 = "INSERT INTO t VALUES (1, 'short one')";
    let ins2 = format!("INSERT INTO t VALUES (2, '{big}')");
    let ins3 = "INSERT INTO t VALUES (3, 'short three')";
    let db = build_sqlite_db(&[create, ins1, &ins2, ins3]);

    // --- Our reader: sqlite_schema → rootpage of `t` → walk + decode.
    let (header, pager) = sqlite_file::Pager::open(&db).unwrap();
    let schema = sqlite_file::btree::walk_table(&pager, &header, 1).unwrap();
    let mut root: Option<u32> = None;
    for (_rowid, record) in &schema {
        let cols = sqlite_file::record::decode(record).expect("schema row decodes");
        let is_table = matches!(&cols[0], sqlite_file::SqlValue::Text(s) if s == "table");
        let is_t = matches!(&cols[1], sqlite_file::SqlValue::Text(s) if s == "t");
        if is_table && is_t {
            root = match &cols[3] {
                sqlite_file::SqlValue::Int(n) => Some(*n as u32),
                other => panic!("rootpage should be INTEGER, got {other:?}"),
            };
        }
    }
    let root = root.expect("table t must appear in sqlite_schema");

    let rows = sqlite_file::btree::walk_table(&pager, &header, root).unwrap();
    let mut ours: Vec<(i64, String)> = rows
        .iter()
        .map(|(rowid, record)| {
            let cols = sqlite_file::record::decode(record).expect("row decodes");
            // col 0 is NULL (the INTEGER PRIMARY KEY alias); col 1 is the body.
            let body = match &cols[1] {
                sqlite_file::SqlValue::Text(s) => s.clone(),
                other => panic!("body should be TEXT, got {other:?}"),
            };
            (*rowid, body)
        })
        .collect();
    ours.sort();

    // --- Oracle: the same rows straight out of bundled-C SQLite over SQL.
    static COUNTER3: AtomicU64 = AtomicU64::new(2_000_000);
    let unique = COUNTER3.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "sqlite_file_rows_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("oracle.db");
    std::fs::write(&path, &db).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    let mut theirs: Vec<(i64, String)> = conn
        .prepare("SELECT id, body FROM t")
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    theirs.sort();
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);

    // The overflow row and the two inline rows must all match byte-for-byte.
    assert_eq!(ours, theirs, "every row must round-trip through our reader");
    assert!(
        ours.iter().any(|(_, body)| body.len() > 5000),
        "the overflow row must have been reassembled in full"
    );
}

/// Phase E4: the public schema API should expose the same table root pages the
/// low-level E3 tests hand-extracted from `sqlite_schema`.
#[test]
fn public_schema_api_finds_table_root_pages() {
    let db = build_sqlite_db(&[
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)",
        "CREATE TABLE u (x INTEGER, y INTEGER)",
        "CREATE VIEW v AS SELECT name FROM t",
        "INSERT INTO t VALUES (1, 'Ada'), (2, 'Grace')",
        "INSERT INTO u VALUES (10, 20), (30, 40)",
    ]);

    let schema = sqlite_file::read_schema(&db).expect("schema should decode");
    assert!(
        schema.iter().any(|entry| entry.object_type == "table"
            && entry.name == "t"
            && entry.table_name == "t"
            && entry.root_page.is_some()
            && entry
                .sql
                .as_deref()
                .unwrap_or("")
                .contains("CREATE TABLE t")),
        "table t should appear in decoded sqlite_schema"
    );
    assert!(
        schema.iter().any(|entry| entry.object_type == "view"
            && entry.name == "v"
            && entry.root_page.is_none()),
        "view v should appear without a root page"
    );

    let t_root = sqlite_file::table_root_page(&db, "t").expect("table t root");
    let u_root = sqlite_file::table_root_page(&db, "u").expect("table u root");
    assert_ne!(t_root, 0);
    assert_ne!(u_root, 0);

    static COUNTER4: AtomicU64 = AtomicU64::new(3_000_000);
    let unique = COUNTER4.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "sqlite_file_schema_api_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("oracle.db");
    std::fs::write(&path, &db).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    let theirs: Vec<(String, i64)> = conn
        .prepare("SELECT name, rootpage FROM sqlite_schema WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(theirs.contains(&("t".to_string(), i64::from(t_root))));
    assert!(theirs.contains(&("u".to_string(), i64::from(u_root))));
}

/// Phase E4: callers should be able to read a table by name without knowing how
/// to walk `sqlite_schema` manually. This also keeps the overflow row gate on
/// the public API path.
#[test]
fn read_table_api_decodes_named_table_rows() {
    let big = "public api overflow ".repeat(400);
    assert!(big.len() > 5000, "the large row must exceed one page");
    let ins2 = format!("INSERT INTO t VALUES (2, '{big}', 2.5, x'cafe', 'kept')");
    let db = build_sqlite_db(&[
        "CREATE TABLE t (id INTEGER PRIMARY KEY, body TEXT, score REAL, payload BLOB, note TEXT)",
        "INSERT INTO t VALUES (1, 'short one', 1.25, x'dead', NULL)",
        &ins2,
        "INSERT INTO t VALUES (3, 'short three', NULL, NULL, 'tail')",
    ]);

    let rows = sqlite_file::read_table(&db, "t").expect("read table t");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].0, 1);
    assert_eq!(rows[0].1[0], sqlite_file::SqlValue::Null);
    assert_eq!(
        rows[0].1[1],
        sqlite_file::SqlValue::Text("short one".to_string())
    );
    assert_eq!(rows[0].1[2], sqlite_file::SqlValue::Real(1.25));
    assert_eq!(rows[0].1[3], sqlite_file::SqlValue::Blob(vec![0xde, 0xad]));
    assert_eq!(rows[0].1[4], sqlite_file::SqlValue::Null);

    assert_eq!(rows[1].0, 2);
    assert_eq!(rows[1].1[1], sqlite_file::SqlValue::Text(big.clone()));
    assert_eq!(rows[1].1[2], sqlite_file::SqlValue::Real(2.5));
    assert_eq!(rows[1].1[3], sqlite_file::SqlValue::Blob(vec![0xca, 0xfe]));
    assert!(
        matches!(&rows[2].1[2], sqlite_file::SqlValue::Null),
        "NULL REAL values should decode as SqlValue::Null"
    );
}

#[test]
fn read_table_reports_missing_tables() {
    let db = build_sqlite_db(&["CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)"]);
    assert_eq!(
        sqlite_file::read_table(&db, "missing"),
        Err(sqlite_file::SqliteError::NoSuchTable("missing".to_string()))
    );
}

/// Write-path cross-check: bytes produced by OUR writer — including an overflow
/// chain for a large row — are accepted and correctly read back by the real
/// bundled-C SQLite. This is the inverse of the reader gates above:
/// `write_single_table_db` → open with `rusqlite` → `PRAGMA integrity_check` +
/// `SELECT`. `integrity_check` returning "ok" means real SQLite validated the
/// page count, b-tree framing, and overflow chain we emitted.
#[test]
fn our_writer_output_reads_in_real_sqlite_with_overflow() {
    use sqlite_file::page_writer::write_single_table_db;
    use sqlite_file::SqlValue;

    let big = "overflow-payload ".repeat(500); // ~8500 bytes, ≫ the inline limit
    let rows = vec![
        (1i64, vec![SqlValue::Int(10), SqlValue::Text("alpha".into())]),
        (2, vec![SqlValue::Int(20), SqlValue::Text(big.clone())]),
        (3, vec![SqlValue::Int(30), SqlValue::Text("gamma".into())]),
    ];
    let db = write_single_table_db(4096, "docs", "CREATE TABLE docs(n, body)", &rows).unwrap();
    assert!(
        db.len() / 4096 > 2,
        "the big row must have spilled into overflow pages"
    );

    static COUNTER_W: AtomicU64 = AtomicU64::new(9_000_000);
    let unique = COUNTER_W.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!("sqlite_file_writer_{}_{}", std::process::id(), unique));
    std::fs::create_dir(&dir).expect("create fresh fixture dir");
    let path = dir.join("ours.db");
    std::fs::write(&path, &db).unwrap();

    let conn = rusqlite::Connection::open(&path).unwrap();
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        integrity, "ok",
        "real SQLite's integrity_check must pass on our written file"
    );

    let mut got: Vec<(i64, i64, String)> = conn
        .prepare("SELECT rowid, n, body FROM docs")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, String>(2)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    got.sort();
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        got,
        vec![
            (1, 10, "alpha".to_string()),
            (2, 20, big),
            (3, 30, "gamma".to_string()),
        ],
        "real SQLite must read back every row our writer emitted"
    );
}

/// Multi-table write-path cross-check: a database our writer assembles with
/// several tables — one carrying an overflow row — is accepted by real
/// bundled-C SQLite (`PRAGMA integrity_check` → "ok") and every table reads
/// back over SQL.
#[test]
fn our_multi_table_writer_output_reads_in_real_sqlite() {
    use sqlite_file::page_writer::{write_multi_table_db, TableSpec};
    use sqlite_file::SqlValue;

    let big = "multi-overflow ".repeat(600); // ~9000 bytes → overflow chain
    let notes = vec![
        (1i64, vec![SqlValue::Int(1), SqlValue::Text("alpha".into())]),
        (2, vec![SqlValue::Int(2), SqlValue::Text(big.clone())]),
    ];
    let cards = vec![
        (1i64, vec![SqlValue::Int(10)]),
        (2, vec![SqlValue::Int(20)]),
        (3, vec![SqlValue::Int(30)]),
    ];
    let tables: &[TableSpec] = &[
        ("notes", "CREATE TABLE notes(nid, body)", &notes),
        ("cards", "CREATE TABLE cards(cid)", &cards),
    ];
    let db = write_multi_table_db(4096, tables).unwrap();

    static COUNTER_M: AtomicU64 = AtomicU64::new(11_000_000);
    let unique = COUNTER_M.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!("sqlite_file_multi_{}_{}", std::process::id(), unique));
    std::fs::create_dir(&dir).expect("create fresh fixture dir");
    let path = dir.join("ours.db");
    std::fs::write(&path, &db).unwrap();

    let conn = rusqlite::Connection::open(&path).unwrap();
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok", "real SQLite integrity_check on our multi-table file");

    // sqlite_schema lists both tables.
    let mut schema_names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    schema_names.sort();
    assert_eq!(schema_names, vec!["cards".to_string(), "notes".to_string()]);

    // Each table's rows read back, including the overflow row.
    let notes_body: Vec<(i64, String)> = conn
        .prepare("SELECT nid, body FROM notes ORDER BY nid")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get::<_, String>(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    let cids: Vec<i64> = conn
        .prepare("SELECT cid FROM cards ORDER BY cid")
        .unwrap()
        .query_map([], |r| r.get::<_, i64>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        notes_body,
        vec![(1, "alpha".to_string()), (2, big)],
        "notes (incl. overflow row) must read back"
    );
    assert_eq!(cids, vec![10, 20, 30], "cards must read back");
}

/// Tree-growth write-path cross-check: a table with more rows than fit on one
/// leaf produces an interior-rooted b-tree that real bundled-C SQLite accepts
/// (`PRAGMA integrity_check` → "ok") and reads back in full and in order.
#[test]
fn our_multi_leaf_btree_reads_in_real_sqlite() {
    use sqlite_file::page_writer::write_single_table_db;
    use sqlite_file::SqlValue;

    // 500 rows on a 512-byte page span many leaves under an interior root.
    let rows: Vec<(i64, Vec<SqlValue>)> = (1..=500)
        .map(|n| (n, vec![SqlValue::Int(n * 3), SqlValue::Text(format!("row-{n}"))]))
        .collect();
    let db = write_single_table_db(512, "items", "CREATE TABLE items(v, name)", &rows).unwrap();
    assert!(
        db.len() / 512 > 5,
        "500 rows on a 512-byte page must span several leaves"
    );

    static COUNTER_T: AtomicU64 = AtomicU64::new(13_000_000);
    let unique = COUNTER_T.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!("sqlite_file_tree_{}_{}", std::process::id(), unique));
    std::fs::create_dir(&dir).expect("create fresh fixture dir");
    let path = dir.join("ours.db");
    std::fs::write(&path, &db).unwrap();

    let conn = rusqlite::Connection::open(&path).unwrap();
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok", "real SQLite integrity_check on our multi-leaf tree");

    let count: i64 = conn
        .query_row("SELECT count(*) FROM items", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 500);

    // Full ordered readback matches what we wrote.
    let got: Vec<(i64, i64, String)> = conn
        .prepare("SELECT rowid, v, name FROM items ORDER BY rowid")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, String>(2)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);

    let want: Vec<(i64, i64, String)> = (1..=500).map(|n| (n, n * 3, format!("row-{n}"))).collect();
    assert_eq!(got, want, "real SQLite must read every row of the tree in order");
}

/// Multi-level tree write-path cross-check: enough rows that the *interior*
/// level itself overflows one page, so the writer stacks a second interior
/// level under the root. Real bundled-C SQLite must still accept the file
/// (`PRAGMA integrity_check` → "ok") and read back every row in order — proof
/// the deeper divider/right-most-child wiring matches the on-disk format.
#[test]
fn our_multi_level_btree_reads_in_real_sqlite() {
    use sqlite_file::page_writer::write_single_table_db;
    use sqlite_file::SqlValue;

    // 3000 rows on a 512-byte page → ~90 leaves, more than one 512-byte interior
    // page can index, forcing a root-over-interiors-over-leaves tree.
    let rows: Vec<(i64, Vec<SqlValue>)> = (1..=3000)
        .map(|n| (n, vec![SqlValue::Int(n * 3), SqlValue::Text(format!("row-{n}"))]))
        .collect();
    let db = write_single_table_db(512, "items", "CREATE TABLE items(v, name)", &rows).unwrap();

    // Confirm the tree is genuinely multi-level (root interior child is interior)
    // so this test can't silently degrade into the single-interior-level case.
    let schema = sqlite_file::schema::read_schema(&db).unwrap();
    let root = schema[0].root_page.unwrap() as usize;
    let root_off = (root - 1) * 512;
    assert_eq!(db[root_off], 0x05, "root must be interior");
    let first_ptr = u16::from_be_bytes([db[root_off + 12], db[root_off + 13]]) as usize;
    let first_child = u32::from_be_bytes([
        db[root_off + first_ptr],
        db[root_off + first_ptr + 1],
        db[root_off + first_ptr + 2],
        db[root_off + first_ptr + 3],
    ]) as usize;
    assert_eq!(
        db[(first_child - 1) * 512],
        0x05,
        "expected a multi-level tree (root's child is also interior)"
    );

    static COUNTER_ML: AtomicU64 = AtomicU64::new(14_000_000);
    let unique = COUNTER_ML.fetch_add(1, Ordering::Relaxed);
    let mut dir = std::env::temp_dir();
    dir.push(format!("sqlite_file_mltree_{}_{}", std::process::id(), unique));
    std::fs::create_dir(&dir).expect("create fresh fixture dir");
    let path = dir.join("ours.db");
    std::fs::write(&path, &db).unwrap();

    let conn = rusqlite::Connection::open(&path).unwrap();
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok", "real SQLite integrity_check on our multi-level tree");

    let count: i64 = conn
        .query_row("SELECT count(*) FROM items", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 3000);

    let got: Vec<(i64, i64, String)> = conn
        .prepare("SELECT rowid, v, name FROM items ORDER BY rowid")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, String>(2)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);

    let want: Vec<(i64, i64, String)> = (1..=3000).map(|n| (n, n * 3, format!("row-{n}"))).collect();
    assert_eq!(got, want, "real SQLite must read every row of the multi-level tree in order");
}
