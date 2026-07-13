# two-layer-network (C)

A two-layer (one hidden layer) neural network with backpropagation, in pure ISO
C17 — a faithful port of the Rust `two-layer-network` crate. **No libm**: the
sigmoid uses a from-scratch `e^x`.

## Why a hidden layer

A single dense layer can only separate data with a straight line, so it **cannot
learn XOR**. Add one hidden layer and the network learns non-linearly-separable
functions. The forward pass:

```
hidden_raw = inputs · W_ih + b_h ;  hidden     = activation(hidden_raw)
output_raw = hidden · W_ho + b_o ;  prediction = activation(output_raw)
```

`tln_train_one_epoch` runs one full-batch mean-squared-error step,
backpropagating the error through both layers (output deltas → hidden deltas via
the transposed output weights) and returning every gradient plus the next
parameters.

## API

```c
#include "two_layer_network.h"

TlnParameters p;
tln_xor_warm_start_parameters(&p);              /* a 2-2-1 XOR network */

double inputs[8] = {0,0, 0,1, 1,0, 1,1};        /* 4 samples × 2 features */
TlnForwardPass fp;
tln_forward(inputs, 4, 2, &p, TLN_ACT_SIGMOID, TLN_ACT_SIGMOID, &fp);
/* fp.predictions[1] ≈ 1 (XOR of 0,1), fp.predictions[0] ≈ 0 */

double targets[4] = {0, 1, 1, 0};
TlnTrainingStep step;
tln_train_one_epoch(inputs, 4, 2, targets, 4, 1, &p, 0.5,
                    TLN_ACT_SIGMOID, TLN_ACT_SIGMOID, &step);
/* step.next_parameters, step.loss, all gradients … */

tln_training_step_free(&step);
tln_forward_pass_free(&fp);
tln_parameters_free(&p);
```

Matrices are flat **row-major** `double` arrays. Build custom parameters with
`tln_parameters_init` (it deep-copies the four arrays).

## Divergence from the Rust crate

Rust returns `Result<_, String>`; this port returns a `TlnStatus`
(`TLN_OK` / `TLN_ERR_SHAPE` / `TLN_ERR_NOMEM`) and writes results through
out-parameters. `e^x` is computed without `<math.h>`.

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17, no `<math.h>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Where it fits

The capstone of the campaign's neural-network sequence:
[`activation-functions`](../activation-functions) →
[`loss-functions`](../loss-functions) →
[`single-layer-network`](../single-layer-network) → this two-layer network,
which adds the hidden layer that makes XOR learnable.
