# Database fundamentals delivery backlog

Owner direction: 2026-11-25. Parent: [DB00](DB00-database-learning-program.md).

Status: `planned`, `in-progress`, `implemented`, `validated`, `merged` are
different. No row becomes validated from documentation or a directory count.
Priority is dependency-first; re-prioritize after every completed slice.

## Ordered work

| ID | Priority | Work | Depends on | Acceptance |
|---|---|---|---|---|
| DB-001 | P0 | Inventory existing DB/storage/transistor APIs, tests and assurance | — | Revision-pinned paths and evidence; reconcile stale SQLite roadmap; distinguish whole-file writing from incremental writing |
| DB-002 | P0 | Common semantics and per-lane conformance matrix | — | All 18 native lanes named; independent expected outcomes; explicit error/resource/thread/persistence contracts |
| DB-003 | P0 | NAND byte programming primitive in every lane | DB-002 | All 65,536 byte pairs checked against an independent per-bit oracle; every invalid boundary rejected; known mutants rejected |
| DB-004 | P0 | Raw NAND geometry/pages/blocks | DB-003 | Erased state, whole-page program-once profile, block erase, bounds and no mutation on rejected operations; bounded history exploration; fuzz invariants |
| DB-005 | P0 | Deterministic HDD simulator | DB-002 | Sector data round trip, overwrite, seek/rotation/transfer accounting, phase after seek, wraparound/cross-track tests and bounded geometry sweep |
| DB-006 | P0 | Pure block-device and failure contract | DB-004, DB-005 | Volatile/stable bytes, completion vs persistence, barriers, flush/FUA and explicit crash schedules |
| DB-007 | P1 | Transistor polarity/region audit | DB-001 | Correct PMOS polarity under documented terminal domains; analog/DC comparison; no claim that ordinary MOSFET equals flash cell |
| DB-008 | P1 | Flash-cell discrete charge/threshold model | DB-007 | Program/erase/read state, explicit threshold windows, SLC first, saturation and invalid voltage handling |
| DB-009 | P1 | NAND failures and endurance | DB-004, DB-008 | Bad blocks, finite erase budget, retention/read/program disturb, injected bit errors; deterministic reproducible fault plans |
| DB-010 | P1 | ECC and integrity primitives | DB-009 | Enumerated correctable patterns, detected unsupported patterns, explicit uncorrectable results; reuse existing coding/checksum packages |
| DB-011 | P1 | Minimal flash translation layer | DB-006, DB-009 | Logical/physical mapping, out-of-place writes, invalid pages, spare capacity; no stale reads after overwrite |
| DB-012 | P1 | SSD garbage collection and wear leveling | DB-011 | Preserve live data, bounded progress/space rules, measured write amplification and erase distribution |
| DB-013 | P1 | Controller metadata recovery | DB-012 | Interrupt every metadata/data persistence step; recover mapping or fail explicitly; no acknowledged durable data lost |
| DB-014 | P1 | Queues and NVMe teaching model | DB-006 | Queue IDs, command IDs, submissions/completions, backpressure, out-of-order completion, flush semantics; protocol/model fidelity limits stated |
| DB-015 | P1 | Device comparison lessons and experiments | DB-005, DB-012, DB-014 | Same seeded workloads/units on HDD and SSD; explain sequential/random access, GC tail latency and queue depth |
| DB-016 | P2 | Record/page codecs and checksums | DB-006 | Round trips, hostile bytes, truncation/overflow, canonical encoding and finite primitive sweeps |
| DB-017 | P2 | Pager, page allocator and buffer pool | DB-016 | Dirty/pinned/evicted page invariants, allocation ownership, bounded capacity, I/O failure propagation |
| DB-018 | P2 | Heap tables and catalog | DB-017 | Stable row IDs, CRUD, scans, schema metadata and restart persistence |
| DB-019 | P2 | Page-backed B+ tree | DB-017 | Split/merge/root transitions, duplicate policy, range scans, corruption rejection, reference-map differential tests |
| DB-020 | P2 | WAL and rollback journal packages | DB-006, DB-016 | Log framing/checksum, redo/undo contracts, write-ahead ordering and every crash boundary tested |
| DB-021 | P2 | Transactions and recovery | DB-018, DB-020 | Commit/abort/checkpoint/restart; atomicity and durability under declared faults; distinguish crash vs power-loss models |
| DB-022 | P2 | Locks, latches and thread scheduler contracts | DB-002 | Lock ordering, ownership, starvation policy, cancellation, deterministic schedules and native threads |
| DB-023 | P2 | Multithreaded B+ tree engine | DB-019, DB-021, DB-022 | Concurrent scans/writes, linearizable KV operations, no data races, bounded deadlock behavior and clean shutdown |
| DB-024 | P2 | MVCC and isolation | DB-023 | Snapshot lifetime/reclamation; anomaly histories for each supported level; serializability checks where promised |
| DB-025 | P2 | SSTable, Bloom filter and merge iterator | DB-016 | Sorted immutable blocks, sparse indexes, checksums, no false negatives, deterministic merges and tombstones |
| DB-026 | P2 | LevelDB-style LSM engine | DB-020, DB-022, DB-025 | Memtable/WAL/flush/manifest/compaction/reopen, snapshots and thread safety; independent ordered-map oracle |
| DB-027 | P3 | RocksDB-style feature increments | DB-026 | One feature per slice: leveled/universal compaction, column families, block cache, write batches, compaction filters, rate limiting |
| DB-028 | P2 | Existing SQL stack parity and backend wiring | DB-001, DB-021 | Per-feature lane matrix; supported queries run on native persisted backends; unsupported features fail explicitly |
| DB-029 | P3 | Relational feature queue | DB-028 | NULL/types, joins, aggregates, indexes, constraints, subqueries, CTEs, sets, views, triggers, windows; each oracle-gated independently |
| DB-030 | P3 | Real OS adapters and direct/buffered I/O | DB-006, DB-023 | Scoped capabilities, actual flush behavior and filesystem assumptions, integration tests on Linux/macOS/Windows |
| DB-031 | P3 | Production behavior | DB-024, DB-026, DB-030 | Metrics, cancellation, resource limits, backpressure, fault reporting, backup/restore, corruption tools, recovery drills |
| DB-032 | P3 | Performance experiments | DB-015, DB-023, DB-026 | Reuse VER07 harness/statistics; throughput, tails, amplification, CPU/cache/memory; separate simulation ticks from measured nanoseconds |
| DB-033 | P3 | Warrant adoption and contract discharge | Each slice | Native bindings, generated evidence, source/contract binding and negative controls as tool lanes become available; no fabricated pass while unavailable |
| DB-034 | P4 | Replication, consensus, sharding and distributed transactions | DB-031 | Separately specified failures, partitions, clock assumptions, consistency histories and deterministic multi-node tests |
| DB-035 | P1 | WASM execution target | DB-003 onward | Explicit supported host profiles and conformance execution; no WASM directory counted as a nineteenth native-language port |

## All-language delivery matrix

Each cell must link implementation, test command and actual run evidence before
changing to validated. `queued` means it has not passed. Warrant is a separate
evidence dimension and cannot silently change a native-test result.

| Lane | NAND primitive | Raw NAND | HDD | Device faults | Threaded engine |
|---|---|---|---|---|---|
| C | queued | queued | queued | queued | queued |
| C++ | queued | queued | queued | queued | queued |
| C# | queued | queued | queued | queued | queued |
| Dart | queued | queued | queued | queued | queued |
| Elixir | queued | queued | queued | queued | queued |
| F# | queued | queued | queued | queued | queued |
| Go | queued | queued | queued | queued | queued |
| Haskell | queued | queued | queued | queued | queued |
| Java | queued | queued | queued | queued | queued |
| Kotlin | queued | queued | queued | queued | queued |
| Lua | queued | queued | queued | queued | queued |
| OCaml | queued | queued | queued | queued | queued |
| Perl | queued | queued | queued | queued | queued |
| Python | queued | queued | queued | queued | queued |
| Ruby | queued | queued | queued | queued | queued |
| Rust | queued | queued | queued | queued | queued |
| Swift | queued | queued | queued | queued | queued |
| TypeScript | queued | queued | queued | queued | queued |

## Current execution

- DB-002: in progress. DB00 and the first NAND/HDD contracts define the scope.
- DB-003: first implementation slice, with exhaustive tests across all lanes.
- DB-004 and DB-005 follow before controller, pager or SQL expansion.
- Scaffolding currently supports 15 of these lanes. Add/repair native C#/F#/Dart
  scaffold support before treating their package metadata as generator-complete.
- Full workspace/build-planner checks require a materialized Git checkout;
  isolated package checks are recorded separately, never substituted silently.
