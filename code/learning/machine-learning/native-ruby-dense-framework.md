# A native Ruby dense framework

NN45 adds a reusable Ruby learner, not a binding to somebody else's training
loop. Its matrix dependency performs native Ruby dot products. A dense layer
first multiplies each input row by input-by-output weights, then adds biases
and applies an activation. For weights `[[2], [3]]`, bias `[1]`, input `[[4, 5]]`,
the prediction is `4*2 + 5*3 + 1 = 24`.

The word gradient means a slope: how much the mean squared error changes when
one parameter changes a little. Backpropagation carries those slopes backward
through every activation and matrix product. Central finite differences check
the analytical result by nudging every weight and bias in both directions.
Tests use unequal layer widths and multiple outputs so accidental transposes
and missing reductions cannot hide behind a symmetric toy network.

One SGD update prepares a complete candidate network. Its weights, predictions,
loss and gradients must stay finite before any model state is adopted. If an
overflow occurs, neither parameters nor the training step change. Fit repeats
exactly the requested number of successful updates and records the initial
loss too; zero epochs is a real no-update operation.

Ruby's matrix foundation deliberately aliases mutable arrays. The framework
therefore validates and copies them before constructing a Matrix. Checkpoint
arrays, prediction arrays and gradient arrays are independent values, never
handles into the trained model. JSON `null` is not numeric zero: all 55 shared
checkpoint documents run directly through the public Ruby loader, including
the six explicit null-cell regressions.

The native XOR command trains for 3000 steps and emits a JSON checkpoint.
Bounded `--resume` stdin advances exactly one step; tests compare every saved
parameter and prediction against uninterrupted training through 3002. The
Python validator only launches the fixed Ruby command and checks receipts;
it does not do the training. An isolated gem-install smoke confirms both Ruby
libraries load from their installed payloads, not accidentally from checkout.

This delivery proves Ruby native training. Integrating it into the existing
four-native exchange graph is a separate prerequisite for five-native claims.
See the [package API](../../packages/ruby/neural-framework/README.md) and
[NN45 contract](../../specs/NN45-native-ruby-dense-framework.md).
