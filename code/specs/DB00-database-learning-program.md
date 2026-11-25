# DB00 — Database learning and implementation program

Owner direction: 2026-11-25. Build inspectable databases from storage devices to
query execution, with native implementations throughout the repository. This is
a delivery plan, not a claim that its proposed packages already exist.

## Delivery rules

- Small packages, narrow interfaces, literate source and explicit assumptions.
- Native implementations in all 18 implementation lanes: C, C++, C#, Dart,
  Elixir, F#, Go, Haskell, Java, Kotlin, Lua, OCaml, Perl, Python, Ruby, Rust,
  Swift and TypeScript. Emerging C/C++/OCaml status does not remove them from
  this program. WASM is an execution target, tracked separately; Mosaic and
  Twig are application/teaching integration tracks, not silently counted as
  native database implementations. Starlark remains build configuration.
- Shared semantic contracts and independent expected results, not wrappers
  around a Rust engine and not one port treated as an infallible oracle.
- Every milestone has a per-language implementation and validation matrix.
  Directory presence is not conformance. Missing tools are reported as not-run.
- Warrant is progressing independently. Exhaustive finite-domain unit tests,
  property tests, seeded fuzzing, fault injection, mutation controls and
  integration tests proceed now. Later Warrant bindings reuse these contracts.
  No handwritten proof labels or claims of exhaustive coverage of unbounded
  states. Follow VER27, VER29 and the effective parts of VER31.
- A milestone closes only when every required lane passes its applicable gates;
  an individual package can be reviewed in a smaller PR without closing it.
- Preserve existing SQL work. A parser is one consumer of storage, not the root
  abstraction. Reconcile current code with old roadmaps before scheduling ports.

## Architecture and substitution

Device behavior and protocol are separate: NAND is a storage medium, an SSD
controller translates it, and NVMe is a command/queue interface. HDDs expose
rewritable sectors but introduce seek and rotational latency. A database can
run against deterministic device models or a real OS adapter using the same
declared persistence contract.

Each layer publishes exact guarantees and cites exact lower-layer assumptions:

1. MOSFET audit and explicit flash-cell models (discrete first; analog later).
2. Raw NAND pages/erase blocks and HDD sectors/mechanics.
3. Flash controller: mapping, overprovisioning, reclamation, wear and recovery.
4. Block-device contract: submitted/completed/persistent are distinct states.
5. Device queues, barriers, flush/FUA, NVMe submission/completion modeling.
6. Page codec, checksums, allocator and buffer pool.
7. Heap files, B+ trees, SSTables, Bloom filters and LSM trees.
8. WAL/journals, checkpoints, recovery and backup.
9. Threaded execution, locks/latches, deadlock detection, MVCC and isolation.
10. Key-value engines and relational execution, optimization and SQL facade.
11. Operations, observability, load control, performance and deployment limits.

## Verification contract

For each public operation record: accepted input domain, invalid input results,
explicit state, state transition, failure atomicity, resource bounds, ownership,
thread safety, persistence point and independent test oracle. Publish the exact
tested bounds, seeds, fault positions, toolchain and source revision with results.

Use exhaustive enumeration where the entire scalar domain is small (for example
all 65,536 old/requested byte pairs). Enumerated device histories are bounded
sequence tests: print geometry, alphabet and maximum depth. Fuzz larger states
and malformed input with deterministic seeds and regression fixtures. Mutation
controls must demonstrate that the checks reject meaningful wrong behavior.
Coverage targets follow repository policy (95% preferred for libraries), with
branch/error coverage and invariants as well as percentages.

Persistence tests interrupt every modeled write/flush boundary, include torn and
reordered writes, and compare recovered state with the declared transaction
contract. A successful write completion never implies durability by accident.

Concurrency has two independent gates: deterministic scheduling explores named,
bounded interleavings; real worker threads exercise the same native engine.
Check linearizability for KV operations and the promised transaction isolation
for SQL histories. Include race/deadlock diagnostics and timeouts, memory safety
instrumentation for C/C++, and publication/visibility tests under each language's
memory model. Neither simulation nor stress testing alone proves all schedules.

## Feature progression

Build two engine families from the same lower layers: a B+ tree/page engine and
a LevelDB-style LSM KV engine. Add RocksDB-style capabilities separately after
the minimal LSM is correct. These are educational implementations initially;
LevelDB/RocksDB API, binary format and performance compatibility require distinct
versioned conformance targets and cannot be implied by a similar design.

Relational features proceed one capability at a time: values/NULL semantics,
catalog/schema, CRUD, transactions, scans/indexes, filters/projections, joins,
aggregation/sorting, constraints, subqueries/CTEs/set operations, views/triggers,
window functions and advanced optimization. SQLite compatibility remains its
own oracle-gated track. Distributed replication, consensus, sharding and failover
follow a correct single-node multithreaded engine.

## First delivery

The first executable slice is the finite NAND programming rule, natively in all
18 lanes, with exhaustive tests. It is a behavioral primitive, not a transistor
physics simulation or a full SSD. Raw page/block state and the HDD model follow
the contracts in DB01 and DB02. See DB-BACKLOG for dependencies and acceptance.
