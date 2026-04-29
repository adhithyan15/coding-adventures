# @coding-adventures/neural-graph-vm

Reference Neural Graph VM compiler and scalar bytecode interpreter.

This package is the first executable target for `NN00-neural-graph-vm.md`.
It is not a GPU backend. It is the portable reference layer that future CPU,
WebGPU, Rust-native GPU, compute-unit simulator, and ASIC-style backends can
lower from.

```typescript
import { createNeuralNetwork } from "@coding-adventures/neural-network";
import {
  compileNeuralNetworkToBytecode,
  runNeuralBytecodeForward,
} from "@coding-adventures/neural-graph-vm";

const network = createNeuralNetwork("tiny-model")
  .input("x0")
  .input("x1")
  .weightedSum("sum", [
    { from: "x0", weight: 0.25, edgeId: "w0" },
    { from: "x1", weight: 0.75, edgeId: "w1" },
  ])
  .output("out", "sum", "prediction");

const bytecode = compileNeuralNetworkToBytecode(network);
const outputs = runNeuralBytecodeForward(bytecode, { x0: 4, x1: 8 });
// { prediction: 7 }
```

Supported v0 graph ops:

| `nn.op` | Behavior |
| --- | --- |
| `input` | Loads a scalar runtime input. |
| `weighted_sum` | Multiplies incoming source values by edge weights and sums them. |
| `activation` | Applies `relu`, `sigmoid`, `tanh`, or `none`. |
| `output` | Stores a named scalar output. |

The interpreter is intentionally scalar and small. Its job is correctness,
debuggability, and portability before optimized matrix lowering exists.

`MultiDirectedGraph` remains generic and domain-neutral. The
`@coding-adventures/neural-network` package is the neural primitive layer on top:
helpers such as `input`, `weightedSum`, `activation`, and `output` author the
metadata that this VM compiler lowers into bytecode.
