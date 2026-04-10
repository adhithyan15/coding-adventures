# go/graph

Undirected weighted graph package with adjacency-list and adjacency-matrix
storage, plus the core DT00 algorithms.

```go
g := graph.New(graph.AdjacencyList)
g.AddEdge("London", "Paris", 300)
g.AddEdge("London", "Amsterdam", 520)

path, _ := graph.ShortestPath(g, "London", "Paris")
mst, _ := graph.MinimumSpanningTree(g)
```
