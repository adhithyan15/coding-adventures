# graph (Go)

An undirected graph data structure library. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

## What is an undirected graph?

An undirected graph is a set of nodes connected by edges, where each edge has no direction — it connects two nodes symmetrically. Think of it like a two-way street map: if you can travel from A to B, you can also travel from B to A.

In an undirected graph, edges are mutual relationships. For example:
- In a social network, friendships are mutual (if Alice is friends with Bob, Bob is friends with Alice)
- In a road network, roads go both ways
- In a peer-to-peer network, connections are symmetric

## API

```go
g := graph.New()

// Build the graph
g.AddNode("Alice")
g.AddEdge("Alice", "Bob")  // Alice and Bob are connected

// Query
g.HasNode("Alice")         // true
g.Neighbors("Alice")       // ["Bob"]
g.Degree("Alice")          // 1

// Graph structure
g.Nodes()                  // ["Alice", "Bob"]
g.Edges()                  // [["Alice", "Bob"]]
```

## Key methods

| Method | What it does | Use case |
|--------|-------------|----------|
| `AddNode(node)` | Add a node | Build social network |
| `AddEdge(from, to)` | Add an undirected edge | Connect two people/places |
| `HasEdge(from, to)` | Check if edge exists | Query relationship |
| `Neighbors(node)` | All adjacent nodes | Find direct connections |
| `Degree(node)` | Number of neighbors | Find most connected node |
| `RemoveNode(node)` | Remove node and edges | Prune disconnected components |
| `RemoveEdge(from, to)` | Remove edge | Break relationship |

## Usage

```go
package main

import (
	"fmt"
	"github.com/adhithyan15/coding-adventures/code/packages/go/graph"
)

func main() {
	g := graph.New()

	// Build a simple social network
	g.AddEdge("Alice", "Bob")
	g.AddEdge("Bob", "Charlie")
	g.AddEdge("Alice", "Charlie")

	// Query the graph
	neighbors, _ := g.Neighbors("Bob")
	fmt.Println("Bob's friends:", neighbors)  // [Alice Charlie]

	degree, _ := g.Degree("Bob")
	fmt.Println("Bob's degree:", degree)      // 2
}
```

## How it fits in the stack

The undirected graph is a foundational data structure alongside the directed-graph package. While directed-graph is used for build system dependency resolution, the undirected graph is useful for:
- Social networks and connection analysis
- Game map pathfinding
- Network topology analysis
- Peer-to-peer network modeling

## Error handling

Operations that might fail return `(value, error)`. Error cases:
- `NodeNotFound` -- referenced node doesn't exist
- `EdgeNotFound` -- referenced edge doesn't exist
- `SelfLoop` -- self-loops are not allowed
