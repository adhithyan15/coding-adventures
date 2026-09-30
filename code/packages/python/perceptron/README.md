# Perceptron (Python)

An educational binary classifier composed from the repository's Python matrix,
activation-functions, and loss-functions packages. Training uses batch gradient
descent with sigmoid activation and binary cross-entropy loss.

## Usage

```python
from perceptron import Perceptron

features = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]
labels = [[0.0], [0.0], [0.0], [1.0]]

model = Perceptron(learning_rate=0.8, epochs=5000)
model.fit(features, labels, log_steps=5001)
predictions = model.predict(features)
```

`predict` raises until the model has been trained. After `fit`, the weight
matrix contains one row per input feature and one output column.

## Validation

```bash
uv venv .venv --no-project --clear --python 3.12
uv pip install --python .venv --no-deps -e ../activation-functions -e ../matrix -e ../loss-functions -e .
uv pip install --python .venv pytest pytest-cov ruff
.venv/bin/python -m pytest tests/ -v
```
