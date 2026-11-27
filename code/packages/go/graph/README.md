# go/graph

Undirected weighted graph package with adjacency-list and adjacency-matrix
storage, plus the core DT00 algorithms.

```go
g := graph.New(graph.AdjacencyList)
g.AddNode("London", graph.PropertyBag{"kind": "city"})
g.AddEdge("London", "Paris", 300, graph.PropertyBag{"route": "train"})
g.AddEdge("London", "Amsterdam", 520)

properties, _ := g.EdgeProperties("Paris", "London")
fmt.Println(properties["weight"]) // 300

path, _ := graph.ShortestPath(g, "London", "Paris")
mst, _ := graph.MinimumSpanningTree(g)
```

Graph, node, and edge property bags are copied on read. The canonical edge
property `weight` is kept in sync with `EdgeWeight` and weighted algorithms.
Edge identity uses the exact unordered pair of endpoint strings, including
labels with embedded NUL bytes.

`ShortestPath` validates every stored edge before searching, including for a
same-node query. It returns an error if any weight is negative, NaN, or infinite.
`AddEdge` retains its existing zero-means-default behavior and permits finite
negative weights for algorithms such as `MinimumSpanningTree`.

The package tests also exhaust all 729 four-node graphs whose edges are absent,
weight 1, or weight 2 against an independent simple-path oracle for both
representations and every endpoint pair. The native Go fuzz target bounds input
graphs to four nodes and runs for five seconds in `BUILD` and `BUILD_windows`.
These are bounded tests, not a proof for arbitrary graph sizes or float64
distance overflow.

From this module directory, run the fuzz target manually with:

```sh
go test -run=^$ -fuzz=FuzzShortestPathWeightPreconditions -fuzztime=5s -parallel=1 .
go test -run=^$ -fuzz=FuzzConcurrentGraphModelHistories -fuzztime=5s -parallel=1 .
go test -run=^$ -fuzz=FuzzEdgeIdentityModel -fuzztime=5s -parallel=1 .
```

## Concurrent use

`Graph` is a single-owner type and is not safe for concurrent access. For
shared use, `ConcurrentGraph` owns the graph behind one mutex. Each operation
is atomic on its own, and `Snapshot` makes a detached read-only view for
multi-step queries and iteration:

```go
shared := graph.NewConcurrentGraph(graph.AdjacencyList)
if err := shared.AddEdge("London", "Paris", 300); err != nil {
    panic(err)
}

snapshot := shared.Snapshot()
path, err := snapshot.ShortestPath("London", "Paris")
if err != nil {
    panic(err)
}
fmt.Println(path) // [London Paris]
```

Concurrent property values must be immutable scalars (`string`, `bool`, numeric
scalars, or `nil`). Property maps are copied on input and output. A snapshot
does not change after the shared graph is modified; each edge iterator has its
own cursor.

Create concurrent graphs with `NewConcurrentGraph`; its zero value is not
usable. Copying an initialized `ConcurrentGraph` shares the same graph and
mutex. Copying an `EdgeIterator` forks its cursor at the current position;
concurrent advancement of one iterator handle is not safe.
