# coding_adventures_graph

DT00 graph for Ruby.

This gem provides an undirected weighted graph with adjacency-list and
adjacency-matrix backends plus BFS, DFS, connectivity checks, shortest path,
connected components, cycle detection, and minimum spanning tree.

## Usage

```ruby
require "coding_adventures_graph"

graph = CodingAdventures::Graph::Graph.new
graph.add_node("London", { "kind" => "city" })
graph.add_edge("London", "Paris", 300.0, { "route" => "train" })
graph.add_edge("London", "Amsterdam", 520.0)
graph.add_edge("Amsterdam", "Berlin", 655.0)

graph.edge_properties("Paris", "London")
# => { "route" => "train", "weight" => 300.0 }

CodingAdventures::Graph.bfs(graph, "London")
CodingAdventures::Graph.shortest_path(graph, "London", "Berlin")
```

Graph, node, and edge property bags are copied on read. The canonical edge
property `"weight"` is kept in sync with `edge_weight` and weighted algorithms.

## Development

```bash
bundle install
bundle exec rake test
```
