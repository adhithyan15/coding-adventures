# neural-network (C++)

A **property-graph representation of neural-network topologies**, header-only in
pure ISO C++17 (namespace `ca::neural_network`) — a faithful port of the Rust
`neural-network` crate. Not a trainable network; the graph IR describing one.

## The model

- **`PropertyValue`** = `std::variant<std::string, double, bool,
  std::monostate>`; **`PropertyBag`** = `std::unordered_map<std::string,
  PropertyValue>`.
- **`Edge`** `{ id, from, to, weight, properties }`.
- **`NeuralGraph`** — nodes (each with a bag), directed weighted edges, an
  edge-id counter, `add_edge` (auto-endpoints + `"e<n>"` minting + merged
  `"weight"`), `incoming_edges`, and `topological_sort` (Kahn's algorithm,
  deterministic tie-breaking).

The fluent `NeuralNetwork` builder and free-function layer builders wire the
`nn.op` / `nn.*` properties; `create_xor_network` assembles the classic XOR
topology.

## Usage

```cpp
#include "neural_network.hpp"
namespace nn = ca::neural_network;

nn::NeuralNetwork net = nn::create_xor_network("xor");
net.graph.incoming_edges("out_sum").size();   // 3
auto order = net.graph.topological_sort();    // std::optional (nullopt on cycle)
```

## Divergence from the Rust crate

`add_constant` throws `std::invalid_argument` on a non-finite value (the Rust
panic); `topological_sort` returns `std::optional<std::vector<std::string>>`
(`std::nullopt` on a cycle) in place of the Rust `Result<_, String>`.

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17, no `<cmath>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).
