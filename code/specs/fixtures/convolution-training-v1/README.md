# Convolution Training Fixtures V1

This corpus accompanies NN06. It pins one trainable valid 1D
cross-correlation step: forward outputs, mean-squared-error derivatives,
per-position shared-kernel gradient contributions, an independent numerical
gradient, and the updated kernel and loss.

Validate it from the repository root:

```text
python code/scripts/validate_convolution_training_labs.py
```

The first lab reuses NN05's signal and asymmetric kernel. Only two outputs are
wrong, keeping the complete backward pass small enough to reproduce by hand.
