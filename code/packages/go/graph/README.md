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
