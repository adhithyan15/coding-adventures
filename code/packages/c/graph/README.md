# graph (C)

An **undirected weighted graph** in pure ISO C17. A faithful port of the Rust
[`graph`](../../rust/graph) crate: string-named nodes, weighted undirected
edges, heterogeneous property bags on the graph / nodes / edges, and the
standard graph algorithms.

## What it does

| area | operations |
|------|------------|
| nodes | `add_node`, `add_node_props`, `remove_node`, `has_node`, `nodes` (sorted) |
| edges | `add_edge`, `add_edge_props`, `remove_edge`, `has_edge`, `edge_weight`, `edges` (sorted) |
| neighbors | `neighbors` (sorted), `degree` |
| properties | graph / node / edge property getters & setters; every edge exposes a synthesized `"weight"` property |
| algorithms | `bfs`, `dfs`, `is_connected`, `connected_components`, `has_cycle`, `shortest_path` (BFS for unit weights, else Dijkstra), `minimum_spanning_tree` (Kruskal + union-find) |

Every internal map is **ordered by key** (the Rust crate uses `BTreeMap`), so
traversals and listings are deterministic and sorted — matching the crate on the
shared conformance vectors.

## Representation

The Rust crate keeps two interchangeable internal layouts (adjacency list and
adjacency matrix) that produce identical observable output. This C port stores
the chosen `GraphRepr` (returned by `graph_repr`) but backs both with a single
ordered-adjacency model — the public behavior is identical across
representations, exactly as the crate's own dual-representation tests assert.

## Memory & safety

`graph_new` / `graph_free` bracket the lifetime. Result lists
(`GraphStrList`, `GraphEdgeList`, `GraphComponents`) hand back owned copies you
release with the matching `_free`. All growable buffers guard `size_t` overflow
in their doubling loop; every allocation failure surfaces as
`GRAPH_ERR_OUT_OF_MEMORY`. Verified clean under ASan + UBSan and the macOS
`leaks` tool (0 leaks).

## API

```c
#include "graph.h"

Graph *g = graph_new(GRAPH_ADJ_LIST);
graph_add_edge(g, "London", "Amsterdam", 520.0);
graph_add_edge(g, "Amsterdam", "Berlin", 655.0);

GraphStrList path;
graph_shortest_path(g, "London", "Berlin", &path);   /* London, Amsterdam, Berlin */
graph_str_list_free(&path);
graph_free(g);
```

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
