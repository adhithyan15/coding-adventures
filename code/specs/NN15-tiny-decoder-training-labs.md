# NN15: Tiny Decoder-Only Language Model Training Labs

## Status

Draft specification for deterministic, language-neutral next-token loss and
output-head update traces over saved causal decoder states.

## Purpose

NN15 closes the first transformer learning loop. Earlier labs produced causal
decoder states. This lab shifts a three-token sequence into two next-token
examples, turns each saved state into vocabulary logits, applies stable
softmax and mean cross-entropy, sends the error back into the decoder stream,
reduces shared gradients, and performs one SGD update.

The decoder body is intentionally frozen for this first optimizer trace. The
fixture still records the gradient entering each saved decoder state, while
only the shared unembedding matrix and bias are updated. A later automatic-
differentiation tranche can continue those state gradients through every
attention and feed-forward parameter without changing this contract.

## Sequence Shift and Causal Contract

V1 uses the vocabulary `red`, `blue`, `purple` and the sequence:

```text
red blue purple
```

The model never receives the token it must predict at the same position:

```text
decoder input  = [red, blue]
next targets   = [blue, purple]
causal prefixes = [[red], [red, blue]]
```

The saved two-coordinate decoder states are `[1,0]` and `[0,1]`. They stand at
the boundary after a causal decoder block and before the vocabulary head.

## Forward Contract

Treat each decoder state as a row and the unembedding as a
`model_width x vocabulary_size` matrix. For position `i` and vocabulary item
`v`:

```text
logit_products[i,v,d] = state[i,d] * unembedding[d,v]
logit[i,v] = sum_d(logit_products[i,v,d]) + bias[v]
```

Apply stable softmax separately at each position:

```text
row_max = max(logits)
shifted[v] = logits[v] - row_max
probability[v] = exp(shifted[v]) / sum_k(exp(shifted[k]))
```

The position loss is the negative log probability of its next-token target.
The batch loss is the arithmetic mean across both positions.

## Backward and Update Contract

Because the loss reduction is `mean`, the logit gradient already contains the
factor `1 / position_count`:

```text
d_mean_loss/d_logit[i,v]
  = (probability[i,v] - one_hot(target[i],v)) / position_count
```

Each position contributes to the same parameters:

```text
unembedding_gradient_contribution[i,d,v]
  = state[i,d] * logit_gradient[i,v]

bias_gradient_contribution[i,v] = logit_gradient[i,v]

state_gradient[i,d]
  = sum_v(logit_gradient[i,v] * unembedding[d,v])
```

Reduce position contributions by addition, then apply SGD:

```text
parameter_after = parameter_before - learning_rate * gradient
```

Before the update, independently perturb every unembedding and bias parameter
by `+epsilon` and `-epsilon`, rerun the mean loss, and estimate its gradient:

```text
numerical_gradient
  = (loss(parameter + epsilon) - loss(parameter - epsilon)) / (2 * epsilon)
```

The canonical lab uses `epsilon = 1e-6` and records every numerical gradient
plus the maximum absolute difference from the analytical gradients.

The post-update trace reruns logits, stable softmax, and loss with the updated
unembedding and bias. Its mean loss must be lower for the canonical lab.

## Fixture Layout

```text
code/specs/fixtures/tiny-decoder-training-v1/
  schema.json
  labs/00-two-position-next-token-step.json
```

Consumers reject duplicate keys, non-finite numbers, unknown fields, incorrect
shapes, unsupported operation metadata, and trace values outside the declared
absolute tolerance.

## Conformance Levels

1. **Shift:** reproduce the input/target alignment and causal prefixes.
2. **Predict:** reproduce every unembedding product, logit, and softmax value.
3. **Measure:** reproduce target probabilities, position losses, and mean loss.
4. **Differentiate:** reproduce logit, state, and per-position parameter gradients.
5. **Audit:** reproduce the central finite-difference gradient check.
6. **Update:** reduce shared gradients, perform SGD, and reproduce post-update loss.

## Cross-Language and Rust-Core Direction

Every language should implement these tiny scalar loops first. The fixture is
the portable oracle for row-major and column-major implementations alike; only
the mathematical indices, not an in-memory layout, are normative.

A performant Rust core can later expose batched matrix multiplication,
stable-softmax cross-entropy, fused logit gradients, shared-gradient reduction,
and SGD kernels through a stable C ABI. That ABI should make batch size,
position count, model width, vocabulary size, strides, target indices,
reduction, learning rate, and caller-owned buffers explicit. A trace mode must
be able to return logits, probabilities, per-position losses, state gradients,
and reduced parameter gradients even when the fast path fuses operations.
