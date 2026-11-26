# DT00 Go profile: shortest-path weight preconditions

The Go `graph.ShortestPath` operation uses Dijkstra's algorithm. Before endpoint
shortcuts or path search, it validates every stored edge in the graph, including
edges in components unrelated to the query. A weight is valid for shortest-path
search only when it is finite and non-negative. If any edge has a negative,
NaN, or infinite weight, `ShortestPath` returns an error and no path.

This validation applies to both adjacency-list and adjacency-matrix graphs, and
to same-node queries. It does not change `AddEdge`'s existing zero-means-default
behavior, the `AddEdge` signature, or the ability to store finite negative
weights. This keeps existing callers source-compatible and leaves algorithms
such as minimum spanning tree free to operate on finite negative weights.

For valid weights, endpoint errors and the existing path result behavior are
preserved. Validation costs O(E) for adjacency lists and O(V²) for adjacency
matrices, consistent with their stored-edge iteration behavior. The
implementation uses positive infinity as its unreachable-distance marker so a
finite representable path distance is not mistaken for unreachable merely
because it exceeds a finite sentinel. Correct weighted path ordering is scoped
to cumulative distances representable by `float64`; handling sums that overflow
that domain remains a follow-up.

## Verification evidence

The Go package's unit tests cover invalid stored weights and API compatibility.
A deterministic model property exhausts the 3^6 four-node undirected graphs
whose six possible edges are absent, weight 1, or weight 2. For both graph
representations and every start/end pair, it compares `ShortestPath` reachability
and returned path cost with an independent simple-path enumerator. A fixed
negative control confirms that the oracle rejects a known non-shortest path.

A native Go fuzz target maps bounded byte inputs to graphs of at most four nodes
with absent, valid, and invalid weights. Its seed corpus is checked in with the
tests, and the build runs a time-bounded fuzz smoke campaign. These checks are
reproducible finite evidence, not a proof for arbitrary graph sizes or
float64 values; the fuzz target also does not claim a VER06 engine campaign or
per-edge coverage report.
