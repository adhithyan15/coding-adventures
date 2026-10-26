# Gradient Descent (Python)

Pure Python implementation of the ML02 stochastic-gradient-descent update. The
package owns one small operation: given weights, their gradients, and a learning
rate, return a new vector whose coordinates have each moved one step opposite
the gradient.

## Usage

```python
from gradient_descent import sgd

updated = sgd(
    weights=[1.0, -0.5, 2.0],
    gradients=[0.1, -0.2, 0.0],
    learning_rate=0.1,
)

assert updated == [0.99, -0.48, 2.0]
```

Inputs must be non-empty vectors of equal length. The function returns a new
list and does not mutate either input. A zero learning rate therefore returns
an equal but distinct list; a negative learning rate follows the same published
formula without imposing a convergence policy.

This package performs no training loop, random sampling, automatic
differentiation, convergence analysis, or model I/O. It is deterministic pure
arithmetic and requires no host capabilities.

## Validation

```bash
uv venv .venv --no-project --clear --python 3.12
uv pip install --python .venv --no-deps -e .
uv pip install --python .venv pytest pytest-cov ruff
.venv/bin/python -m ruff check src tests
.venv/bin/python -m ruff format --check src tests
.venv/bin/python -m pytest tests/ -v
```

The repository `BUILD` and `BUILD_windows` fronts encode the corresponding
native commands for Unix and Windows.
