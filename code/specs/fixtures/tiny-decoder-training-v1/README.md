# Tiny Decoder Training V1 Fixtures

This corpus pins a two-position decoder-only next-token training step: causal
sequence shift, shared unembedding logits, stable softmax, mean cross-entropy,
state and parameter gradients, one SGD update, and the resulting lower loss.
The analytical shared-head gradients are also checked by central finite
differences.

```text
python code/scripts/validate_tiny_decoder_training_labs.py
```

See [`NN15-tiny-decoder-training-labs.md`](../../NN15-tiny-decoder-training-labs.md).
