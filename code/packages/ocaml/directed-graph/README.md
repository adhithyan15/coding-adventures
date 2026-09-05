# directed-graph

An OCaml implementation of the portable directed-graph contract. The `Make`
functor keeps ordered forward and reverse adjacency maps so edge orientation,
properties, and dependency queries remain independent and deterministic.

The package provides directed mutation, weighted properties, BFS/DFS,
topological sorting, independent execution groups, transitive closure and
dependents, affected-node analysis, strongly connected components, optional
self-loops, and multi-label directed edges. Adding an edge creates missing
endpoints after validating its weight and self-loop policy.

## Dependencies

- graph

## Development

```bash
# Run tests
bash BUILD
```

The build runs ocamlformat checks, Alcotest, and nonempty bisect_ppx coverage
with a 95% production-line minimum.
