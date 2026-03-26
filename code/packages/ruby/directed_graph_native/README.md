# coding_adventures_directed_graph_native

A Rust-backed native extension for Ruby that provides a directed graph with
topological sort, cycle detection, transitive closure, and parallel execution
level computation.

## How it fits in the stack

This gem is the **native counterpart** to `coding_adventures_directed_graph`
(the pure Ruby implementation). It uses the same `directed-graph` Rust crate
that powers the Python native extension, connected to Ruby via our
`ruby-bridge` crate instead of Magnus or rb-sys.

```
Ruby code
  |
  v
ruby-bridge (Rust, extern "C" to libruby)
  |
  v
directed-graph (pure Rust algorithms)
```

## Building

Prerequisites: Rust toolchain (rustup), Ruby 3.4+.

```bash
cd ext/directed_graph_native
ruby extconf.rb
make
```

Or via Rake:

```bash
rake compile
```

## Usage

```ruby
require "coding_adventures_directed_graph_native"

g = CodingAdventures::DirectedGraphNative::Graph.new

g.add_edge("compile", "link")
g.add_edge("link", "package")
g.add_edge("test", "package")

g.topological_sort    # => ["compile", "link", "test", "package"]
g.independent_groups  # => [["compile", "test"], ["link"], ["package"]]
g.has_cycle?          # => false

g.affected_nodes(["compile"])  # => ["compile", "link", "package"]
g.transitive_closure("compile")  # => ["link", "package"]
```

## API

All methods match the pure Ruby `CodingAdventures::DirectedGraph::Graph` API:

| Method | Args | Returns | Description |
|--------|------|---------|-------------|
| `add_node(name)` | String | nil | Add a node |
| `remove_node(name)` | String | nil | Remove node and its edges |
| `has_node?(name)` | String | Boolean | Check if node exists |
| `nodes` | none | Array | Sorted node names |
| `size` | none | Integer | Number of nodes |
| `add_edge(from, to)` | String, String | nil | Add directed edge |
| `remove_edge(from, to)` | String, String | nil | Remove an edge |
| `has_edge?(from, to)` | String, String | Boolean | Check edge existence |
| `edges` | none | Array | Sorted [from, to] pairs |
| `predecessors(node)` | String | Array | Sorted predecessor names |
| `successors(node)` | String | Array | Sorted successor names |
| `topological_sort` | none | Array | Kahn's algorithm result |
| `has_cycle?` | none | Boolean | Cycle detection |
| `transitive_closure(node)` | String | Array | All reachable nodes |
| `affected_nodes(changed)` | Array | Array | Changed + transitive dependents |
| `independent_groups` | none | Array | Parallel execution levels |

## Testing

```bash
rake test
```

## License

MIT
