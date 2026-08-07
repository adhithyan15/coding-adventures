# neural-network (C)

A **property-graph representation of neural-network topologies**, in pure ISO
C17 — a faithful port of the Rust `neural-network` crate. This is *not* a
trainable network; it's the graph IR a compiler builds to describe one.

## The model

- **`NgProperty`** — a tagged value: String / Number / Boolean / Null.
- **`NgPropertyBag`** — a `string → NgProperty` map, attached to the graph, each
  node, and each edge.
- **`NgEdge`** `{ id, from, to, weight, properties }`.
- **`NeuralGraph`** — nodes (each with a bag), directed weighted edges, and an
  edge-id counter.

Layer builders (`ng_add_input` / `_constant` / `_weighted_sum` / `_activation` /
`_output`) set the `nn.op` / `nn.*` properties and wire the edges;
`ng_create_xor_network` assembles the classic XOR topology. Two graph
operations:

- **`ng_add_edge`** auto-creates its endpoints and mints an id (`"e0"`, `"e1"`,
  …) when none is given, merging a `"weight"` property;
- **`ng_topological_sort`** — Kahn's algorithm with deterministic
  (lexicographic) tie-breaking, reporting a cycle.

## API sketch

```c
#include "neural_network.h"

NeuralGraph *g;
ng_new(&g, "tiny");
ng_add_input(g, "x0", "x0", NULL);
ng_add_constant(g, "bias", 1.0, NULL);

NgWeightedInput in[2];
ng_weighted_input_init(&in[0], "x0", 0.25, "x0_to_sum");
ng_weighted_input_init(&in[1], "bias", -1.0, "bias_to_sum");
ng_add_weighted_sum(g, "sum", in, 2, NULL);
ng_weighted_input_free(&in[0]); ng_weighted_input_free(&in[1]);

char **order; size_t n;
if (ng_topological_sort(g, &order, &n) == NG_OK) {
    /* order[n-1] is a sink node */
    ng_string_array_free(order, n);
}
ng_free(g);
```

## Divergence from the Rust crate

Rust panics on a non-finite constant and returns owned values / `Result`; this
port returns an `NgStatus` (`NG_OK` / `NG_ERR_NOMEM` / `NG_ERR_NOT_FINITE` /
`NG_ERR_CYCLE`) and writes results through out-parameters. Finiteness is checked
without `<math.h>` (`x - x == 0`).

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17, no `<math.h>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness); the test suite also runs clean under
AddressSanitizer + UndefinedBehaviorSanitizer.
