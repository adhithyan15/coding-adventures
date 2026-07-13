# graph (C++)

An **undirected weighted graph**, header-only, ISO C++17. A faithful port of the
Rust [`graph`](../../rust/graph) crate, in namespace `ca::graph`: string-named
nodes, weighted undirected edges, heterogeneous property bags, and the standard
graph algorithms.

## What it does

- **Two representations** — `Repr::AdjacencyList` and `Repr::AdjacencyMatrix`,
  both faithfully implemented, producing identical observable output.
- **Ordered semantics** — backed by `std::map` / `std::set` (a direct analogue
  of Rust's `BTreeMap` / `BTreeSet`), so `nodes()`, `neighbors()`, `edges()`,
  and every traversal come out sorted and deterministic.
- **Property bags** — `PropertyValue` (string / number / bool / null) in ordered
  `PropertyBag` maps on the graph, its nodes, and its edges; every edge always
  exposes a `"weight"` property mirroring its numeric weight.
- **Algorithms** — free functions `bfs`, `dfs`, `is_connected`,
  `connected_components`, `has_cycle`, `shortest_path` (BFS for unit weights,
  else Dijkstra), and `minimum_spanning_tree` (Kruskal + union-find).

Where the Rust crate returns `Result`, this port throws `ca::graph::Error`
(carrying an `ErrorKind`). Edge sorting uses a faithful `total_cmp` (Rust
`f64::total_cmp`) so ordering matches even across ±0.

## API

```cpp
#include "graph.hpp"
namespace g = ca::graph;

g::Graph gr(g::Repr::AdjacencyList);
gr.add_edge("London", "Amsterdam", 520.0);
gr.add_edge("Amsterdam", "Berlin", 655.0);

auto path = g::shortest_path(gr, "London", "Berlin");   // {London, Amsterdam, Berlin}
auto mst  = g::minimum_spanning_tree(gr);
```

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
