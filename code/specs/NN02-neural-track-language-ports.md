# NN02: Neural Network Track Language Ports

Status: draft

## Purpose

NN02 defines the common cross-language surface for the neural-network track. The
goal is that a learner can build the same small neural graph in any supported
language, lower it to the same bytecode shape, and compare outputs against the
same conformance examples.

This spec covers the graph-native track introduced by NN00:

- `neural-network`: authoring primitives for executable neural graphs.
- `neural-graph-vm`: scalar bytecode compiler and reference forward runner.

Existing math packages such as `activation-functions`, `loss-functions`,
`gradient-descent`, `feature-normalization`, `single-layer-network`, and
`two-layer-network` remain standalone Unix-like packages. They are not replaced
by this track. The graph-native packages are the shared model authoring and VM
front door that later backends can target.

## Required `neural-network` Surface

Each language port must expose:

- A neural graph value with graph, node, and edge property bags.
- Stable node IDs and stable edge IDs.
- Directed edges with numeric weights.
- Parallel edges when the language graph backend supports them. If the local
  graph backend does not yet have a multi-directed graph package, the
  neural-network package may keep its own edge list until that package is
  ported.
- Deterministic `nodes`, `edges`, `incoming_edges`, and `topological_sort`.
- Primitive helpers:
  - `create_neural_graph(name?)`
  - `create_neural_network(name?)`
  - `add_input(graph, node, input_name = node)`
  - `add_constant(graph, node, value)`
  - `add_weighted_sum(graph, node, inputs)`
  - `add_activation(graph, node, input, activation)`
  - `add_output(graph, node, input, output_name = node)`
  - `create_xor_network(name = "xor")`

Required activation names for graph metadata:

- `relu`
- `sigmoid`
- `tanh`
- `none`

## Required `neural-graph-vm` Surface

Each language port must expose:

- Bytecode module metadata:
  - magic: `CANN`
  - version: `0`
  - graph nodes
  - graph edges with `id`, `from`, `to`, and `weight`
  - one forward function
- Bytecode opcodes:
  - `LOAD_INPUT`
  - `LOAD_CONST`
  - `LOAD_EDGE_WEIGHT`
  - `MUL`
  - `ADD`
  - `ACTIVATE`
  - `STORE_OUTPUT`
- Compiler helpers:
  - `compile_neural_graph_to_bytecode(graph)`
  - `compile_neural_network_to_bytecode(network)`
- Runtime helpers:
  - `run_neural_bytecode_forward(module, inputs)`

The scalar VM is the correctness oracle. Matrix, GPU, NPU, browser, and native
accelerated backends should compare against this runner.

## Conformance Examples

Every language port should include tests for:

1. Tiny weighted sum:

```text
x0 -> sum weight 0.25
x1 -> sum weight 0.75
bias(1) -> sum weight -1
sum -> relu -> prediction

inputs: x0 = 4, x1 = 8
output: prediction = 6
```

2. XOR network:

```text
inputs: (0,0), (0,1), (1,0), (1,1)
outputs: approximately 0, 1, 1, 0
```

The XOR helper is intentionally explicit: bias node, hidden OR-like activation,
hidden NAND-like activation, and output activation. It exists to teach why a
hidden layer can represent a non-linear relationship.

## Port Matrix

| Language | Existing ML math packages | `neural-network` | `neural-graph-vm` |
| --- | --- | --- | --- |
| TypeScript | yes | done | done |
| Python | yes | done | done |
| Go | yes | done | done |
| Rust | yes | done | done |
| C# | yes | done | done |
| F# | yes | done | done |
| Java | partial | done | done |
| Kotlin | partial | done | done |
| Swift | partial | done | done |
| Ruby | yes | done | done |
| Perl | yes | done | done |
| Dart | partial | done | done |
| Haskell | partial | done | done |
| Elixir | partial | done | done |
| Lua | partial | done | done |
| WASM/Rust crates | graph only | defer | defer |
| Starlark | no package track yet | defer | defer |

## Implementation Order

1. Keep TypeScript as the browser and visualizer-facing reference.
2. Keep the scalar VM tests aligned across every language port.
3. Add shared fixture files once serialized bytecode output needs to be compared
   byte-for-byte across runners.
4. Defer Starlark and WASM-specific wrapper packages until those package tracks
   have a clear neural-network consumer.
