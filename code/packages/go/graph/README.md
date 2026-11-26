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
```
