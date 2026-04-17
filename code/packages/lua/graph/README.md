# lua/graph - Graph (DT00)

An undirected weighted graph implementation in pure Lua with both adjacency-list
and adjacency-matrix storage, plus the core DT00 algorithms.

## API

```lua
local graph = require("coding_adventures.graph")
local Graph = graph.Graph
local GraphRepr = graph.GraphRepr

local g = Graph.new({ repr = GraphRepr.ADJACENCY_LIST })
g:add_edge("London", "Paris", 300)
g:add_edge("London", "Amsterdam", 520)

local path = graph.shortest_path(g, "London", "Paris")
local mst = graph.minimum_spanning_tree(g)
```

## Exports

- `Graph`
- `GraphRepr`
- `bfs(graph, start)`
- `dfs(graph, start)`
- `is_connected(graph)`
- `connected_components(graph)`
- `has_cycle(graph)`
- `shortest_path(graph, start, goal)`
- `minimum_spanning_tree(graph)`
