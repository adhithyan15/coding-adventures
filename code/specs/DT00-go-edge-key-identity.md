# DT00 Go profile: edge endpoint identity

Go graph node IDs are strings with no delimiter restriction. Edge identity is
the unordered pair of endpoint strings, ordered canonically for storage. An
internal encoding must distinguish every pair of accepted endpoint strings;
concatenating endpoints with a separator is not unambiguous when endpoint
strings may contain that separator.

The same canonical pair identity is used for edge properties, adjacency-list
edge enumeration deduplication, and edge removal through either endpoint
order. Removing a node removes metadata only for incident edges. Distinct
edges retain independent properties and appear once each in `Edges()` for both
graph representations.

`ConcurrentGraph.Snapshot` preserves this identity in its detached copy. The
snapshot retains its own edge properties after mutations to the concurrent
owner. Public method signatures and ordinary endpoint-label behavior do not
change.

## Verification scope

Unit regressions use two distinct NUL-containing endpoint pairs that collide
under delimiter concatenation. They cover topology, weights, properties,
property updates and removal, edge removal, node removal, both endpoint orders,
both `GraphRepr` values, and detached snapshot behavior. An independent bounded
pair-key model and native Go fuzz target exercise add, update, property delete,
edge delete, and node delete using ordinary and NUL-containing labels. These
tests do not claim exhaustive behavior over all strings or a proof of the
implementation.
