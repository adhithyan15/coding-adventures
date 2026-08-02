# ml-learning-visualizer

Interactive machine learning lab for building intuition around small models.

## What It Shows

The app has nine workbenches that move from one arithmetic update to small
spatial and hidden-layer networks.

### Training-step microscope

The default view pauses one neuron on one training example and reveals seven
phases:

```text
example -> multiply -> add bias -> activate -> loss -> backprop -> update
```

Future values stay hidden until their phase is selected. The learner can change
the input, target, weight, bias, activation, and learning rate, inspect the
chain-rule factors, and apply exactly one proposed update.

### Optimization microscope

The optimization workbench plots the mean-squared-error surface for a
four-point linear problem. It shows the current parameters, the known minimum,
and one full-batch gradient step on the same surface. An independent central
finite-difference estimate checks the analytical gradient, while synchronized
loss curves compare stochastic, mini-batch, and full-batch row selection.

### Linear lab

The linear workbench includes 100 selectable examples that reuse one training
shell:

```text
y = weight * x + bias
```

You can tune the learning rate, initial weight, initial bias, loss function, and
activation preview, then step through gradient descent while the fitted line,
prediction points, loss, gradients, and error distances update.

### Hidden-layer lab

The hidden workbench traces XOR-like logic, absolute value, piecewise pricing,
circle classification, two moons, and feature interactions. It exposes selected
hidden activations, matrix shapes, gradients, parameter updates, and the shared
neural graph VM execution path.

### Spatial lab

The spatial workbench slides an editable one-dimensional kernel over an
editable signal in valid, stride-one mode. Select any feature-map output to see
its receptive-field window, element-wise products, and running accumulator. An
asymmetric default kernel makes the neural cross-correlation convention
visible instead of silently flipping weights. The same selected output exposes
its loss derivative and contribution to each shared kernel weight. A reduction
table checks the analytical gradient against central finite differences and
previews or applies one loss-reducing update.

### Image CNN lab

The image workbench opens a complete two-channel image pipeline. Select either
filter and any `2 x 2` output position to inspect its two input windows,
channel-specific kernels, products, partial sums, bias, and final feature-map
value. Step forward to see which four values share normalization statistics,
how ReLU changes them, and which row-major location wins `2 x 2` max pooling.

### Residual and receptive-field lab

The residual workbench sends a five-value signal through two same-padded local
layers and an identity shortcut. Select any output to expose both routes at the
addition, expand the local route through its hidden values, and count every
path back to the original inputs. Boundary selections make receptive-field
clipping visible, while the shortcut toggle separates what depth changes from
what the identity path preserves.

### Recurrent-state lab

The recurrent workbench unrolls one scalar ReLU cell across three time steps.
Select any execution to separate its new-input product, previous-state product,
bias, preactivation, and next state. The same parameter strip stays above all
three cells to make sharing explicit. A memory toggle removes only the
recurrent contribution and exposes why the final zero input still produces a
positive state when earlier context is carried forward.

Switch to the backward microscope to reverse the same saved states. Select a
time step to combine direct and future state gradients, inspect its local
parameter contributions, and see all three executions reduce into one shared
gradient set. A central finite-difference table audits BPTT independently, and
an update preview reruns the forward pass with the proposed parameters.

Continue to the gate comparator to align a scalar GRU and LSTM on the same
inputs. Select reset, update, forget, input, or output; then use its canonical
value or force it to zero or one. The two lanes remain visible together, and
the LSTM keeps its private cell state separate from its exposed hidden state.

### Attention score lab

The attention workbench projects `red`, `blue`, and `purple` into query, key,
and value vectors with three shared matrices. Open any cell in the aligned
`3 x 3` score matrix to see its coordinate products and dot-product sum. A
single control applies the standard square-root dimension scaling while
keeping the raw score visible in the arithmetic. Values remain explicitly
downstream: this lab stops before softmax, masking, or weighted value mixing.

## Lab Families

- Basics: clean linear relationships such as Celsius to Fahrenheit.
- Learning rate: examples tuned to show slow, useful, and unstable step sizes.
- Loss functions: MSE vs MAE with clean points and outliers.
- Scaling: normalized, centered, and wide input ranges.
- Noise: increasingly noisy observations around the same underlying trend.
- Generalization: sparse and curved data where a line has limits.
- Real data: a checked-in CC0 Palmer Penguins CSV sample.

## Dataset Policy

The browser app should not live-load Hugging Face, Kaggle, or other remote
datasets. Small teaching datasets can be checked in as local JSON or CSV only
when their license and source are clear. Dataset notes live in
`src/data/SOURCES.md`.

The smallest canonical examples also live in language-neutral corpora. NN03
under `code/specs/fixtures/neural-learning-v1` pins forward values and first-step
gradients. NN04 under `code/specs/fixtures/optimization-learning-v1` pins
finite-difference checks and batch-strategy trajectories so other language
implementations can reproduce the same visualized arithmetic.

NN05 under `code/specs/fixtures/convolution-learning-v1` pins the first
sliding-kernel trace, including every window, product, accumulator, and output.
NN06 under `code/specs/fixtures/convolution-training-v1` pins every shared-
weight gradient contribution, the finite-difference oracle, and one optimizer
step.

NN07 under `code/specs/fixtures/tiny-image-cnn-v1` pins two-channel 2D
cross-correlation, output-channel normalization statistics, ReLU maps, and
max-pool values and winner coordinates.

NN08 under `code/specs/fixtures/residual-receptive-v1` pins two same-padded
local layers, the identity addition, all hidden-to-input expansions, input
path counts, and boundary-clipped receptive fields.

NN09 under `code/specs/fixtures/recurrent-unroll-v1` pins an explicit initial
state, three shared-parameter recurrent steps, all scalar arithmetic, and the
counterfactual trace with its recurrent contribution removed.

NN10 under `code/specs/fixtures/recurrent-bptt-v1` pins the reverse-time state
gradients, local and accumulated shared-parameter gradients, initial-state
gradient, central finite-difference oracle, and one loss-reducing update.

NN11 under `code/specs/fixtures/gated-recurrent-v1` pins scalar GRU and LSTM
gate activations, candidate and memory contributions, explicit recurrent
states, and reset/update/forget/input/output counterfactuals.

NN12 under `code/specs/fixtures/attention-qkv-v1` pins three token embeddings,
the shared query/key/value projection matrices, every coordinate product in
all nine query-key dot products, and the raw and scaled score matrices.

## Development

```bash
bash BUILD
```
