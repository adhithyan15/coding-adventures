# DT00 Rust profile: shortest-path weight preconditions

## Status and scope

This profile applies to `code/packages/rust/graph` and its Rust-backed WebAssembly
wrapper. It specifies a compatibility-preserving query-time guard for Dijkstra's
preconditions. It does not change the shared DT00 mutation contract, graph storage,
weighted MST behavior, or the public shape of the existing `shortest_path` and
`WasmGraph.shortestPath` functions. A checked Rust entry point lets new callers
distinguish an invalid weighted graph from an ordinary missing or unreachable path.

The existing Rust mutation APIs cannot report an error: `Graph::add_edge` and
`Graph::add_edge_with_properties` return `()`, and `set_edge_property` accepts any
`GraphPropertyValue::Number`. This profile keeps those signatures unchanged.
Invalid weights can remain stored; shortest-path queries detect them before search.
A future checked mutation API and graph-state atomicity policy require a separate
specification.

## Valid shortest-path weights

Every stored edge weight must be finite and nonnegative for a shortest-path query.
`+0.0` and `-0.0` are valid. Negative finite values, NaN, positive infinity, and
negative infinity are invalid. Validation examines all stored edges, including
self-loops and edges outside the portion reachable from the requested start node.
This global check prevents a query from hiding an invalid graph behind early exits
or disconnected components.

The guard is query-time only. It does not reject an invalid value when the edge is
added or its canonical `weight` property is changed. It does not rewrite the graph.

## Rust API

Add a public `InvalidPathWeight` error type with the offending canonical endpoint
pair and a useful `Display` implementation. It is separate from `GraphError`, so
adding it does not make existing exhaustive matches on `GraphError` fail to compile.

Add:

```rust
pub fn try_shortest_path(
    graph: &Graph,
    start: &str,
    end: &str,
) -> Result<Vec<String>, InvalidPathWeight>
```

`try_shortest_path` checks every edge first. If any edge is invalid, it returns
`Err(InvalidPathWeight)` before checking endpoint presence or the `start == end`
shortcut. On a valid graph, a missing endpoint or unreachable destination returns
`Ok(Vec::new())`, preserving the established path representation. A valid existing
`start == end` returns the one-node path.

Keep:

```rust
pub fn shortest_path(&Graph, &str, &str) -> Vec<String>
```

as a compatibility wrapper. It delegates to `try_shortest_path` and returns an
empty vector on `InvalidPathWeight`, so it terminates safely while preserving its
source and return-type compatibility. Callers that need to distinguish invalid
weights use the checked function.

`WasmGraph.shortestPath` keeps returning a JavaScript array. Because it calls the
compatibility function, an invalid stored weight yields an empty array; this profile
does not introduce a JavaScript exception or change the binding signature.

## Other weighted algorithms and known limits

`minimum_spanning_tree` is unchanged. It continues to accept negative finite edge
weights; Kruskal's algorithm remains valid for them. This profile does not define
or change MST handling of NaN or infinite weights.

Finite edge weights can still overflow when accumulated along a path. For example,
two `1e308` edges have finite individual weights but an infinite `f64` sum. The
current `Vec<String>` result cannot report arithmetic overflow separately from an
unreachable path. Define a checked distance representation and overflow outcome in
a follow-up; this profile does not claim to fix that behavior.

## Verification contract

- Unit tests cover invalid add/update weights in both graph representations, both
  signs of infinity, NaN, negative finite values, signed zero, and validation
  precedence for missing endpoints and `start == end`. They check that graph
  storage remains unchanged by the query.
- A replayable, seeded property suite compares both representations against an
  independent shortest-path oracle over small labeled undirected graphs with
  finite nonnegative discrete weights. Failure output identifies the seed, graph,
  representation, endpoints, and returned path. The suite includes mutations of a
  correct path or skipped validation as negative controls.
- Fuzz inputs are byte-replayable and decode into bounded graph construction and
  query operations. Native coverage-guided fuzzing is reported only when the
  repository or `cargo-fuzz` runner actually provides it; seeded replay tests are
  reported as such, not as native branch coverage.
- A bounded SAT miter may ask whether a path cheaper than the implementation's
  observed path exists in a finite discrete graph domain, using positional path
  variables, allowed transitions, terminal constraints, and a finite cost bound.
  Any SAT witness is replayed against the graph. A deliberately wrong claimed
  bound must produce a replayable cheaper path. Solver UNSAT is only a bounded
  solver result, not an E5 proof, unless an independently checked certificate is
  also produced.
- No result from these finite domains proves behavior for arbitrary graph sizes
  or all `f64` values. Path-sum overflow remains excluded.
