# Neural Framework — TypeScript

NN40's shared checkpoint corpus is executed directly by the native package
tests: 3 accepted round-trip controls and 52 rejected malformed documents.
NN43 adds six numeric-null cases, including mixed arrays and later layers.
It supplements, rather than replaces, numerical and checkpoint-resume tests.

A native TypeScript framework for configurable dense neural networks above the
existing `matrix` package. Forward propagation, analytical backpropagation,
full-batch SGD, and checkpoint validation run in TypeScript using binary64
numbers. No external ML framework or Rust binding performs the training.

```typescript
import { DenseNetwork } from "@coding-adventures/neural-framework";

const inputs = [[0, 0], [0, 1], [1, 0], [1, 1]];
const targets = [[0], [1], [1], [0]];
const model = DenseNetwork.seeded([2, 4, 1], ["tanh", "sigmoid"], 7);
const losses = model.fit(inputs, targets, 3000, 1);
console.log(losses[0], losses.at(-1), model.predict(inputs));
const trace = model.gradients(inputs, targets); // loss, predictions, every dW/db
const restored = DenseNetwork.fromCheckpoint(JSON.parse(JSON.stringify(model.toCheckpoint())));
restored.trainStep(inputs, targets, 1);
console.assert(restored.step === 3001);
```

Widths may differ at every layer; activations are `linear`, `sigmoid`, `tanh`,
and `relu`. Explicit parameters can be passed as
`new DenseNetwork([{ weights, biases, activation }, ...])`. Weights are indexed
`[input][output]`. MSE averages over samples and outputs. `fit` performs exactly
the requested updates and returns the initial loss followed by each updated
loss. `trainStep` returns a pre-update trace.

Models own copies of all parameter arrays. Inspection and checkpoint results
are independent snapshots. Invalid shapes, non-finite numbers, and overflowing
updates fail before changing model state. The V1 checkpoint transfers directly
to Python and includes the SGD step counter. Applications handle JSON text and
file I/O. Loading a checkpoint never resolves or executes named classes.

From this directory:

```sh
npm ci --ignore-scripts
npm run build
npm run test:coverage
npm run example
```

The build files install the local matrix dependency closure first. Only the
native `Matrix` arithmetic methods are used; the optional addon is not loaded
or required to train. The public API also uses no Node-specific imports, so it
can be bundled for a browser; the command-line example uses Node for JSON I/O.
The package uses the repository's source-entry distribution convention.

Tests check the shared [NN37 fixture](../../../specs/fixtures/neural-framework-v1/dense.json),
every coordinate by finite differences, seeded XOR training, checkpoint resume,
and invalid inputs. The coverage floor is 95% statements/lines/functions and
85% branches. Run `python code/scripts/validate_neural_framework_interchange.py`
from repository root (with Cargo, Go and Ruby installed) and pass
`--c-executable PATH` for a C peer binary built by its ISO harness. NN49 executes
six native lanes and thirty directed checkpoint exchanges through step 3002;
selected exact-head CI/merge remains required before delivery.

This release supplies dense MLP training. Other architectures, graph-wide
autograd, batching, optimizers, and acceleration remain in the
[framework roadmap](../../../learning/machine-learning/FRAMEWORK-ROADMAP.md).
