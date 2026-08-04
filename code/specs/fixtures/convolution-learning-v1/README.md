# Convolution Learning Fixtures V1

This corpus accompanies NN05. It pins a neural-network-style valid 1D
cross-correlation and exposes every multiply-accumulate performed while the
shared kernel slides across the signal.

Validate it from the repository root:

```text
python code/scripts/validate_convolution_learning_labs.py
```

The first lab deliberately uses an asymmetric kernel so that reversing the
kernel produces different answers and fails validation.
