# Multi Directed Graph

Generic directed multigraph storage for Go packages.

The graph keeps insertion-ordered nodes, stable edge IDs, parallel directed
edges, optional self-loops, numeric edge weights, and metadata property bags on
the graph, nodes, and edges. Domain packages should layer their own semantics on
top of this storage instead of baking those semantics into the graph.
