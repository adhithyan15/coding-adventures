# Database implementation and validation backlog

Build from-scratch databases across the repository's languages, one small,
validated component at a time. Start at the media boundary, carry storage and
concurrency guarantees through the engine, and bring the existing SQL frontend
back in when the layers it needs are ready. The final native engines run
concurrent client requests and background work on real threads or equivalent
host workers, with deterministic schedules available for replay in tests.

## Acceptance and reuse

Use the repository's [layered validation roadmap](VER27-layered-validation-roadmap.md),
[contract linking](VER29-layer-contract-linking.md), and
[polyglot evidence](VER04i-polyglot-warrant-evidence.md). Each backlog item
should yield a spec-first change with a narrow API, stable claims, owned
assumptions, and evidence for the exact language and revision tested. Use the
repo's Warrant-derived examples, properties, fuzzing, bounded exhaustive checks,
proofs, and runtime-capability checks where applicable. Record unavailable or
out-of-scope evidence by kind and scope. Coverage locates unexercised code;
independent reference models, mutation controls, and cross-checks tell us whether
the tests detect wrong behavior. Use the existing release tiers for any release.

**Every existing package starts as a candidate.** This includes every tree
port, log reader or writer, LSM package, SQL frontend or runtime, media and
filesystem abstraction, and verification helper. Audit the precise package,
revision, and language before listing it as a dependency. An audit has five
outputs:

1. The public operations, formats, failure modes, and bounds the caller needs.
2. A comparison of the spec, source, and callable API, including omitted or
   stubbed paths and transitive dependencies.
3. Reproduced native tests and independent checks for the required guarantees.
   For stateful packages, include operation traces, malformed input, and state
   after errors. For concurrency or persistence assumptions, include the
   relevant schedules and failure points.
4. An evidence record naming the tested revision, claims, assumptions,
   platform, limits, and any unsupported or unverified behavior.
5. A disposition per required claim. Admit the exact guarantee, narrow the
   caller's dependency, or file a separate repair with a failing test before
   relying on it. Repeat for each port and for contract-affecting changes.

The first media models are zero-runtime-dependency leaves. Existing transistor
models are candidates for a later charge and threshold model. Start with a
logical SLC page contract so higher layers have something executable while we
check what the transistor packages can actually establish.

| Audit queue | Candidates | Required admission evidence |
|---|---|---|
| DB-A01 | Transistors, digital memory, device driver and filesystem packages | Match the actual abstraction, transistor transfer range and units, write ordering, error propagation, and crash assumptions. Keep real-device assumptions distinct from simulation claims. |
| DB-A02 | All B-tree, B+ tree, skip-list, heap, ordered-map, radix and other tree packages proposed for use | Check duplicates, sorted range bounds, update/delete, structural invariants, split/merge boundaries, reference ordered-map equivalence, adversarial input, and stated iterator or concurrency guarantees. Audit in-memory and page-backed trees separately. |
| DB-A03 | Bloom filters, hashes, checksums, compressors, byte codecs | Confirm byte conventions, seeded repeatability, rejection bounds and error classes. Independently check vectors, round trips, no false negatives for supported Bloom-filter operations, and checksum or collision assumptions as applicable. |
| DB-A04 | Rust/C#/F# WAL, Rust/Haskell LSM, SSTable and manifest candidates | Trace append durability and recovery, truncation/corruption, persisted-table reopen, tombstone visibility, compaction and snapshot guarantees. Record stubs and in-memory substitutions explicitly. |
| DB-A05 | File format readers/writers, mini-SQLite, lexer/parser/planner/optimizer/VM, CSV and spreadsheet backends | Separate frontend acceptance from execution semantics. Audit byte/file interop, DDL/DML, types, transactions, indexes and errors per port against an independent oracle and exact feature inventory. |
| DB-A06 | Schedulers, threading, atomics, channels, test harnesses and evidence tooling | Match memory and progress contracts, exercise adverse schedules, verify mutants fail, and check that the evidence runner executes the real public implementation. |

## Media and host storage

| Item | Depends on | Deliverable and acceptance |
|---|---|---|
| DB01 SLC NAND media | Leaf | Read, program and erase with byte-exact vectors. Check erased bytes, program restrictions, address and payload refusals, per-block wear and isolation. First Python implementation. [Spec](DB01-nand-flash.md) |
| DB02 HDD media | Leaf | Deterministic sector I/O with seek, rotation and transfer costs. Verify head and phase across accesses, random versus sequential traces, geometry and refusal behavior. First Python implementation. [Spec](DB02-hdd-simulator.md) |
| DB03 Flash cell and NAND array | DB-A01, DB01 interface | Model threshold sensing and charge states, then word lines, bit lines and strings. Validate that the abstraction yields the SLC media contract under stated assumptions. Add multilevel thresholds, program disturb, retention and ECC as separate slices. |
| DB04 Media fault injection | DB01, DB02 | Named crash cuts, torn writes, unreadable sectors, bad NAND blocks and deterministic fault schedules. Pin stable data, error precedence and allowed outcomes at each cut. |
| DB05 Flash translation layer | DB01, DB04 | Separate page mapping, invalidation, free-block tracking, overprovisioning, garbage collection, wear leveling and crash-safe mapping checkpoints. Show exact host/physical writes on small traces and measure steady-state write amplification. |
| DB06 Block and durability interface | DB02, DB04, DB05 | Common logical block interface with alignment, physical and logical sector sizes, discard, flush, order and atomic-write claims. Exercise volatile caches, reordering and power loss. Run identical client traces over HDD, FTL and a memory model. |
| DB07 Disk scheduling and interfaces | DB06 | Add FIFO, seek-aware scheduling and bounded queues. Model SATA/AHCI and NVMe submission/completion rings, queue depth, out-of-order completion, flush and force-unit-access as distinct slices. Validate request identity, completion and durability ordering separately. |
| DB08 Host adapters and calibration | DB06, DB07 | File-backed and native adapters, page cache/direct I/O alignment, file and directory sync, short writes and resource failures. Compare sequential/random workloads on declared real hardware with model traces. State empirical and device-specific assumptions. |

## Tree fundamentals

Teach each tree with a tiny printable example and one operation at a time.
Advance from an in-memory implementation to pages only after its invariants
are clear. DB-A02 determines whether to teach and harden an existing port or
implement the missing behavior from its contract.

| Item | Depends on | Deliverable and acceptance |
|---|---|---|
| DB09a B-tree | DB-A02 for any reuse | Insert and search across fixed-degree nodes, then split, borrow, merge and delete. Explain occupancy and why all leaves are at one depth. Enumerate small insert/delete histories and compare with an independent ordered map. |
| DB09b B+ tree | DB09a as learning prerequisite, DB-A02 for reuse | Keep values in leaves, define routing separator invariants and link leaves for range scans. Exercise root height changes, separator repair, and iterator results during redistribution. |
| DB09c Sorted runs and merge iteration | DB09a as learning prerequisite | Build sorted runs, merge them in key order and resolve duplicate versions and tombstones. Trace how the same updates differ between a tree that maintains one index and an LSM that merges several runs. |

## Shared engine components

| Item | Depends on | Deliverable and acceptance |
|---|---|---|
| DB10 Record and key codecs | DB-A03 | Stable byte vectors, explicit numeric order and collation, record framing/checksums, versioning and bounded decoders. Cross-read files between ports and corrupt every field of tiny fixtures. |
| DB11 Pages, allocation and buffer pool | DB06, DB10 | Page IDs, slotted pages, overflow, freelist, pinning, dirty tracking and eviction. Model short/torn I/O and ensure pinned frames and dirty-page flush barriers honor their contracts. |
| DB12 WAL and recovery | DB04, DB06, DB10 | Log sequence numbers, checksummed framing, flush, commit, replay and checkpointing. Exhaustively inject bounded crash cuts, including recovery interrupted by another crash. Prove or check idempotence and survival of acknowledged commits under the specified device contract. |
| DB13 Page-backed B+ tree | DB09b, DB11, DB12 | Search, scans/cursors, split, merge, delete, overflow and persisted root updates. Compare random operations with a sorted reference map, reopen after writes, and check invariants and recovery around splits. |
| DB14 Concurrency model and harness | DB-A06 | Deterministic yield points, replayable schedules and bounded history checkers. Keep a native threaded stress runner for real execution. Negative controls include lost updates, deadlocks, visibility races and premature publication. |
| DB15 Concurrent B+ tree and buffer pool | DB13, DB14 | Page latches, latch ordering, pin/unpin synchronization and split publication. Check scans against the declared consistency model, lifetime safety, deadlock policy and progress under contention. |
| DB16 Transactions and isolation | DB12, DB14, one storage engine | Start with lock-based transactions and deadlock handling, then MVCC, snapshots, garbage collection, and isolation profiles. Test write skew, phantoms, rollback, savepoints and crash/schedule combinations. Pin the transaction outcome semantics at each supported isolation level. |

## LevelDB and RocksDB from scratch

Implement a LevelDB-style engine from the shared components first. Build a
separate RocksDB-style profile on that foundation with its own API, workloads,
and compatibility claims. Inventory behavior against pinned upstream versions
before claiming compatibility. Treat upstream LevelDB and RocksDB as optional
independent test oracles, with no dependency on their engines at runtime.

| Item | Depends on | Deliverable and acceptance |
|---|---|---|
| DB20 Ordered memtable | DB10, DB-A02 | Internal keys ordered by user key and sequence, tombstones, one-writer snapshots and immutable rotation. Compare point and range results as seen at every sequence on small traces. |
| DB21 SSTable and block cache | DB09c, DB10, DB-A03 | Prefix-compressed blocks, restart points, index and filter blocks, footer, checksums, cache policy and pluggable compression. Audit seek boundaries and malformed offsets. Check byte compatibility independently where declared. |
| DB22 Versions, manifests and recovery | DB12, DB20, DB21 | Log and manifest records, atomic `CURRENT` publication, file numbering, reference lifetimes, orphan cleanup and open/reopen. Cut failures at every publish boundary on bounded traces. |
| DB23 LevelDB-style API | DB22 | `Get`, `Put`, `Delete`, atomic write batches, ordered iterators, snapshots and comparators. Verify ordering, overwrite/delete semantics, reopen, and iterator validity. Separate API parity from file compatibility. |
| DB24 Leveled compaction | DB23 | L0 overlap, level scheduling, merge iterators, tombstone elision, oldest-snapshot retention and backpressure. Compare reads before and after compaction and measure read, write and space amplification. |
| DB25 Threaded LevelDB profile | DB14, DB24 | Foreground readers/writers, grouped writes, background flush/compaction, shutdown and disk-full behavior. Check publication races, concurrent snapshots, error propagation, and replay against the sequential model. |
| DB26 RocksDB concurrent I/O | DB25, DB07 | Concurrent flush/compaction, subcompactions, configurable priority pools, rate limits, write stalls and asynchronous I/O. Audit job lifetimes, fairness policy and bounded backpressure. |
| DB27 RocksDB table and read features | DB26 | Configurable table/index/filter layouts, cache partitioning, prefix seek, `MultiGet`, iterators, statistics and checksums. Test each option with a differential corpus and measure reads on normal and skewed workloads. |
| DB28 RocksDB keyspace and mutation features | DB26 | Column families, atomic batches across families, merge operators, range tombstones, TTL, blob separation, and optimistic/pessimistic transactions. Give each capability its own semantics, crash cases and compatibility profile. |
| DB29 RocksDB administration | DB27, DB28 | Checkpoints, backup/restore, external SST ingestion, compaction strategies (leveled, universal and FIFO), tuning, repair and version/format compatibility. Pin data visibility through failures and test restore into a fresh engine. |

The upstream inventory should pull API and behavior from the pinned
[LevelDB source](https://github.com/google/leveldb) and
[RocksDB source](https://github.com/facebook/rocksdb), including feature
interactions. Any row above containing multiple features breaks into one
spec and delivery item per feature during refinement.

After DB13 and DB24, run the same point reads, sequential scans, ordered and
random writes, and hot-key workloads through both engine families on HDD and
FTL backends. Compare device reads, host/physical writes, space overhead,
compaction or page-split costs, and modeled tail latency. Keep workload,
durability settings, and acknowledgement points fixed so the comparison
teaches the storage-engine tradeoff.

## Relational engines and comparison projects

| Item | Depends on | Deliverable and acceptance |
|---|---|---|
| DB30 Relational catalog and constraints | DB13, DB16 | Catalog/schema versions, typed rows, indexes, constraints, and transactional schema changes. Check boundaries for concurrent schema changes and crash during DDL. |
| DB31 SQL frontend and semantics | DB30, DB-A05 | Reuse only audited parts of the SQL frontend. Pin dialect behavior for nulls, values, expression semantics, joins, aggregate/grouping, subqueries, CTEs, windows and DDL/DML one feature at a time. |
| DB32 Planner and execution | DB31 | Cost/cardinality model, indexed access, join algorithms, sort/aggregation, spill and parallel operators. Verify plan rewrites preserve result semantics and add explain and workload checks. |
| DB33 SQLite-style embedded profile | DB13, DB30–DB32 | First-class file and SQL profiles, pager/rollback and WAL modes, snapshots, one-writer discipline, import/export and integrity checking. Cross-check files and queries independently, with published coverage boundaries. |
| DB34 PostgreSQL-style server profile | DB15, DB16, DB31, DB32 | Client protocol, concurrent sessions and worker pools, shared buffers, MVCC, vacuum, checkpoint/restart, authentication/authorization and query cancellation. Add WAL archiving, point-in-time restore and replication as separately qualified capabilities. |
| DB35 LMDB-style embedded profile | DB10, DB13, DB14 | Copy-on-write B+ tree, dual metadata pages, reader snapshots, page reclamation and single-writer contention. Check crash publication and reader reclamation independently. |
| DB36 DuckDB-style analytical profile | DB11, DB31, DB32 | Columnar row groups, encoding/compression, vectorized scans, predicate pushdown, parallel operators, spill and analytical file integration. Compare fixed analytical results and track I/O and memory costs. |
| DB37 Redis-style profile | DB-A02, DB-A04, DB14 | Audit the existing datastore and protocol, then qualify commands, expiration, eviction, snapshots, append-only recovery and concurrency. Keep this track separate from relational compatibility. |
| DB38 Distributed profiles | DB25 or DB34 | Add replicated logs, consensus, partitions, consistency models, failover, distributed transactions, and explicit availability bounds as separate work. Check histories under message loss, delays and process restarts. |
| DB39 Production readiness | Each engine profile | Workload-based throughput and latency, metrics and tracing, quotas, disk-full and corruption handling, safe upgrades, offline repair, backup drills and long-running threaded tests. Record operating limits and hardware assumptions per profile. |

## Polyglot delivery

Use one contract, fixture vocabulary, and acceptance matrix per package. Track
every lane present at the initial audit (`c`, `cpp`, `csharp`, `dart`, `elixir`,
`fsharp`, `go`, `haskell`, `java`, `kotlin`, `lua`, `mosaic`, `ocaml`, `perl`,
`python`, `ruby`, `rust`, `starlark`, `swift`, `twig`, `typescript`, `wasm`) and
inventory new lanes as the repo grows. Record spec, implementation, conformance,
evidence, and execution status separately. The initial lanes use shared JSON
fixtures with exact errors, bytes, and observable state; adapt those into the
repo's evidence machinery as the corresponding Warrant contracts land.

1. Start with the two Python media leaves and shared fixtures. Once their
   contracts stabilize, add independently written Rust or C ports to expose
   portability gaps, then bring each remaining lane up against the same cases.
2. Port each graduated contract before using that guarantee in the same lane's
   next package. Keep backend tests interoperable in both directions where a
   file or wire format is promised. An FFI binding and an independent
   implementation are separate matrix statuses.
3. Give each native engine a threaded runner with concurrent foreground clients
   and background I/O. For runtimes with limited threading, specify whether the
   implementation uses host workers, runtime concurrency or a deterministic
   scheduler only. Validate the portable semantics everywhere and claim actual
   parallel execution only where the host runner demonstrates it.
4. For every engine profile, keep a versioned feature inventory with per-port
   rows for API semantics, file/wire compatibility, isolation, durability,
   threaded execution, and evidence. A passthrough or partial implementation
   has an explicit matrix state. Close a row only when its declared scope is
   covered, and qualify the composed stack with end-to-end traces.

**Next handoff:** After DB01 and DB02 are locally validated, choose one second
lane for both leaves and specify DB04's crash/fault vocabulary. In parallel
with that work, audit DB-A01 and the precise DB-A02/DB-A04 candidates needed
for the first durable storage slice. Keep the simulators under the database
test harness so device behavior can be replayed from a failing engine test.
