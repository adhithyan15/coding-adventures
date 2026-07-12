# Changelog

All notable changes to the C `neural-network` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `neural-network` crate — a
  property-graph IR for neural-network topologies.
- `NgProperty` tagged value (String/Number/Boolean/Null) and `NgPropertyBag`
  map; `NgEdge` / `NgWeightedInput`; the `NgActivation` enum with
  `ng_activation_str`.
- `NeuralGraph` with `ng_add_node` / `ng_add_edge` (auto-endpoints + `"e<n>"`
  id minting + merged `"weight"`), `ng_incoming_edges`, and
  `ng_topological_sort` (Kahn's algorithm, deterministic lexicographic
  tie-breaking, cycle detection).
- Layer builders (`ng_add_input` / `_constant` / `_weighted_sum` /
  `_activation` / `_output`) and `ng_create_xor_network`.
- `NgStatus` status-code API (`NG_OK` / `NG_ERR_NOMEM` / `NG_ERR_NOT_FINITE` /
  `NG_ERR_CYCLE`) in place of the Rust panic / `Result`; finiteness checked
  without `<math.h>`; every owning value pairs a constructor with a `*_free`
  and growable arrays guard reallocation against `size_t` overflow.
- 61 checks mirroring the Rust crate's tests (tiny-graph incoming/topo, XOR
  topology) plus edge-id minting and cycle detection, run under every available
  C compiler via the shared `iso-harness`; the suite also passes under ASan +
  UBSan.
