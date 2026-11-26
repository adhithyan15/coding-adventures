# Go shortest-path SAT evidence

This test-only Go program independently checks bounded behavior of
`code/packages/go/graph.ShortestPath` using the repository's `sat-solver`
package. It is a separate discovered program package so its test dependencies
do not enter the graph package's published `go.mod`.

The harness exhausts every labeled undirected simple topology on two, three,
and four vertices. Each canonical edge is absent, weight 1, or weight 2. It
checks both adjacency-list and adjacency-matrix layouts and every ordered pair
of distinct endpoints: 17,832 cases total. It tests positive integer weights
that map exactly to `float64`; loops, parallel edges, zero/negative/non-finite
weights, arbitrary graph sizes, rounding, and cumulative overflow are outside
the claim.

The test contains two independent reference mechanisms. A small DFS enumerates
all simple paths as an oracle. The SAT encoder instead creates a positional
CNF for each exact length: one-hot vertex states, fixed endpoints, no repeated
vertices, only legal graph transitions, and one-hot cumulative integer costs
advanced by each edge weight. SAT models are decoded and replayed against the
original edge list. For reachable pairs, the returned graph path is compared to
the DFS optimum, and SAT must report UNSAT for a strictly cheaper path. A
negative control raises the claimed bound by one; SAT must return a valid
cheaper path. For unreachable pairs, every length's reachability query must be
UNSAT.

Failure messages include representation, vertex count, topology index, exact
edge list, ordered endpoints, solver outcome, and any SAT witness. This makes a
failing topology reproducible without relying on a random seed.

The solver returns UNSAT without a DRAT certificate, so this is bounded
solver-backed evidence, not an independently checked proof or an E5 claim. See
[DT01](../../../specs/DT01-go-shortest-path-sat-evidence.md) for the formal
finite-domain statement and encoding contract.

Run with Go 1.26.1 (the module pins `toolchain go1.26.1`):

```sh
go test -race ./... -cover
```

All dependencies are repository-local modules; no third-party module is
needed.
