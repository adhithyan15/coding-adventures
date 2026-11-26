# DT00B — Concurrent Go undirected graph

## Purpose and dependency

DT00B specifies an opt-in, concurrency-safe wrapper for the Go DT00 undirected
graph package. The existing `*Graph` remains a single-owner, unsynchronized
type. The wrapper owns one `*Graph` exclusively and serializes access with one
`sync.Mutex`; callers never receive that backing pointer or any of its maps,
slices, rows, or property bags.

The constructor initializes the owner. The zero value of `ConcurrentGraph` is
not usable. Copying an initialized `ConcurrentGraph` value copies only a handle:
all copies share the same private state, mutex, and owned graph. A snapshot's
zero value is likewise not a valid snapshot; snapshots returned by `Snapshot`
are immutable and may be copied freely.

This work starts only after DS-GRAPH-WEIGHTS-GO (#20141) merges. Its weight
acceptance and error behavior are inherited from the resulting sequential Go
package. DT00B does not change the shared DT00 contract or add synchronization
to the sequential API.

The wrapper preserves the sequential package's existing mutation semantics:
`AddEdge` still maps zero to one and stores negative or nonfinite float weights;
`SetEdgeProperty("weight", ...)` stores numeric negative or nonfinite values.
It does not turn shortest-path restrictions into mutation validation. A
snapshot's shortest-path query delegates to the sequential algorithm and
returns its finite, non-negative-weight error, including for `start == end`
and for a disconnected destination when an invalid edge exists elsewhere in
the graph. Minimum-spanning-tree queries continue to accept finite negative
weights. A future mutation-checked DT00 API is a separate work item.

## Contract

Every public operation on the wrapper is linearizable: it has one effective
instant between invocation and return. Each mutation, including both
directions of an undirected edge and its weight/property metadata, executes in
one exclusive critical section. A failed mutation leaves the complete graph
state unchanged, following the sequential package's error contract.

The wrapper covers both `AdjacencyList` and `AdjacencyMatrix`. It exposes the
representation as a scalar and creates no representation-specific locking
rules. No user callback, comparator, visitor, or iterator body executes while
the mutex is held.

### Small API surface

The implementation should add a separate `ConcurrentGraph` and a constructor
that accepts a `GraphRepr`. The surface is intentionally limited to:

- the existing node, edge, and graph-property mutations, with the wrapper's
  ordinary single-operation reads needed by callers;
- `Snapshot`, returning a read-only `GraphSnapshot` value with the existing
  query operations; and
- an edge iterator created from a snapshot, with a cursor owned by that
  iterator.

Use `NewConcurrentGraph`, `ConcurrentGraph`, `GraphSnapshot`, and
`EdgeIterator` as the public names. Wrapper mutation methods mirror the
sequential graph methods; mutators that accept property values return an error
when a value is not in the concurrent scalar profile. These checks happen
before any graph mutation. A wrapper method must not return a pointer to the
owned `Graph`, a backing map/slice, or a live iterator. A snapshot is a detached
point-in-time value and remains unchanged after later wrapper mutations. Its
queries include the sequential graph's read operations and the existing
shortest-path and minimum-spanning-tree algorithms. An atomic method combining
a check and mutation is added only if a concrete caller needs that operation;
separate `HasEdge` and `RemoveEdge` calls may interleave.

### Snapshot and property ownership

Snapshot construction holds the mutex while copying all graph structure
and metadata, then releases the lock before any query, iteration, or callback
work. It copies the node collection, adjacency maps or matrix rows, graph,
node, and edge property maps, and all representation indexes. Copying costs
O(V+E+P) for an adjacency list and O(V^2+P) for a matrix, where P is the
number of stored property entries. Snapshot iterators retain their own
snapshot and cursor. Each iterator is single-consumer state and must not be
advanced concurrently through the same handle. Copying an iterator copies its
cursor position and creates an independent cursor over the same immutable edge
slice; each copy advances from that position independently. Consumers that
need a fresh traversal should create a new iterator from the snapshot.

DT00's Go `PropertyBag` uses `any`, but its portable profile permits only
scalar values: strings, booleans, numeric scalar values, and nil. Snapshot
copies the map containers and preserves these immutable scalar values by
value. The concurrent API does not promise safety for pointers, maps, slices,
or other reference-bearing property values. Such values are outside the
concurrent profile; they must not be accepted by the wrapper or be used in
concurrent calls. Callers must not mutate an input property map until its
method returns; after return, the wrapper retains no alias to that map.
Read APIs and snapshots must return independent map containers. Tests cover
the portable scalar categories and prove that modifying an input or returned
map cannot modify wrapper state.

### Locking discipline

Mutations, single-value reads, and collection copies use the mutex. Methods
that return collections either copy them under the mutex or use a snapshot;
they never return internal storage. Multi-step algorithms and
iterators operate only on a snapshot after lock release. The mutex is not
reentrant, and code under it must not call user code or block on external
work. The wrapper does not promise fairness or wait-free progress.

## Validation gates

Tests must run against both representations wherever behavior is shared:

- deterministic barrier/channel schedules for reader/writer interleavings,
  including repeated add/remove of the same edge and an edge's two directions
  plus metadata;
- snapshot consistency during concurrent mutations, including independent
  snapshot iterators and cursor exhaustion behavior;
- bounded concurrent histories compared with a sequential model, preserving
  real-time order for non-overlapping calls;
- mutation and read-isolation checks for graph/node/edge scalar properties,
  including caller map mutation and returned map mutation;
- repeated stress schedules under `go test -race ./...` and ordinary package
  tests.

Semantic history checks and the race detector are separate gates: history
checks establish operation-level consistency for the explored bounded
histories; the race detector reports instrumented memory races in schedules
that were actually executed. No benchmark threshold is a correctness gate.

The pilot closes when both representations pass the semantic suite, repeated
race-enabled stress runs pass, the normal Go graph package checks pass, and
the public API has no path to the backing graph or storage. If a representation
cannot meet the same snapshot or metadata contract, record the reason and stop
before silently weakening the contract.

## Supplemental bounded SAT history check

A packaging-independent SAT check was run against the final wrapper source
(blob `656ab648499c7baf21852ea546a453dd693b68fe`) and SAT solver source
(blob `8bb3e2509d17697b32ae49624300c27d98263525`). It modeled 36 actual
three-, four-, and five-operation histories, requiring a sequential
linearization that respects real-time ordering, and replayed each satisfying
witness against the sequential model. Each history also had an impossible
observation control checked as unsatisfiable. The run reported 72 solver calls,
2 distinct observed `HasEdge`/snapshot outcomes, and 25 overlapping operation
pairs; Go 1.26.1 `go test -race -count=1 -v ./...` and `go vet ./...` passed
in the scratch module.

This supplemental scratch evidence is not part of CI and is not an E5 claim,
a proof for all histories, or a race-freedom proof. The tracked wrapper fuzz
and race-enabled tests remain the package validation gates.
