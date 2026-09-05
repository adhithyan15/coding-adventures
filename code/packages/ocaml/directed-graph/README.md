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

## Installation

Install a released package with
`opam install coding-adventures-directed-graph`. From this source directory,
pin the local graph dependency and install the package:

```bash
opam pin add --no-action --working-dir --no-checksums -y coding-adventures-graph ../graph
opam install .
```

## Usage

```ocaml
module Graph = Coding_adventures_directed_graph.Make (String)

let () =
  let graph = Graph.create () in
  match Graph.add_edge graph "parse" "compile" with
  | Error (Graph.Self_loop node) ->
      Printf.eprintf "self-loop rejected at %s\n" node
  | Error _ -> prerr_endline "could not add dependency"
  | Ok () -> (
      match Graph.topological_sort graph with
      | Ok order -> Printf.printf "%s\n" (String.concat " -> " order)
      | Error Graph.Cycle -> prerr_endline "dependency cycle detected"
      | Error _ -> prerr_endline "could not sort dependencies")
```

Compile and run the example with:

```bash
opam exec -- ocamlfind ocamlc -linkpkg -package coding-adventures-directed-graph example.ml -o example.bc
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
