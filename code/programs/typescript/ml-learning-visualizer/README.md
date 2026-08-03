# ml-learning-visualizer

Interactive machine learning lab for building intuition around small models.

## What It Shows

The app has nineteen workbenches that move from one arithmetic update to small
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
downstream in this first view.

Continue into the causal-softmax mixer to select a whole query row. It shows
the future-key mask, stable maximum subtraction, exponentials, denominator,
and normalized weights as one connected flow. The triangular weight matrix and
three value-contribution lanes update together, while a mask toggle provides an
unmasked comparison without changing scores or values.

Continue once more into the two-head add-and-norm block. The horizontal and
vertical scalar heads stay aligned so their different score rows, weights,
value contributions, and contexts can be compared. Their contexts then move
through concatenation, an explicit output projection, residual addition, and
per-token layer normalization. Residual and normalization toggles act as
controlled ablations without changing either head.

Continue into the tiny decoder training trace to shift `red blue purple` into
two causal next-token examples. Select either position to follow its saved
decoder state through shared unembedding logits, stable softmax, target
cross-entropy, logit gradients, and the gradient entering the decoder body.
An independent finite-difference audit checks the shared-head gradients. The
before/after toggle reruns both positions with one SGD update and checks the
resulting lower mean loss.

### Representation lab

The representation workbench compresses `[2,-1]` through a one-scalar linear
bottleneck and reconstructs both coordinates from that shared value. Select
either output to isolate its decoder formula, coordinate error, and route into
the accumulated bottleneck gradient. The same view exposes all encoder and
decoder gradients, a seven-parameter central finite-difference audit, and the
full-model loss before and after one SGD step.

Switch to the variational view to encode one input as a Gaussian mean and
log-variance, transform a saved standard-normal noise value into a latent
sample, and decode it. Beta controls how reconstruction and KL-to-prior gradient
routes combine. The balance point at beta `0.25`, a six-parameter numerical
audit, and the complete post-update distribution remain directly inspectable.

Continue into the adversarial view to place one real point and one generated
point on the same number line. Advance a deterministic round from the initial
scores through a detached discriminator update and a generator response against
the frozen updated discriminator. Each player's objective, active gradient
route, two-parameter numerical audit, and the opponent's counter-push stay
visible at every phase.

Finish the representation sequence with the diffusion view. It mixes one clean
scalar with a saved noise value at two cumulative-alpha levels, trains a shared
timestep-aware noise predictor, audits all three parameter gradients, and feeds
two deterministic reverse means into each other. The final reconstruction keeps
its small residual error visible rather than implying a perfect toy denoiser.

### Structured and memory lab

The structured workbench begins with a four-neuron Hopfield memory. It turns one
bipolar pattern into a symmetric zero-diagonal Hebbian weight matrix, presents a
cue with one flipped bit, and advances a saved asynchronous update order. Every
incoming weighted vote, local field, state change, energy value, and normalized
overlap stays visible through the recovered fixed point.

Switch to message passing to open a three-node path. The view expands two
undirected edges into four directed messages, highlights the selected node's
inbox, sums its messages, and combines that aggregate with a shared self route,
bias, and ReLU. All nodes read the same original feature snapshot.

Finish the structured sequence with the GCN-versus-GAT comparison. Select any
self-looped neighborhood, inspect every square-root degree coefficient, then
switch to stable neighborhood softmax and audit scores, exponentials, attention
weights, contributions, and both output vectors.

### Deep-training lab

The deep-training workbench sends four fixed two-value samples through three
layers with one shared sign template. Switch between tiny, Xavier, He, and
deliberately large scales, then compare `tanh` and ReLU. Every layer exposes its
eight activation values, population variance and standard deviation, exact-zero
or saturation fraction, and one selected neuron's full arithmetic. A comparison
panel keeps all four initializer trajectories visible at once.

Switch to gradient flow to reverse four scalar layers. Small-weight and
saturated `tanh` chains show two ways a gradient can vanish, unit ReLU provides
a stable control, and large ReLU doubles every local Jacobian. Select any layer
to expand `dL/da`, `da/dz`, `dL/dz`, the input gradient, and the weight gradient.
The complete chain product and an independent input finite difference remain
visible beside all four scenarios.

Switch to stabilizers to hold one four-coordinate learned branch fixed while
comparing a plain control, population layer normalization, deterministic
inverted dropout, and an identity residual. Open any coordinate to see the
selected vector-Jacobian-product arithmetic, shared normalization sums,
dropout mask scaling, residual branch/skip split, input gradient, branch-weight
gradient, and matching central finite differences. Dropout's evaluation output
and exact training-mask expectation remain visible beside its sampled output.

### Tensor shape and broadcasting lab

The Tensor + Autograd workbench lines scalar, vector, and matrix shapes up from
the right. Switch among two-sided expansion, rank padding, scalar expansion,
and one mismatch. For every valid case, open any output cell to see the exact
left and right coordinates it reuses. The reverse panel sums upstream values
over expanded axes to recover both original input shapes and checks every
gradient with central finite differences.

### Dynamic autograd and saved-value lab

The autograd graph workbench builds three scalar graphs as the operations run.
Open any node to see its executed operation, parents, forward value, and the
minimal immutable snapshots its local backward rule needs. Reverse-order
buttons expand every upstream-gradient times local-derivative calculation.
Switch to a negative runtime branch to see the unused identity operation stay
out of the graph, or mutate a live input after forward and compare it with the
saved value that backward correctly preserves. Fresh forward executions audit
every leaf gradient with central finite differences.

### Gradient accumulation and zeroing lab

The Grad Buffers workbench treats `w` and persistent `w.grad` as separate
mutable state. Open every scheduled backward, optimizer, and zero call to see
the buffer before and after. Compare two accumulating backward calls with a
reset between them, average two micro-batches before one SGD step, and omit the
reset to watch a new `0.8` gradient contaminate a stale buffer of `4`. The
optimizer panel makes its non-clearing behavior explicit, while fresh forward
passes audit every local gradient numerically.

### Forward graph lowering lab

The IR Lowering workbench holds one six-node weighted-ReLU graph fixed while it
becomes a deterministic twelve-instruction NN00 `CANN` forward function and a
six-operation NN01 `CANM` matrix plan. Open any scalar instruction to inspect
its first-row reads and write, or open any matrix operation to reveal its exact
source instructions, graph nodes, and edge provenance. Switch from one row to a
two-row batch while direct graph, NeuralIR, and MatrixIR outputs remain aligned.

### Backward and optimizer lowering lab

The Train Lowering workbench keeps a one-parameter multiplication fixed. Its
saved predictions come from the production neural graph compiler and VM. Open
any of six backward instructions, four separate SGD instructions, or ten
matrix-training operations to see reads, writes, values, attributes, and source
provenance. Switch from one row to two, then enter with a nonzero buffer to see
the current batch reduce separately before it accumulates into persistent
state. The optimizer applies an explicit divisor, updates `w`, and leaves the
buffer populated. A fresh finite-difference route audits only the current batch
contribution.

### Backend parity lab

The Backend Parity workbench holds one dense batch and graph contract fixed
while four execution lanes expose their different mechanics. The production
scalar graph and TypeScript matrix plan execute in the browser; a
Node-free Rust helper is checked against the same `f32` payload bytes in its
native test; and a button requests a real WebGPU adapter and dispatches the
same multiply-plus-bias graph when hardware support exists. The output table
keeps binary64 versus `f32`, host versus device residency, and live versus
fixture evidence separate, including honest unavailable and error states.

### Precision and residency lab

The Precision + Residency workbench keeps `y = x * 2` fixed while binary32,
binary16, and symmetric int8 place two close inputs on progressively coarser
number grids. Every encoded input, accumulator, output, and absolute error stays
visible. A separate binary32 transfer baseline contrasts eager copies with
resident buffers as the learner changes the repeat count; the view reports byte
counts without presenting them as a timing benchmark.

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

NN13 under `code/specs/fixtures/attention-softmax-v1` pins both masked and
unmasked stable-softmax rows, including allowed positions, shifts,
exponentials, denominators, weights, value contributions, and contexts.

NN14 under `code/specs/fixtures/multi-head-attention-v1` pins two independently
projected scalar heads, every causal-softmax intermediate, head concatenation,
output projection, residual addition, and population-variance layer
normalization.

NN15 under `code/specs/fixtures/tiny-decoder-training-v1` pins a causal
next-token sequence shift, saved decoder states, unembedding logits, stable
softmax, mean cross-entropy, state and shared-head gradients, central finite
differences, one SGD update, and its post-update probabilities and loss.

NN16 under `code/specs/fixtures/two-number-autoencoder-v1` pins a two-number
input, one-scalar encoding, two reconstructed outputs, mean squared error,
complete decoder and encoder gradients, all seven central finite differences,
updated parameters, and the lower post-update reconstruction loss.

NN17 under `code/specs/fixtures/variational-autoencoder-v1` pins a scalar
Gaussian encoder, saved epsilon, reparameterized sample, half-squared
reconstruction error, beta-weighted KL divergence, both encoder gradient routes,
six central finite differences, updated parameters, and lower total objective.

NN18 under `code/specs/fixtures/one-dimensional-gan-v1` pins one complete
scalar adversarial round: saved noise, real and fake scores, a detached
discriminator update, a frozen-discriminator generator response, both players'
loss snapshots, and two objective-specific finite-difference audits.

NN19 under `code/specs/fixtures/one-dimensional-diffusion-v1` pins two scalar
forward noise levels, cumulative schedule coefficients, saved-noise targets, a
shared timestep-conditioned denoiser update, a three-parameter numerical audit,
and the complete deterministic reverse-mean path.

NN20 under `code/specs/fixtures/hopfield-associative-memory-v1` pins one
four-neuron bipolar memory, normalized Hebbian weights, a one-bit-damaged cue,
all in-place asynchronous updates, energy descent, overlap improvement, and the
recovered fixed point.

NN21 under `code/specs/fixtures/tiny-message-passing-v1` pins one synchronous
three-node graph round, including every directed message, target inbox,
aggregate, self contribution, affine preactivation, ReLU, and output feature.

NN22 under `code/specs/fixtures/graph-convolution-attention-v1` pins the same
three self-looped neighborhoods under symmetric-normalized GCN and stable-
softmax GAT, including every coefficient, score, weight, contribution, and
output.

NN23 under `code/specs/fixtures/initialization-activation-distribution-v1`
pins three bias-free layers under four scales and two activations, including a
canonical Xavier-plus-tanh trace and cross-mode standard-deviation, zero, and
saturation summaries.

NN24 under `code/specs/fixtures/gradient-flow-v1` pins four scalar chains,
every activation derivative and local Jacobian, all reverse-mode input and
weight gradients, vanishing/stable/exploding classifications, and input finite-
difference checks.

NN25 under `code/specs/fixtures/training-stabilizers-v1` pins the shared branch,
population layer-normalization statistics, deterministic inverted dropout and
its expectation, an identity residual, all vector-Jacobian products, and input
plus branch-weight finite-difference checks.

NN26 under `code/specs/fixtures/tensor-broadcasting-v1` pins right-aligned
shape inference, every output-to-input coordinate mapping, reduction over
expanded axes, scalar and rank-padding cases, deterministic mismatch details,
and all input finite-difference checks.

NN27 under `code/specs/fixtures/dynamic-autograd-v1` pins executed dynamic graph
topology, actual branch operations, immutable saved snapshots, reverse steps,
live post-forward mutations, and analytical-versus-numerical leaf gradients.

NN28 under `code/specs/fixtures/gradient-accumulation-v1` pins persistent
gradient-buffer transitions across backward, optimizer, and explicit-zero
events, including mean scaling and one intentionally stale next batch.

NN29 under `code/specs/fixtures/forward-graph-lowering-v1` pins one stable
topological schedule, exact NeuralIR value IDs, seven-to-one weighted-sum
fusion, complete source provenance, and direct/NeuralIR/MatrixIR parity for one
row and a two-row batch.

NN30 under `code/specs/fixtures/backward-optimizer-lowering-v1` pins saved
forward values, six backward instructions, four optimizer instructions, ten
matrix-training operations, stable row reduction, explicit mean divisors,
non-clearing optimizer steps, and numerical gradient audits.

NN31 under `code/specs/fixtures/backend-parity-v1` pins one canonical MatrixIR
graph, lowercase little-endian `f32` payloads, scalar and TypeScript CPU traces,
Rust execution-helper parity, and the expected output for an optional live
WebGPU dispatch.

NN32 under `code/specs/fixtures/precision-residency-v1` pins exact binary32 and
binary16 payloads, symmetric int8 bytes and scales with ties-to-even rounding,
and a binary32 eager-versus-resident transfer baseline.

## Development

```bash
bash BUILD
```
