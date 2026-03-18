# coding_adventures_directed_graph

A directed graph library with topological sort, cycle detection, transitive closure, and parallel execution levels. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) computing stack.

## What is a directed graph?

A directed graph (digraph) is a set of nodes connected by one-way edges. If there is an edge from A to B, it means "A depends on B" or "A must come before B." This data structure is the backbone of build systems, package managers, and task schedulers.

## Where it fits in the stack

This gem is a foundational utility used by higher-level packages in the computing stack. For example, the build pipeline uses it to determine which packages can be compiled in parallel and which must wait for their dependencies.

## Installation

```ruby
gem "coding_adventures_directed_graph"
```

## Usage

```ruby
require "coding_adventures_directed_graph"

g = CodingAdventures::DirectedGraph::Graph.new

# Build a dependency graph
g.add_edge("logic_gates", "arithmetic")
g.add_edge("arithmetic", "cpu")
g.add_edge("cpu", "assembler")
g.add_edge("assembler", "vm")

# Get a valid build order
g.topological_sort
# => ["logic_gates", "arithmetic", "cpu", "assembler", "vm"]

# Which packages can be built in parallel?
g.independent_groups
# => [["logic_gates"], ["arithmetic"], ["cpu"], ["assembler"], ["vm"]]

# If I change arithmetic, what else needs rebuilding?
g.affected_nodes(["arithmetic"])
# => ["arithmetic", "assembler", "cpu", "vm"]

# Check for cycles
g.has_cycle?  # => false

# Transitive closure: who can reach whom?
closure = g.transitive_closure
closure["logic_gates"]
# => Set["arithmetic", "cpu", "assembler", "vm"]
```

## API

### Core methods

| Method | Description |
|---|---|
| `add_node(node)` | Add a node (idempotent, chainable) |
| `add_edge(source, target)` | Add a directed edge (raises `CycleError` on self-loop) |
| `remove_node(node)` | Remove a node and all its edges |
| `remove_edge(source, target)` | Remove a single edge |
| `has_node?(node)` | Check if a node exists |
| `has_edge?(source, target)` | Check if an edge exists |
| `nodes` | Sorted array of all nodes |
| `edges` | Sorted array of `[source, target]` pairs |
| `predecessors(node)` | Sorted array of nodes with edges pointing to this node |
| `successors(node)` | Sorted array of nodes this node points to |
| `size` | Number of nodes |

### Algorithm methods

| Method | Description |
|---|---|
| `topological_sort` | Kahn's algorithm; raises `CycleError` if graph has a cycle |
| `has_cycle?` | Returns true if the graph contains a cycle |
| `transitive_closure` | Hash mapping each node to a Set of all reachable nodes |
| `transitive_dependents(node)` | Sorted array of all nodes reachable from the given node |
| `independent_groups` | Array of arrays; each inner array is a parallelizable layer |
| `affected_nodes(changed)` | All nodes affected by changes to the given set of nodes |

### Error classes

- `CodingAdventures::DirectedGraph::CycleError`
- `CodingAdventures::DirectedGraph::NodeNotFoundError`
- `CodingAdventures::DirectedGraph::EdgeNotFoundError`

## Development

```bash
bundle install
bundle exec rake test
```

## License

MIT
