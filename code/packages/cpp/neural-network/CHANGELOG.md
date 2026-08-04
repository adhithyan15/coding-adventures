# Changelog

All notable changes to the C++ `neural-network` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `neural-network` crate
  (namespace `ca::neural_network`) — a property-graph IR for neural-network
  topologies.
- `PropertyValue` (`std::variant`), `PropertyBag` (`std::unordered_map`), the
  `ActivationKind` enum with `as_str`, `Edge`, and `WeightedInput`.
- `NeuralGraph` with `add_node` / `add_edge` (auto-endpoints + `"e<n>"` id
  minting + merged `"weight"`), `incoming_edges`, and `topological_sort`
  (Kahn's algorithm, deterministic lexicographic tie-breaking, `std::nullopt`
  on a cycle).
- Free-function layer builders, the fluent `NeuralNetwork` builder, and
  `create_xor_network`.
- `add_constant` throws `std::invalid_argument` on a non-finite value (finiteness
  checked without `<cmath>`); `topological_sort` returns
  `std::optional<std::vector<std::string>>` in place of the Rust `Result`.
- 20 checks mirroring the Rust crate's tests plus edge-id minting and cycle
  detection, run under every available C++ compiler via the shared `iso-harness`.
