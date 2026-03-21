# CodingAdventures DirectedGraph Native (Ruby)

A Ruby native extension wrapping the Rust `directed-graph` crate via [Magnus](https://github.com/matsadler/magnus). Provides the same API as the pure Ruby `coding_adventures_directed_graph` gem but with significantly better performance on large graphs.

## How It Fits in the Stack

This package sits at the same layer as the pure Ruby `directed_graph` package but uses a completely different implementation strategy:

| Package | Language | Implementation |
|---------|----------|---------------|
| `directed-graph` | Rust | Core algorithms (the source of truth) |
| `directed-graph-native` (Python) | Rust + PyO3 | Python wrapper around the Rust crate |
| `directed_graph_native` (Ruby) | Rust + Magnus | **This package** -- Ruby wrapper around the Rust crate |
| `directed_graph` (Ruby) | Pure Ruby | Educational implementation |

The Rust crate contains all the graph algorithms. This gem contains zero algorithm logic -- it is pure glue code that marshals types between Ruby and Rust.

## Prerequisites

- Ruby >= 3.1.0
- Rust toolchain (rustc, cargo)
- The `rb_sys` gem (installed automatically via Bundler)

## Installation

```bash
bundle install
rake compile
```

## Usage

```ruby
require "coding_adventures_directed_graph_native"

g = CodingAdventures::DirectedGraphNative::DirectedGraph.new

# Add nodes and edges
g.add_node("compile")
g.add_edge("compile", "link")
g.add_edge("link", "package")

# Query the graph
g.has_node?("compile")       # => true
g.has_edge?("compile", "link") # => true
g.nodes                      # => ["compile", "link", "package"]
g.edges                      # => [["compile", "link"], ["link", "package"]]
g.size                       # => 3

# Neighbor queries
g.predecessors("link")       # => ["compile"]
g.successors("link")         # => ["package"]

# Algorithms
g.topological_sort           # => ["compile", "link", "package"]
g.has_cycle?                 # => false
g.transitive_closure("compile") # => ["link", "package"]
g.independent_groups         # => [["compile"], ["link"], ["package"]]
g.affected_nodes(["compile"]) # => ["compile", "link", "package"]
```

## API Reference

### Node Operations

| Method | Description |
|--------|-------------|
| `add_node(name)` | Add a node (no-op if it already exists) |
| `remove_node(name)` | Remove a node and all its edges. Raises `NodeNotFoundError` |
| `has_node?(name)` | Check whether a node exists |
| `nodes` | Return a sorted array of all node names |

### Edge Operations

| Method | Description |
|--------|-------------|
| `add_edge(from, to)` | Add a directed edge (creates nodes if needed). Raises `ArgumentError` for self-loops |
| `remove_edge(from, to)` | Remove an edge. Raises `EdgeNotFoundError` |
| `has_edge?(from, to)` | Check whether an edge exists |
| `edges` | Return a sorted array of `[from, to]` pairs |

### Neighbor Queries

| Method | Description |
|--------|-------------|
| `predecessors(node)` | Nodes that point TO this node. Raises `NodeNotFoundError` |
| `successors(node)` | Nodes this node points TO. Raises `NodeNotFoundError` |

### Properties

| Method | Description |
|--------|-------------|
| `size` | Number of nodes in the graph |
| `inspect` | Human-readable string representation |
| `to_s` | String representation |

### Algorithms

| Method | Description |
|--------|-------------|
| `topological_sort` | Kahn's algorithm. Raises `CycleError` if cyclic |
| `has_cycle?` | DFS-based cycle detection |
| `transitive_closure(node)` | All nodes reachable from `node` (sorted array). Raises `NodeNotFoundError` |
| `affected_nodes(changed)` | All nodes transitively affected by changes (sorted array) |
| `independent_groups` | Parallel execution levels. Raises `CycleError` if cyclic |

### Exception Classes

| Exception | When raised |
|-----------|-------------|
| `CodingAdventures::DirectedGraphNative::CycleError` | Graph contains a cycle |
| `CodingAdventures::DirectedGraphNative::NodeNotFoundError` | Node does not exist |
| `CodingAdventures::DirectedGraphNative::EdgeNotFoundError` | Edge does not exist |

## Running Tests

```bash
bundle install
bundle exec rake test    # compiles and runs tests
```

## Why Native?

For small graphs (< 100 nodes), the pure Ruby implementation is perfectly fine. The native extension shines on large graphs (thousands of nodes) where Rust's cache-friendly data structures and zero-GC-pressure memory model provide significant speedups.
