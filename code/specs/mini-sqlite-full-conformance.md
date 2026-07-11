# Mini-SQLite → Full SQLite Replacement: Conformance Roadmap

## Purpose

`mini-sqlite` is a from-scratch reimplementation of SQLite. This document is the
roadmap for taking it from its current state — an **in-memory SQL query engine**
that passes a 24-fixture DB-API suite — to a **drop-in replacement for real
SQLite**: one that speaks the same SQL, produces the same answers, and reads and
writes the same byte-compatible `.sqlite` files.

"Conformance" here means **conformance to real SQLite**, measured continuously by
running the same SQL through mini-sqlite and through the bundled C SQLite library
and asserting the results match — not merely passing hand-authored fixtures.

This spec sets the levels, the three parallel workstreams, the measurement
instrument, and the working discipline. Individual capabilities each get their
own smaller spec and PR as they are built.

## Two halves of one system

SQLite is really two things bolted together, and this repo already has both
halves as separate, composable pieces:

| Half | What it is | Repo pieces (Rust) |
|---|---|---|
| **Query engine** | SQL text → results | `sql-lexer` → `sql-parser` → `sql-planner` → `sql-optimizer` → `sql-codegen` → `sql-vm`, plus the `mini-sqlite` DB-API façade |
| **Storage engine** | byte-compatible `.sqlite` files | `sqlite-file` (read-only subset, merged) — to be extended by a new full read/write `storage-sqlite` crate |

The seam between them is the **`Backend` trait** (`sql-backend/src/lib.rs`). Today
mini-sqlite always uses `InMemoryBackend`. A storage engine that implements
`Backend` lets the *entire unmodified* query pipeline run against a real file.
Python has already proven this end to end: `python/storage-sqlite` is a full
read/write byte-compatible file format whose `SqliteFileBackend` plugs in under
the same `Backend` interface, so Python's mini-sqlite already queries real
`.sqlite` files. The Rust side is the catch-up.

## Where we are today (baseline)

**Query engine — works, in-memory:** `CREATE`/`DROP TABLE` (with `NOT NULL` /
`PRIMARY KEY` / `UNIQUE` / `DEFAULT` enforced), multi-row `INSERT`, `UPDATE`,
`DELETE`, `SELECT` (projection, aliases, `*`, `DISTINCT`, `WHERE`, `GROUP BY` on
columns, `HAVING`, `ORDER BY`, `LIMIT`/`OFFSET`), `INNER`/`CROSS` joins, the
common operators plus `BETWEEN`/`IN`/`LIKE`/`IS NULL`, ~11 scalar and 5 aggregate
functions, and snapshot-based `BEGIN`/`COMMIT`/`ROLLBACK`.

**Gaps, in three classes:**

1. **A silent correctness bug.** `LEFT`/`RIGHT`/`FULL OUTER JOIN` parse and plan
   correctly but the code generator falls back to a cross join and drops the
   `ON` clause (`sql-codegen/src/lib.rs`, the `JoinKind::Inner` guard) — wrong
   answers, no error. This is why we build the oracle *first*.
2. **Missing SQL surface.** Subqueries, CTEs, `UNION`/`INTERSECT`/`EXCEPT`,
   `CASE`, `CAST`, window functions, `CREATE INDEX`/`VIEW`/`TRIGGER`/`ALTER`,
   `INSERT…SELECT`, UPSERT, `PRAGMA`; no type **affinity** or **collation**;
   string quoting and BLOB literals diverge from SQLite.
3. **No persistence.** All state is `Vec<Row>` in RAM; `connect()` refuses any
   database name but `:memory:`. No pager, no b-tree, no file format in the query
   engine's own storage.

**Conformance instruments today:** the 24 JSON fixtures
(`code/specs/mini-sqlite-conformance/`, Levels 0–1, in-memory) run in several
language ports. Differential testing against *real* SQLite exists only in the
file-format packages (`sqlite-file` via `rusqlite`, `python/storage-sqlite` via
stdlib `sqlite3`) — never yet for the query engine. Building that instrument for
the query engine is the linchpin of this roadmap.

## The differential oracle (the measuring instrument)

Every level is gated by a **differential conformance harness**: a corpus of SQL
scripts, each executed through both mini-sqlite and bundled SQLite (`rusqlite`, a
**dev-dependency only** — never a runtime dependency), with results and errors
compared. This mirrors the proven pattern in
`sqlite-file/tests/cross_check_reader.rs`.

- **Match criteria:** identical result rows (order-sensitive only when the query
  has `ORDER BY`), identical column names (case-insensitive), and error-vs-success
  agreement (with error *class* mapped, not exact message text).
- **Known-divergence ledger:** where mini-sqlite intentionally differs or hasn't
  reached a feature yet, cases are quarantined in an explicit allow-list with a
  reason — never silently skipped. Shrinking the ledger *is* the conformance
  metric.
- **Corpus growth:** seeded Anki/real-world first (see Scope), then broadened
  toward SQLite's own semantics.

## Levels

| Level | Goal | Core work |
|---|---|---|
| **L0/L1** ✅ | in-memory DB-API | the existing 24 fixtures |
| **L2** | *measure*; stop shipping wrong answers | differential oracle harness; fix the outer-join correctness bug; quarantine-ledger established |
| **L3** | real-world `SELECT` semantics | correct outer joins, subqueries, `CASE`, `CAST`, set ops, `NATURAL`/`USING`/comma joins, `ORDER BY NULLS`, `GROUP BY` expressions, more functions, **type affinity + collation**, SQLite string/BLOB literal syntax — each oracle-gated |
| **L4** | `SELECT` from a real `.sqlite` file | new `rust/storage-sqlite`: full-read format (schema catalog, index b-trees, freelist) + `SqliteFileBackend: Backend`; flip `connect()` to open real files read-only |
| **L5** | byte-compatible **writer** | pager + rollback journal + b-tree insert/split + record encode + freelist + index maintenance → files the C library and `sqlite3` CLI open. **Also Engram "Phase F"** — removes `rusqlite` from the Anki write path |
| **L6** | DDL breadth + planner uses indexes | `CREATE INDEX`/`VIEW`/`TRIGGER`/`ALTER`, UPSERT, `INSERT…SELECT`, `PRAGMA`, `AUTOINCREMENT`/`FK`/`CHECK`; optimizer emits index scans instead of full scans |
| **L7** | rollout + perf | replicate the stack across language ports; WAL journal mode; optimizer improvements |

## Execution model: three alternating streams

Work rotates across three streams rather than completing one before the next, so
the query engine, its correctness measurement, and its storage engine advance
together (and no single front stalls the others):

- **Stream A — Oracle & correctness (L2, ongoing):** the differential harness and
  the correctness fixes it surfaces. Starts with the outer-join bug.
- **Stream B — SQL surface (L3, ongoing):** grammar/planner/codegen/vm feature
  work, one capability per PR, each proven against the oracle.
- **Stream C — Storage engine (L4→L5):** the new `rust/storage-sqlite` crate and
  the `Backend` adapter, then the byte-compatible writer.

Each PR advances one stream; consecutive PRs rotate streams. L6/L7 begin once the
read+write storage engine and the core SQL surface are in place.

## Rust storage engine: a new `storage-sqlite` crate

The full read/write file format lives in a **new** crate
`code/packages/rust/storage-sqlite`, name-parity with `python/storage-sqlite`
(the reference to port). The existing `sqlite-file` crate stays as the read-only
subset the Engram Anki importer already depends on; `storage-sqlite` builds on its
primitives (varint, record, header, pager, b-tree walk) and adds the write path,
index b-trees, freelist, and the `SqliteFileBackend` adapter. Consolidation of
the two crates (and of the two `SqlValue` types — `sql-backend`'s 6-variant with
`Blob` vs the execution engine's 4-variant) is a later cleanup, not a blocker.

## Scope: Anki / real-world first

Prioritize the SQL and file features the Engram/Anki path and common applications
actually exercise, then broaden toward SQLite's full semantics. Concretely, the
early oracle corpus and feature order favor: reading real `collection.anki2`
tables; the `SELECT` shapes real apps issue (joins, subqueries, `CASE`,
aggregates, `ORDER BY`); correct NULL and type-affinity behavior; and writing
files the real library re-opens. Exotic corners (window functions, recursive
CTEs, `ATTACH`, WAL) come after the common path is faithful.

## Working discipline

Same cadence that carried the Phase E reader to completion:

- **Specs first** — this roadmap, then a short spec per non-trivial capability,
  committed before its implementation.
- **Small, single-capability PRs**, each rotating the active stream.
- **Differential oracle is the gate** — no feature lands without the harness
  agreeing with real SQLite (or an explicit, justified ledger entry).
- **`#![forbid(unsafe_code)]`** across the storage engine; hostile-input safety
  (bounds/overflow/cycle/amplification guards) as established in `sqlite-file`.
- **Security review with the diff inline** before every push; **babysit each PR
  to MERGED**; `rusqlite`/bundled SQLite remains **dev-dependency only**.

## Related specs

- `code/specs/mini-sqlite-conformance/` — the existing L0/L1 fixture suite.
- `code/specs/mini-sqlite-python.md` — the Python reference engine spec.
- `code/specs/mini-sqlite-porting.md` — cross-language porting guide.
- `code/specs/storage-sqlite.md` — the on-disk file format spec.
- `code/specs/storage-sqlite-v2-auto-index.md`,
  `code/specs/storage-sqlite-v3-auto-index.md` — index b-tree evolution.
- `code/specs/DT11-b-tree.md` — generic b-tree data structure.
