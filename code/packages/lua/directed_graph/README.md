# directed-graph

A directed graph library for Lua 5.4 with algorithms for topological sorting, cycle detection, transitive closure, independent group computation, and affected-node analysis.

This is a port of the Go `directed-graph` package from the coding-adventures monorepo. It provides both unlabeled (`DirectedGraph`) and labeled (`LabeledGraph`) graph variants, plus visualization in DOT, Mermaid, and ASCII table formats.

## Where it fits

This package is the publishable library version of the directed graph. The build tool at `code/programs/lua/build-tool/` has its own embedded implementation; this library is the complete, standalone version with full error handling, self-loop support, labeled edges, and visualization.

## Installation

```bash
luarocks install coding-adventures-directed-graph
```

Or add to your project's dependencies:

```lua
dependencies = {
    "coding-adventures-directed-graph >= 0.1.0",
}
```

## Usage

### Basic directed graph

```lua
local dg = require("coding_adventures.directed_graph")
local DirectedGraph = dg.DirectedGraph

-- Create a graph representing build dependencies.
-- Edge direction: FROM dependency TO dependent.
local g = DirectedGraph.new()
g:add_edge("logic-gates", "arithmetic")
g:add_edge("arithmetic", "cpu-simulator")
g:add_edge("cpu-simulator", "arm-simulator")
g:add_edge("cpu-simulator", "riscv-simulator")

-- Topological sort: valid build order
local order = g:topological_sort()
-- => {"arithmetic", "arm-simulator", "cpu-simulator", ...}

-- Independent groups: what can run in parallel?
local groups = g:independent_groups()
-- Level 0: {"logic-gates"}
-- Level 1: {"arithmetic"}
-- Level 2: {"cpu-simulator"}
-- Level 3: {"arm-simulator", "riscv-simulator"}  -- parallel!

-- What needs rebuilding if logic-gates changes?
local affected = g:affected_nodes_list({["logic-gates"] = true})
-- => {"arithmetic", "arm-simulator", "cpu-simulator",
--     "logic-gates", "riscv-simulator"}
```

### Labeled graph

```lua
local LabeledGraph = dg.LabeledGraph

local lg = LabeledGraph.new()
lg:add_edge("logic-gates", "arithmetic", "compile")
lg:add_edge("logic-gates", "test-harness", "test")
lg:add_edge("arithmetic", "cpu-simulator", "compile")

-- Filter by label
local compile_deps = lg:successors_with_label("logic-gates", "compile")
-- => {"arithmetic"}

local test_deps = lg:successors_with_label("logic-gates", "test")
-- => {"test-harness"}

-- Access underlying graph for algorithms
local groups = lg:graph():independent_groups()
```

### Self-loops

```lua
-- For state machines, retry loops, etc.
local g = DirectedGraph.new_allow_self_loops()
g:add_edge("locked", "locked")    -- push when locked stays locked
g:add_edge("locked", "unlocked")  -- coin unlocks

local lg = LabeledGraph.new_allow_self_loops()
lg:add_edge("retry", "retry", "fail")
lg:add_edge("retry", "done", "success")
```

### Visualization

```lua
local viz = dg.visualization

-- Graphviz DOT format
print(viz.to_dot(g))
print(viz.labeled_to_dot(lg, {name = "FSM", initial = "locked"}))

-- Mermaid (renders in GitHub Markdown)
print(viz.to_mermaid(g, "TD"))
print(viz.labeled_to_mermaid(lg))

-- ASCII table
print(viz.to_ascii_table(g))
print(viz.labeled_to_ascii_table(lg))
```

### Error handling

All operations that can fail return `nil, error_table` on failure:

```lua
local ok, err = g:remove_node("nonexistent")
if not ok then
    if err.type == "node_not_found" then
        print("Node " .. err.node .. " does not exist")
    end
end

local order, err = g:topological_sort()
if not order then
    if err.type == "cycle" then
        print("Cannot sort: " .. err.message)
    end
end
```

Error types: `cycle`, `node_not_found`, `edge_not_found`, `label_not_found`.

## API Reference

### DirectedGraph

| Method | Description |
|--------|-------------|
| `DirectedGraph.new()` | Create graph (no self-loops) |
| `DirectedGraph.new_allow_self_loops()` | Create graph (self-loops OK) |
| `g:add_node(node)` | Add a node (idempotent) |
| `g:remove_node(node)` | Remove node and incident edges |
| `g:has_node(node)` | Check if node exists |
| `g:nodes()` | All nodes (sorted) |
| `g:size()` | Node count |
| `g:add_edge(from, to)` | Add directed edge |
| `g:remove_edge(from, to)` | Remove edge |
| `g:has_edge(from, to)` | Check if edge exists |
| `g:edges()` | All edges as {from, to} pairs (sorted) |
| `g:predecessors(node)` | Direct parents (sorted) |
| `g:successors(node)` | Direct children (sorted) |
| `g:topological_sort()` | Kahn's algorithm |
| `g:has_cycle()` | DFS three-color detection |
| `g:transitive_closure(node)` | All reachable nodes (BFS) |
| `g:transitive_dependents(node)` | Alias for transitive_closure |
| `g:independent_groups()` | Parallel execution levels |
| `g:affected_nodes(changed)` | Change propagation (set in, set out) |
| `g:affected_nodes_list(changed)` | Change propagation (sorted list) |

### LabeledGraph

Same as DirectedGraph plus label-aware operations:

| Method | Description |
|--------|-------------|
| `lg:add_edge(from, to, label)` | Add labeled edge |
| `lg:remove_edge(from, to, label)` | Remove specific label |
| `lg:has_edge_with_label(from, to, label)` | Check specific label |
| `lg:labels(from, to)` | Get all labels on edge (copy) |
| `lg:successors_with_label(node, label)` | Filter successors by label |
| `lg:predecessors_with_label(node, label)` | Filter predecessors by label |
| `lg:graph()` | Access underlying DirectedGraph |

## Development

```bash
# Run tests (requires busted)
cd tests && busted . --verbose --pattern=test_

# Or use the build system
bash BUILD
```

## Architecture

The package uses three modules:

- `init.lua` — DirectedGraph class, error types, module exports
- `labeled_graph.lua` — LabeledGraph (composition over DirectedGraph)
- `visualization.lua` — DOT, Mermaid, ASCII table formatters

LabeledGraph wraps a DirectedGraph and adds a label map. All structural algorithms (topological sort, cycle detection, etc.) delegate to the underlying DirectedGraph. Labels only affect edge-level queries.
