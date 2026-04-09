# graph (Elixir)

An undirected graph data structure library. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational project.

## What is an undirected graph?

An undirected graph is a set of nodes connected by edges, where each edge has no direction — it connects two nodes symmetrically. Think of it like a two-way street map: if you can travel from A to B, you can also travel from B to A.

In an undirected graph, edges are mutual relationships. For example:
- In a social network, friendships are mutual (if Alice is friends with Bob, Bob is friends with Alice)
- In a road network, roads go both ways
- In a peer-to-peer network, connections are symmetric

## API

```elixir
alias CodingAdventures.Graph

g = Graph.new()

# Build the graph
{:ok, g} = Graph.add_edge(g, "Alice", "Bob")
{:ok, g} = Graph.add_edge(g, "Bob", "Charlie")

# Query
Graph.has_node?(g, "Alice")         # true
{:ok, neighbors} = Graph.neighbors(g, "Alice")
# neighbors = ["Bob"]

{:ok, degree} = Graph.degree(g, "Alice")
# degree = 1

# Graph structure
Graph.nodes(g)                      # ["Alice", "Bob", "Charlie"]
Graph.edges(g)                      # [{"Alice", "Bob"}, {"Bob", "Charlie"}]
```

## Key functions

| Function | What it does | Use case |
|----------|-------------|----------|
| `new()` | Create empty graph | Start fresh |
| `add_node(graph, node)` | Add a node | Create vertex |
| `add_edge(graph, from, to)` | Add an undirected edge | Connect two nodes |
| `has_edge?(graph, from, to)` | Check if edge exists | Query relationship |
| `neighbors(graph, node)` | All adjacent nodes | Find direct connections |
| `degree(graph, node)` | Number of neighbors | Find connectivity |
| `remove_node(graph, node)` | Remove node and edges | Remove vertex |
| `remove_edge(graph, from, to)` | Remove edge | Break relationship |

## Immutability

Like all Elixir data structures, graphs are immutable. Every operation returns a new graph (or error tuple). This makes graphs safe for use in concurrent code without locks.

## Usage

```elixir
alias CodingAdventures.Graph

# Build a social network
g = Graph.new()
{:ok, g} = Graph.add_edge(g, "Alice", "Bob")
{:ok, g} = Graph.add_edge(g, "Bob", "Charlie")
{:ok, g} = Graph.add_edge(g, "Alice", "Charlie")

# Query the graph
{:ok, bob_friends} = Graph.neighbors(g, "Bob")
IO.inspect(bob_friends)  # ["Alice", "Charlie"]

{:ok, bob_degree} = Graph.degree(g, "Bob")
IO.puts(bob_degree)      # 2
```

## How it fits in the stack

The undirected graph is a foundational data structure alongside the directed-graph package. While directed-graph is used for build system dependency resolution, the undirected graph is useful for:
- Social networks and connection analysis
- Game map pathfinding
- Network topology analysis
- Peer-to-peer network modeling

## Error handling

Functions that might fail return `{:ok, value}` or `{:error, reason}` tuples. Error cases:
- `NodeNotFound` -- referenced node doesn't exist
- `EdgeNotFound` -- referenced edge doesn't exist
- `SelfLoop` -- self-loops are not allowed
