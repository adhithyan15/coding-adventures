# Activation Functions (Python)

Pure scalar activation functions and derivatives for educational neural-network
implementations. The package implements the ML04 linear, sigmoid, ReLU, leaky
ReLU, tanh, and softplus contract using only Python's standard `math` module.

## Usage

```python
from activation_functions import sigmoid, sigmoid_derivative, softplus, tanh

probability = sigmoid(1.0)
slope = sigmoid_derivative(1.0)
hidden_value = tanh(-0.5)
smooth_positive = softplus(2.0)
```

`tanh_func` remains available as a compatibility alias for the canonical
`tanh` name. Sigmoid and softplus use overflow-safe formulas for large inputs.

## Validation

```bash
uv venv .venv --no-project --clear --python 3.12
uv pip install --python .venv -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```
