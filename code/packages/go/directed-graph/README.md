# Directed Graph (Go)

A directed graph data structure with algorithms for topological sorting, cycle detection, and parallel execution level computation. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo build system.

## What is a directed graph?

A directed graph (or "digraph") is a set of nodes connected by edges, where each edge has a direction — it goes FROM one node TO another. In this build system, nodes are packages and edges are dependencies: if package B depends on package A, there's an edge from A to B.

## API

```go
g := directedgraph.New()

// Build the graph
g.AddNode("logic-gates")
g.AddEdge("logic-gates", "arithmetic")  // arithmetic depends on logic-gates

// Query
g.HasNode("logic-gates")           // true
g.Predecessors("arithmetic")       // ["logic-gates"]
g.Successors("logic-gates")        // ["arithmetic"]

// Algorithms
order, _ := g.TopologicalSort()     // valid build order
groups, _ := g.IndependentGroups()  // parallel execution levels
affected := g.AffectedNodes(changed) // incremental build targets
hasCycle := g.HasCycle()            // cycle detection
```

## Key methods

| Method | What it does | Used for |
|--------|-------------|----------|
| `TopologicalSort()` | Order nodes so every dep comes first | Valid build order |
| `IndependentGroups()` | Partition into levels by topo depth | Parallel execution |
| `AffectedNodes(changed)` | Changed nodes + all transitive dependents | Incremental builds |
| `HasCycle()` | Detect circular dependencies | Validation |
| `TransitiveClosure(node)` | All nodes reachable downstream | Dependency analysis |
| `TransitiveDependents(node)` | All nodes that depend on this one | Impact analysis |

## Operations

Every method is wrapped in an `Operation` — a closure that provides:

- **Automatic timing**: every call records elapsed milliseconds
- **Structured logging**: JSON log line per call with operation name, timing, outcome, and metadata
- **Panic recovery**: unexpected panics are caught, logged, and converted to errors
  (except `AddEdge` which uses `PanicOnUnexpected()` to preserve its self-loop panic contract)
- **Typed error preservation**: `rf.Fail(value, err)` returns typed errors intact so
  `errors.As` checks work correctly at the call site

OS capabilities are accessed as namespace fields directly on `op` — `op.File.ReadFile(path)`,
`op.Net.Connect(addr)`, etc. These fields only exist when the corresponding capability is
declared in `required_capabilities.json`. Since this package declares zero OS capabilities,
`Operation[T]` has no capability fields. Any OS access must be declared in the manifest —
making it visible in code review and enforced at compile time.

## Where it fits

```
Build Tool CLI
    │
    └──→ Directed Graph Library ← you are here
         (topological sort, parallel levels, affected nodes)
```

## Testing

```bash
go test ./... -v
go test ./... -cover
```

39 tests, 95% coverage.

## Implementations

This library exists in three languages with identical APIs:

| Language | Location | Tests | Coverage |
|----------|----------|-------|---------|
| Python | `code/packages/python/directed-graph/` | 73 | 98% |
| Ruby | `code/packages/ruby/directed_graph/` | 77 | 100% |
| **Go** | `code/packages/go/directed-graph/` | 39 | 94% |
