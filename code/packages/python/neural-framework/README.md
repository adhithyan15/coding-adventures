# Neural Framework — Python

NN40's shared checkpoint corpus is executed directly by the native package
tests: 3 accepted round-trip controls and 46 rejected malformed documents.
It supplements, rather than replaces, numerical and checkpoint-resume tests.

A native Python framework for configurable dense neural networks. It composes
the existing `matrix` package into forward propagation, analytical gradients,
full-batch SGD, and portable JSON checkpoints. No external ML framework or
native binding performs the training.

```python
import json
from neural_framework import DenseNetwork

inputs = [[0, 0], [0, 1], [1, 0], [1, 1]]
targets = [[0], [1], [1], [0]]
model = DenseNetwork.seeded([2, 4, 1], ["tanh", "sigmoid"], seed=7)
losses = model.fit(inputs, targets, epochs=3000, learning_rate=1)
print(losses[0], losses[-1], model.predict(inputs))
trace = model.gradients(inputs, targets)  # loss, predictions, every dW and db
text = json.dumps(model.to_checkpoint(), allow_nan=False)
restored = DenseNetwork.from_checkpoint(json.loads(text))
restored.train_step(inputs, targets, learning_rate=1)
assert restored.step == 3001
```

Widths can vary at every layer; activations are `linear`, `sigmoid`, `tanh`, and
`relu`. You can also construct `DenseNetwork([DenseLayer(weights, biases,
activation), ...])` for explicit parameters. Weights use `[input][output]`
order. MSE averages over samples **and** output coordinates. `fit` performs
exactly the requested updates; its first loss is before training and its last
loss corresponds to the returned model. `train_step` returns a pre-update trace.

Inputs, traces, and checkpoints do not alias model state. Non-finite data and
overflowing updates fail before changing parameters. Checkpoints are validated
data objects, not pickle or executable classes. Applications own JSON text
parsing and storage. The V1 format stores dense parameters and the SGD step
counter; future momentum/Adam need their own versioned state contract.

Install local dependencies and run the example from this directory:

```sh
python -m pip install --no-deps -e ../matrix -e .
python examples/xor.py
```

`BUILD` and `BUILD_windows` run Ruff, tests with a 95% coverage floor, and the
example. Numerical tests use the shared [NN37 fixture](../../../specs/fixtures/neural-framework-v1/dense.json)
and finite differences for every coordinate in a deep multi-output network.
For checkpoint exchange, install the TypeScript peer's dependencies, then run
`python code/scripts/validate_neural_framework_interchange.py` at repository root.

This release implements dense MLP training on native CPU matrices. CNNs,
recurrent networks, transformers, tensor autograd, GPU training, and streaming
datasets remain in the [framework roadmap](../../../learning/machine-learning/FRAMEWORK-ROADMAP.md).
