---
category: Python
---

# An MST consumer fixture must connect every node before asserting the tree result

The wheel consumer added an isolated node, then asserted that
`minimum_spanning_tree` returned a tree. Kruskal correctly rejects a
disconnected graph, so the consumer check failed before it reached the
shortest-path assertion. Keep the MST example connected; test unreachable
queries in a separate graph. A consumer smoke test should reach the API
behavior it intends to verify before checking it.
