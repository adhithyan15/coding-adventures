# graph

An OCaml implementation of the portable generic undirected-graph contract.
The `Make` functor accepts any ordered node type and exposes identical,
deterministic behavior over genuine adjacency-list and dense adjacency-matrix
storage.

The package includes node, edge, graph, and typed property operations; BFS and
DFS; connectivity, component, and cycle analysis; Dijkstra shortest paths;
and Kruskal minimum spanning trees. Non-finite and negative weights are
rejected without mutation, while zero-weight edges remain representable.

## Installation

Install a released package with `opam install coding-adventures-graph`. From
this source directory, use `opam install .`.

## Usage

```ocaml
module Graph = Coding_adventures_graph.Make (String)

let () =
  let graph = Graph.create () in
  match Graph.add_edge ~weight:2.5 graph "A" "B" with
  | Error (Graph.Invalid_weight weight) ->
      Printf.eprintf "invalid edge weight: %g\n" weight
  | Error _ -> prerr_endline "could not add the edge"
  | Ok () -> (
      match Graph.shortest_path graph "A" "B" with
      | Ok path -> Printf.printf "%s\n" (String.concat " -> " path)
      | Error (Graph.Node_not_found node) ->
          Printf.eprintf "unknown node: %s\n" node
      | Error _ -> prerr_endline "could not find the path")
```

Compile and run the example with:

```bash
opam exec -- ocamlfind ocamlc -linkpkg -package coding-adventures-graph example.ml -o example.bc
opam exec -- ocamlrun example.bc
```

## Development

```bash
# Run tests
bash -e BUILD
```

The build runs ocamlformat checks, Alcotest, and verifies that bisect_ppx
measures the production source. The package's current measured line coverage
is above 95%; numeric threshold enforcement belongs to the cross-platform
package CI contract.
