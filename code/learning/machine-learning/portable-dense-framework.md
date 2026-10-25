# From Teaching Examples to a Trainable Framework

A teaching example chooses the network for you. A framework lets you choose
the layer widths, activations, parameters, and data, then applies the same
forward and backward rules to your model. NN37 starts that transition with
native [Python](../../packages/python/neural-framework/README.md) and
[TypeScript](../../packages/typescript/neural-framework/README.md) libraries.

## One complete calculation

Take one input row `[1, 2]` and target `[1]`. The first linear layer has weights
`[[0.5, -0.5], [1, 0.5]]` and zero biases. Its outputs are:

```text
h0 = 1 * 0.5  + 2 * 1   = 2.5
h1 = 1 * -0.5 + 2 * 0.5 = 0.5
```

A second linear layer uses weights `[[1], [-1]]` and bias `0.25`:

```text
prediction = 2.5 * 1 + 0.5 * -1 + 0.25 = 2.25
error      = 2.25 - 1                  = 1.25
MSE        = 1.25 * 1.25               = 1.5625
dLoss/dPrediction = 2 * 1.25           = 2.5
```

The output weights receive gradients `[[6.25], [1.25]]`: each hidden value
multiplies the output gradient. The output bias receives `2.5`. Going backward,
the two hidden values receive `[2.5, -2.5]`, because the output weights are
`[1, -1]`. The first weights therefore receive `[[2.5, -2.5], [5, -5]]`, and
their biases receive `[2.5, -2.5]`.

With learning rate `0.01`, subtract `0.01 * gradient` from each parameter. The
first layer now produces `[2.35, 0.65]`; the output weights are
`[0.9375, -1.0125]` and its bias is `0.225`. The new prediction is `1.77`, and
the new loss is `0.5929`. Every number is pinned in the
[shared fixture](../../specs/fixtures/neural-framework-v1/dense.json).

## Choose a different model

The same gradient engine handles widths `[2, 3, 2, 2]`, or a single output
layer, without a separate derivative implementation. It saves each layer's
inputs during the forward pass and walks backward through those saved values.
Central finite differences independently check every weight and bias, including
networks with multiple outputs and all four supported activation functions.

The runnable examples choose `[2, 4, 1]` with tanh then sigmoid and learn XOR
from seeded random weights. You can inspect predictions and every gradient
through `gradients`, train exactly one step, or run a chosen number of epochs.
History includes both the initial loss and the loss at the final model state.

## Keep training in another language

A checkpoint records each layer's activation, weights, and biases, plus the
number of completed SGD steps. It is ordinary JSON data, so Python can write
it and TypeScript, Rust or Go can load it, or vice versa. No implementation executes
code from the document. Run
`python code/scripts/validate_neural_framework_interchange.py` at repository
root after installing the TypeScript package dependencies, Cargo and Go to prove
training and all twelve directed checkpoint exchanges. The JSON receipt identifies
four native lanes and zero binding lanes, not four wrappers over one runtime.

NN42 extends NN39: first train each model to step 3000, then compute native 3001/3002
baseline. The trace for Python to Rust is `Python(3000) -> Rust(3001) ->
Python(3002)`: each arrow transports the actual owned JSON parameters on stdin.
Every parameter, prediction, loss, and exact step is compared with the baseline;
the same trace repeats for each ordered pair. The paper calculation above and
its shared fixture stay unchanged. The Go trace is `Go(3000) -> Rust(3001) ->
Go(3002)`, using Go's independent gradient engine on both Go steps. Four fresh
models, eight native baseline resumes and twenty-four exchange steps produce
36 actual child executions. A registry entry alone cannot produce this evidence.
See [NN42](../../specs/NN42-four-native-dense-conformance.md).

On a sparse local checkout, `--rust-manifest-path` may name a disposable
workspace containing exact Rust/matrix/bounded-JSON/fixture copies. That proves
native numerical interchange locally, not full repository-workspace resolution;
the required CI job separately builds/tests that real workspace.

Automatic changes to any native lane, its matrix foundation, shared fixture,
or validator now select a reusable conformance job inside repository CI. The
required `CI gate` rejects a selected job that fails, is cancelled, or skips;
manual dispatch remains available. See the
[CI ownership contract](../../specs/neural-framework-ci-gate.md).

NN40 adds one [shared checkpoint corpus](../../specs/fixtures/neural-framework-v1/checkpoints.json):
originally 49 small documents, with 3 accepted controls and 46 rejected cases. Each native
package test suite loads the same documents through its public API. For example,
`step: false` must fail: a truth value is not an optimizer-update count.
The trace is `JSON document -> native loader -> accept/reject`; accepted models
must also survive save/reload unchanged. Valid controls prevent a loader that
rejects everything from passing. This tests parsed-document portability, not
identical text-parser handling of duplicate keys or nesting limits. See
[NN40](../../specs/NN40-shared-checkpoint-rejection-corpus.md).

NN43 extends that same corpus to 55 documents: the original 49 plus six explicit
null numeric coordinates, with 3 accepted controls and 52 rejections. For example,
`weights: [[null]]` is not `weights: [[0]]`: null means no numeric value was
supplied, whereas zero is a legitimate parameter. Some decoders can silently
convert the former to the latter, so each native loader must reject the original
document. The shared trace also puts null among valid neighbors in rectangular
arrays and in a connected second layer, ensuring validation is not just a check
of the first element. Python, TypeScript, Rust and Go load these identical data
through their public APIs; no production algorithm or checkpoint field changes.
See [NN43](../../specs/NN43-shared-null-numeric-checkpoints.md).

The next framework milestones add native ports, shared tensor autograd,
architecture components, and measured increases in model size. See the
[framework roadmap](./FRAMEWORK-ROADMAP.md). A successful dense checkpoint does
not yet establish trainable CNN, recurrent, transformer, or GPU support.
