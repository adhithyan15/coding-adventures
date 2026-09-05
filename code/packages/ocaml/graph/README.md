# graph

An OCaml implementation of the portable generic undirected-graph contract.
The `Make` functor accepts any ordered node type and exposes identical,
deterministic behavior over genuine adjacency-list and dense adjacency-matrix
storage.

The package includes node, edge, graph, and typed property operations; BFS and
DFS; connectivity, component, and cycle analysis; Dijkstra shortest paths;
and Kruskal minimum spanning trees. Non-finite and negative weights are
rejected without mutation, while zero-weight edges remain representable.

## Development

```bash
# Run tests
bash BUILD
```

The build runs ocamlformat checks, Alcotest, and nonempty bisect_ppx coverage
with a 95% production-line minimum.
