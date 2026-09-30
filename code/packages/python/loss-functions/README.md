# Loss Functions (Python)

Pure Python implementations of the ML01 mean squared error, mean absolute
error, binary cross-entropy, and categorical cross-entropy functions together
with their prediction derivatives.

## Usage

```python
from loss_functions import bce, bce_derivative, mse

regression_error = mse([1.0, 0.0], [0.9, 0.1])
classification_error = bce([1.0, 0.0], [0.9, 0.1])
gradient = bce_derivative([1.0, 0.0], [0.9, 0.1])
```

Every function rejects empty or unequal-length inputs. Cross-entropy functions
clamp predictions to a safe epsilon range before logarithm or division.

## Validation

```bash
uv venv .venv --no-project --clear --python 3.12
uv pip install --python .venv -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```
