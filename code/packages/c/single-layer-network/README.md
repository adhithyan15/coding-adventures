# single-layer-network (C)

A single dense neural-network layer with batch gradient descent, in pure ISO
C17 — a faithful port of the Rust `single-layer-network` crate. **No libm**: the
sigmoid uses a from-scratch `e^x`.

## What it does

One dense layer maps `input_count` features to `output_count` outputs:

```
prediction[o] = activate( bias[o] + Σ_i input[i] · weight[i][o] )
```

Training is full-batch **mean-squared-error gradient descent**. One epoch runs a
forward pass, computes `error = prediction − target` and the per-output delta
`(2/(N·O))·error·act'(pred)`, accumulates weight/bias gradients, and returns the
next parameters `param − learning_rate · grad`, along with the mean loss.

Activations are **Linear** (identity) and **Sigmoid** (numerically stable).

## API

```c
#include "single_layer_network.h"

SlnNetwork net;
sln_network_init(&net, 3, 2, SLN_ACT_LINEAR);   /* 3 inputs -> 2 outputs, zeros */

double inputs[9]  = { 0,0,1,  1,2,1,  2,1,1 };   /* 3 samples × 3 features */
double targets[6] = { 1,-1,   3,2,    4,1   };   /* 3 samples × 2 outputs  */

SlnTrainingStep *history; size_t epochs;
sln_network_fit(&net, inputs, 3, 3, targets, 3, 2, 0.05, 500, &history, &epochs);
/* history[epochs-1].loss < history[0].loss */

double query[3] = {1,1,1}, pred[2];
sln_network_predict(&net, query, 1, 3, pred);

sln_history_free(history, epochs);
sln_network_free(&net);
```

Matrices are flat **row-major** `double` arrays. `sln_train_one_epoch` exposes
one epoch's full record (`SlnTrainingStep`).

## Divergence from the Rust crate

Rust returns `Result<_, String>`; this port returns an `SlnStatus`
(`SLN_OK` / `SLN_ERR_SHAPE` / `SLN_ERR_NOMEM`) and writes results through
out-parameters. Because matrix dimensions are explicit, ragged rows aren't
representable. `e^x` is computed without `<math.h>`.

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17, no `<math.h>`. Builds clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Where it fits

Completes the neural-network trilogy in this campaign:
[`activation-functions`](../activation-functions) (the nonlinearity),
[`loss-functions`](../loss-functions) (the objective), and this layer (which
learns via gradient descent).
