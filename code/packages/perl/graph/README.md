# Graph (Perl) — DT00

An undirected weighted graph data structure implemented in pure Perl from scratch.

## Overview

This package provides a complete undirected graph implementation with:

- **Two representations**: adjacency list (default, O(V+E) space) or adjacency matrix (O(V²) space, O(1) edge lookup)
- **Core operations**: node/edge management, degree, neighbourhood queries
- **Graph algorithms**: BFS, DFS, shortest path (BFS or Dijkstra), cycle detection, connected components, minimum spanning tree, bipartite checking
- **Weighted edges**: default weight 1.0, customizable per edge
- **Literate programming**: inline documentation with examples and theory

## Installation

```bash
cpanm CodingAdventures::Graph
```

## Quick Start

```perl
use CodingAdventures::Graph;

# Create a graph with adjacency list (default)
my $g = CodingAdventures::Graph->new();
# or with adjacency matrix for dense graphs:
my $g_matrix = CodingAdventures::Graph->new('matrix');

# Add edges (creates nodes automatically)
$g->add_edge('A', 'B', 1.5);
$g->add_edge('B', 'C', 2.0);

# Node queries
$g->has_node('A');           # true
$g->nodes();                 # list of all nodes
$g->degree('A');             # number of neighbours

# Edge queries
$g->has_edge('A', 'B');      # true
$g->edge_weight('A', 'B');   # 1.5
$g->neighbors('A');          # ('B')

# Traversals
my @bfs = CodingAdventures::Graph::bfs($g, 'A');
my @dfs = CodingAdventures::Graph::dfs($g, 'A');

# Algorithms
my $connected = CodingAdventures::Graph::is_connected($g);
my $has_cycle = CodingAdventures::Graph::has_cycle($g);
my @components = CodingAdventures::Graph::connected_components($g);
my @path = @{CodingAdventures::Graph::shortest_path($g, 'A', 'C')};
my @mst = CodingAdventures::Graph::minimum_spanning_tree($g);
my $bipartite = CodingAdventures::Graph::is_bipartite($g);
```

## API

### Construction

- `new([$repr])` — Create graph. Repr: `'list'` (default) or `'matrix'`

### Node Operations

- `add_node($node)` — Add node (no-op if exists)
- `remove_node($node)` — Remove node and incident edges (dies if not found)
- `has_node($node)` — Check existence
- `nodes()` — Return list of all nodes
- `len()` — Return node count

### Edge Operations

- `add_edge($u, $v, [$weight])` — Add edge with weight (default 1.0)
- `remove_edge($u, $v)` — Remove edge (dies if not found)
- `has_edge($u, $v)` — Check existence
- `edges()` — Return list of `[$u, $v, $weight]` triples
- `edge_weight($u, $v)` — Get weight (dies if not found)

### Neighbourhood Queries

- `neighbors($node)` — Return list of neighbours (dies if node not found)
- `neighbors_weighted($node)` — Return hash `{neighbor => weight}`
- `degree($node)` — Return number of incident edges

### Algorithms (module functions)

- `bfs($graph, $start)` — Breadth-first search. Time: O(V+E)
- `dfs($graph, $start)` — Depth-first search. Time: O(V+E)
- `is_connected($graph)` — True if all nodes reachable. Time: O(V+E)
- `connected_components($graph)` — List of node lists. Time: O(V+E)
- `has_cycle($graph)` — True if graph contains cycle. Time: O(V+E)
- `shortest_path($graph, $start, $end)` — Shortest path. Time: O(V+E) or O((V+E) log V)
- `minimum_spanning_tree($graph)` — MST via Kruskal. Raises error if disconnected. Time: O(E log E)
- `is_bipartite($graph)` — True if 2-colorable. Time: O(V+E)

## Design Patterns

### Undirected Edge Symmetry

Because edges are undirected, every operation maintains symmetry:
```perl
$g->add_edge('A', 'B', 1.0);
$g->has_edge('A', 'B');  # true
$g->has_edge('B', 'A');  # true (symmetric)
$g->neighbors('A');      # includes B
$g->neighbors('B');      # includes A
```

### Representation Agnosticism

Algorithms work identically on both representations:
```perl
my $g_list = CodingAdventures::Graph->new('list');
my $g_matrix = CodingAdventures::Graph->new('matrix');

# Same API, same results, different internal structure
CodingAdventures::Graph::bfs($g_list, 'A');
CodingAdventures::Graph::bfs($g_matrix, 'A');
```

## Theory

A graph G = (V, E) consists of:
- **V**: vertices (nodes) — any hashable value
- **E**: edges — unordered pairs {u, v} with optional weights

Undirected means {u,v} = {v,u}. This implementation maintains full symmetry automatically.

## Testing

```bash
perl -Ilib t/00-load.t
perl -Ilib t/01-basic.t
```

Comprehensive test suite covers:
- Node/edge operations on both representations
- All algorithms with various graph topologies
- Edge cases (empty, single-node, disconnected)
- Error conditions

## Performance

**Adjacency List** (default):
- Space: O(V + E)
- Edge lookup: O(degree(u))
- Best for sparse graphs

**Adjacency Matrix**:
- Space: O(V²)
- Edge lookup: O(1)
- Best for dense graphs or when O(1) edge lookup is critical

## License

MIT
