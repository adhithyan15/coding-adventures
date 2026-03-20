# Algorithms

This section covers the algorithmic ideas that power the repository's infrastructure and data structures.

The first goal here is not to create an encyclopedia of algorithms. It is to explain the algorithms that the repository actually uses in a way that connects directly back to the code.

## Topics

- [Kahn's algorithm](./kahns-algorithm.md)

## Why This Track Exists

Some packages in this repository are obviously "about algorithms," but others hide the algorithmic part under an application.

For example:

- the `directed-graph` package is about graph algorithms
- the build tool uses those graph algorithms to compute valid build order
- incremental rebuild logic depends on graph reachability and dependency propagation

Without an algorithms track, those pieces are easy to use but harder to truly understand.

## Current Coverage

| Concept | Where it appears in the repo |
|---------|------------------------------|
| Topological sorting | `directed-graph`, build tools |
| Dependency levels / parallel groups | `directed-graph`, build tools |
| Affected-node propagation | `directed-graph`, incremental build planning |
| Cycle detection | `directed-graph`, dependency validation |

The Kahn's algorithm deep dive is the starting point because it explains the most important graph operation used by the build system: computing an order where every dependency comes before the thing that depends on it.
