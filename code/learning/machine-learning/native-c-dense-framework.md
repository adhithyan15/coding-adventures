# A native C dense learner: ownership before speed

NN48 implements training in C itself. `Mat` owns row-major double arrays and
performs matrix products, transposes and reductions; native float-math supplies
exp, tanh and square root. A Python test runner launches the native executable,
but does not supply gradients or training. Neither a Rust binding nor an
external ML engine earns native C credit.

## One hand-calculable update

For one input row `[1, 2]`, use two linear layers:

```text
input [1, 2]
  W1 [[0.5, -0.5], [1, 0.5]], b1 [0, 0]
  activation [2.5, 0.5]
  W2 [[1], [-1]], b2 [0.25]
  prediction 2.5 - 0.5 + 0.25 = 2.25
target 1; squared error (2.25 - 1)^2 = 1.5625
```

A gradient is a slope: how rapidly that error changes when one parameter moves.
The output slope is `2 * (2.25 - 1) = 2.5`. Moving backward through W2 gives
hidden slopes `[2.5, -2.5]`. Thus dW1 is `[[2.5,-2.5],[5,-5]]`, db1 is
`[2.5,-2.5]`, dW2 is `[[6.25],[1.25]]` and db2 is `[2.5]`.

With learning rate 0.01, subtract each slope times 0.01. The new hidden row is
`[2.35,0.65]`, W2 is `[[0.9375],[-1.0125]]` and b2 is `[0.225]`:
`2.35*0.9375 - 0.65*1.0125 + 0.225 = 1.77`. Loss becomes 0.5929.
The [shared fixture](../../specs/fixtures/neural-framework-v1/dense.json) pins
every one of these values. Tests also nudge every parameter in both directions
to check analytical gradients, using active ReLU paths away from its kink and
a separate test of the specified zero derivative.

## Why the network is opaque

C cannot determine whether an arbitrary pointer really contains a declared
array. The caller must provide readable buffers; the framework then validates
dimensions, finite values and closed capacities before copying. Its network
handle is opaque. Predictions, inspection matrices, traces and checkpoint
snapshots own independent copies. Resetting release functions prevent a second
release of the same handle; shallow-copying an owning C record is not ownership
transfer and remains a caller error.

One SGD step computes a candidate separately. Only after its parameters and
actual batch evaluation remain finite does the network exchange ownership and
increment its step. A failed update keeps the old model. Fit is a sequence of
those atomic updates, not a transaction across all epochs; an error after an
earlier successful step retains that step. Zero epochs returns just initial loss.

## Parsing is not accepting a model

NN47's parser retains duplicate names and explicit string lengths. NN48 rejects
duplicate, unknown, missing and embedded-NUL names; integer metadata must really
be integer tokens. `1.0` is not version 1, and `null` is not a zero weight.
All 55 shared checkpoints run through the public schema loader. JSON text also
has byte, nesting, node and arena ceilings. The serializer uses round-trip
double precision and writes negative zero as `-0.0`, so the typed integer parser
does not discard its sign. Library code never changes global locale.

The XOR command owns a fixed 2-input/1-output dataset. Even a schema-valid
checkpoint with other widths must be rejected before the command passes those
fixed C buffers to training. Checkpoint validity and dataset compatibility are
different checks.

Native tests train XOR for 3000 updates below 0.01 loss, then compare complete
serialized/resumed states with independently fresh uninterrupted baselines
through 3001 and 3002. A separate installed static-library consumer predicts,
trains and saves/loads/resumes without including checkout sources. These are
local native C results; six-native interchange belongs to the following tranche,
and actual repository CI plus exact-head merge still gate delivery.

See the [public API and commands](../../packages/c/neural-framework/README.md)
and [NN48 specification](../../specs/NN48-native-c-dense-framework.md).
