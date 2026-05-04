# @coding-adventures/neural-network

Generic neural network primitives built on top of
`@coding-adventures/multi-directed-graph`.

This package owns the neural-network authoring layer. The graph package remains
domain-neutral; the VM package consumes neural-network metadata and lowers it to
bytecode.

```typescript
import { createNeuralNetwork } from "@coding-adventures/neural-network";

const network = createNeuralNetwork("tiny-model")
  .input("x0")
  .input("x1")
  .constant("bias", 1)
  .weightedSum("sum", [
    { from: "x0", weight: 0.25, edgeId: "w0" },
    { from: "x1", weight: 0.75, edgeId: "w1" },
    { from: "bias", weight: -1, edgeId: "bias_to_sum" },
  ])
  .activation("relu", "sum", "relu")
  .output("out", "relu", "prediction");

network.graph.nodes(); // ["x0", "x1", "sum", "relu", "out"]
```

Supported v0 primitives:

| Primitive | Metadata authored |
| --- | --- |
| `input` | `nn.op=input`, `nn.input=<name>` |
| `constant` | `nn.op=constant`, `nn.value=<number>` |
| `weightedSum` | `nn.op=weighted_sum` plus weighted incoming edges |
| `activation` | `nn.op=activation`, `nn.activation=relu|sigmoid|tanh|none` |
| `output` | `nn.op=output`, `nn.output=<name>` |

`createXorNetwork()` provides the first hidden-layer teaching graph: two inputs,
a bias constant, OR/NAND hidden activations, and a final sigmoid output.

Future packages can add higher-level layers such as dense, convolution,
normalization, dropout, and optimizers while still lowering to the same graph
metadata contract.
