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
yields a spec-first change with a narrow API, stable claims, owned assumptions,
and evidence for the exact language and revision tested. Write and run those
checks manually now. Warrant can adopt the same claims as its tooling matures.
Use the existing release tiers for any release.

| Evidence | Per-port expectation |
|---|---|
| Unit and integration | Exact edge cases, typed refusals, mutation and resource bounds, plus the behavior of a caller at each admitted dependency boundary. |
| Shared conformance | Replay the same portable traces in every lane and compare output, error and observable state. Byte fixtures are hex; timing and schedules use explicit integer units. |
| Property tests | Generate valid inputs and stateful traces against an independently written oracle where practical. Pin seeds, preserve minimized failures as regression cases, and check relevant invariants after each transition. |
| Fuzz tests | Exercise malformed data, invalid commands, unexpected ordering and adversarial size boundaries. Pin input corpus, budgets and replay commands. Use coverage-guided engines where available and identify seeded fuzz runs separately. |
| Bounded exhaustive checks | Enumerate declared small domains, histories, or crash/scheduler decisions. Record the precise bounds and explore both successful transitions and rejection with unchanged state. |
| Negative controls and capability checks | Demonstrate representative broken variants are detected and audit effects using the repository's available runtime-capability tooling. Measure coverage alongside these checks. |
| Formal verification preparation | Specify observable state, input domains, transition preconditions, invariants and failure semantics separately from representation. Link each upstream assumption to its owner and each claim to its manual evidence. Record outstanding proof and runtime gaps. |

### Delivery loop

After each verified merge, reconcile the queue with fetched `origin/main`,
open PRs, and any discoveries from implementation or review. Prioritize fixes
that protect admitted guarantees first, then missing evidence, then dependencies
blocking the next smallest learning feature. Assign one bounded item at a time
to a Luna implementation agent and keep the number of active agents small.
Commit the spec first and review database correctness and evidence at the
exact head before pushing. Monitor the
required PR checks, resolve failures and conflicts, arm auto-merge for the
reviewed head when checks are healthy, and confirm the exact head merged. When
CI is backlogged, continue preparing one bounded item at a time on local
branches and hold new pushes until capacity returns. Reconcile each local
branch against the eventual merged base before pushing. Record local
verification separately from a verified merge. Split new findings into their
own backlog rows unless they block the current item's acceptance.

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

### Evidence infrastructure queue

| Item | Priority trigger | Acceptance |
|---|---|---|
| DB-E01 Shared fixture CI routing | Local checks passed for Python and TypeScript readers | Exact fixture dependencies select the matching readers and change their cache keys. Extend the registry as new lanes land. [Spec](DB-E01-media-fixture-routing.md) |
| DB-E02 Media claim and mutation corpus | Before accepting the first higher-layer use of a media guarantee | Give state transitions stable claim IDs and add representative negative controls for illegal programming, block isolation, elapsed timing and rejected-operation atomicity. Keep generated evidence and proof migration separate from native test results. |
| DB-E03 Lane admission inventory | Before the first medium port to each constrained or unbootstrapped lane | Record available toolchain, binary data representation, integer bounds, test runner, and native-vs-host runtime scope. Add small enabling items for gaps discovered during porting. |
| DB-E04 TypeScript project inventory | With every new TypeScript port | Run the tracked-project portability audit and reconcile its exact counters with the new package's compiler inputs. The NAND PR exposed this gate at [metadata job 139543001686](https://github.com/adhithyan15/coding-adventures/actions/runs/45709375026/job/139543001686). Keep the portability check active. |

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

Both engines are useful as embedded ordered key-value stores. A relational
engine can also use one under its catalog, transactions and SQL execution. That
integration needs explicit contracts for key encoding, range iteration,
snapshot visibility, and atomic changes to rows and secondary indexes.

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
| DB29a Relational adapter for LSM storage | DB24, DB30, DB16 | Encode rows and secondary indexes as ordered keys. Compare the same SQL workload over B+ tree and LSM backends. Verify atomic row/index updates, visibility across snapshots, uniqueness, and recovery through the adapter. |

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
| DB36 Columnar storage and pruning | DB10, DB11, DB-A03 | Build column vectors, null bitmaps and row groups. Add dictionary, run-length and bit-packed encodings, compression, zone maps and projection pruning in separate slices. Compare reconstruction to a row oracle and measure bytes decoded per query. |
| DB36a DuckDB-style analytical engine | DB31, DB32, DB36 | Build vectorized scans, filter/projection, grouped aggregation and joins, then parallel execution and spill. Compare row and vector results under the same query semantics. Add Parquet read/write as a separate interoperability target with independent file fixtures. |
| DB36b ClickHouse-style columnar server | DB36a, DB14, DB24 | Insert immutable sorted columnar parts, add sparse indexes and merge parts in the background. Check deterministic merge results, partition pruning, concurrent reads and publication across crashes. Add sharding and replication through DB40–DB42 after the single-node contract. |
| DB37 Redis-style profile | DB-A02, DB-A04, DB14 | Audit the existing datastore and protocol, then qualify commands, expiration, eviction, snapshots, append-only recovery and concurrency. Keep this track separate from relational compatibility. |
| DB39 Production readiness | Each engine profile | Workload-based throughput and latency, metrics and tracing, quotas, disk-full and corruption handling, safe upgrades, offline repair, backup drills and long-running threaded tests. Record operating limits and hardware assumptions per profile. |

## Partitioning and distributed systems

| Item | Depends on | Deliverable and acceptance |
|---|---|---|
| DB40 Partitioning and shard routing | DB25 or DB34 | Implement single-node partitions first, then hash and range sharding with versioned routing tables. Check range boundaries, skew, scatter/gather and handling of stale routing metadata. |
| DB41 Shard movement and rebalancing | DB40 | Split and move a live shard, transfer snapshots and catch up updates. Exercise retries, aborts, stale clients, and crashes across cutover. Track which owner may accept writes at each transition. |
| DB42 Replication and failover | DB12, DB14, DB40 | Single-leader replication, quorum/consensus, snapshots and failover. Use a deterministic network with drops, reordering, delays and partitions to check committed-prefix agreement and observable availability bounds. |
| DB43 Distributed queries and transactions | DB40–DB42, DB16 | Cross-shard operators, transaction coordination, participant recovery and retries that preserve operation identity. Check the declared isolation/consistency guarantees against independent histories under process and network faults. |

## Additional database models

| Item | Depends on | Deliverable and acceptance |
|---|---|---|
| DB50 Document store | DB23 or DB13, DB16 | Encode nested values, query paths, update documents atomically, and maintain secondary indexes. Verify index contents and snapshot visibility after arbitrary valid mutations and crashes. |
| DB51 Wide-column store | DB24, DB40–DB42 | Partition-key and clustering-key encodings, sorted partitions, tombstones, deletes and repair. Specify conflict resolution before testing convergence under reordered writes and replication faults. |
| DB52 Graph store | DB13 or DB23, DB16 | Persist node/edge identity, properties and adjacency indexes. Compare traversals to a reference graph and test atomic updates to both sides of an edge. |
| DB53 Time-series store | DB24 or DB36 | Time partitions, out-of-order inserts, retention, time-window aggregations and downsampling. Verify late data, window boundaries, retention races and crash recovery. |
| DB54 Vector and search indexes | DB13 or DB23, DB-A02 | Build exact search as a reference, then inverted and approximate-neighbor indexes with explicit recall, update and persistence contracts. Run deterministic quality and workload comparisons. |

Each profile states its data model, shared storage dependencies, and supported
operations. Sharing a storage engine does not imply sharing an API, consistency
model, query language or distributed design.

## Polyglot delivery

Use one contract, fixture vocabulary, and acceptance matrix per package. Track
every lane present at the initial audit (`c`, `cpp`, `csharp`, `dart`, `elixir`,
`fsharp`, `go`, `haskell`, `java`, `kotlin`, `lua`, `mosaic`, `ocaml`, `perl`,
`python`, `ruby`, `rust`, `starlark`, `swift`, `twig`, `typescript`, `wasm`) and
inventory new lanes as the repo grows. Record spec, implementation, conformance,
evidence, and execution status separately. The initial lanes use shared JSON
fixtures with exact errors, bytes, and observable state; adapt those into the
repo's evidence machinery as the corresponding Warrant contracts land.

1. Start with Python and TypeScript media leaves and shared fixtures. Port to
   C and another available native lane next, then qualify the remaining lanes
   against the same cases. Advance in small batches and retain one backlog row
   per port until its manual validation is complete. Every lane is in scope.
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

### Media port queue

Track DB01 and DB02 separately. `Queued` means the language has no validated
implementation in this initiative yet. For constrained hosts, first record the
language's byte, state, I/O, and concurrency capabilities and specify which
layers can be implemented natively. A host binding remains a separate result.

| Lane | DB01 NAND | DB02 HDD |
|---|---|---|
| C | Queued | Queued |
| C++ | Queued | Queued |
| C# | Queued | Queued |
| Dart | Queued | Queued |
| Elixir | Queued | Queued |
| F# | Queued | Queued |
| Go | Queued | Queued |
| Haskell | Queued | Queued |
| Java | Queued | Queued |
| Kotlin | Queued | Queued |
| Lua | Queued | Queued |
| Mosaic | Queued | Queued |
| OCaml | Queued | Queued |
| Perl | Queued | Queued |
| Python | Local checks passed | Local checks passed |
| Ruby | Queued | Queued |
| Rust | Queued | Queued |
| Starlark | Queued | Queued |
| Swift | Queued | Queued |
| Twig | Queued | Queued |
| TypeScript | Local checks passed | Local checks passed |
| WebAssembly | Queued | Queued |

**Next handoff:** Qualify DB01 and DB02 lane by lane, specify DB04's crash/fault
vocabulary, and audit DB-A01 and the precise DB-A02/DB-A04 candidates needed
for the first durable storage slice. Keep the simulators under the database
test harness so device behavior can be replayed from a failing engine test.
