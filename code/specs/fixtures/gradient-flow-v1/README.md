# Gradient Flow V1 Fixtures

This NN24 corpus follows one half-squared-error gradient through four scalar
layers. It pins small-tanh and saturated-tanh vanishing paths, a unit-ReLU
control, a large-ReLU exploding path, every reverse-mode intermediate, and a
central finite-difference audit of the input gradient.

Validate it with:

```text
python code/scripts/validate_gradient_flow_labs.py
```
