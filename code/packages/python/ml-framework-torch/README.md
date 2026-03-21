# ml-framework-torch

A PyTorch-compatible API layer built on top of `ml-framework-core`. This package provides the familiar PyTorch interfaces for building, training, and evaluating neural networks.

## Architecture

```
ml-framework-torch (this package)
    ├── nn.Module, nn.Linear, nn.ReLU, ...     (layers)
    ├── nn.MSELoss, nn.CrossEntropyLoss, ...   (loss functions)
    ├── optim.SGD, optim.Adam, ...             (optimizers)
    └── utils.data.DataLoader                  (data loading)
         │
         ▼
ml-framework-core
    ├── Tensor          (n-dimensional array)
    ├── Parameter       (learnable tensor)
    ├── Autograd        (backward / gradient computation)
    └── Functions       (differentiable ops: Add, MatMul, ReLU, ...)
```

`ml-framework-core` handles all the math — tensors, autograd, and differentiable operations. This package adds the high-level abstractions that make building neural networks ergonomic.

## Quick Start

```python
import ml_framework_torch as torch

# Create tensors
x = torch.tensor([[1.0, 2.0], [3.0, 4.0]])
w = torch.randn(4, 2, requires_grad=True)

# Build a model
model = torch.nn.Sequential(
    torch.nn.Linear(2, 16),
    torch.nn.ReLU(),
    torch.nn.Linear(16, 1),
)

# Define loss and optimizer
loss_fn = torch.nn.MSELoss()
optimizer = torch.optim.Adam(model.parameters(), lr=0.01)

# Training loop
target = torch.tensor([[1.0], [0.0]])
for epoch in range(100):
    optimizer.zero_grad()
    output = model(x)
    loss = loss_fn(output, target)
    loss.backward()
    optimizer.step()
```

## Components

### nn.Module

Base class for all layers. Handles parameter registration, train/eval modes, and state serialization.

```python
class MyModel(torch.nn.Module):
    def __init__(self):
        super().__init__()
        self.fc1 = torch.nn.Linear(784, 128)
        self.relu = torch.nn.ReLU()
        self.fc2 = torch.nn.Linear(128, 10)

    def forward(self, x):
        return self.fc2(self.relu(self.fc1(x)))
```

### Layers

| Layer | Description |
|-------|-------------|
| `Linear(in, out)` | Fully connected: y = x @ W.T + b |
| `ReLU()` | max(0, x) |
| `GELU()` | Smooth ReLU used in transformers |
| `Sigmoid()` | Squash to (0, 1) |
| `Tanh()` | Squash to (-1, 1) |
| `Softmax(dim)` | Probability distribution |
| `LogSoftmax(dim)` | Numerically stable log-probabilities |
| `Dropout(p)` | Regularization by random zeroing |
| `BatchNorm1d(features)` | Batch normalization |
| `LayerNorm(features)` | Layer normalization |
| `Embedding(vocab, dim)` | Token embedding lookup |
| `Flatten()` | Reshape to 2-D |
| `Sequential(*layers)` | Chain layers in order |

### Loss Functions

| Loss | Use Case |
|------|----------|
| `MSELoss()` | Regression |
| `L1Loss()` | Robust regression |
| `CrossEntropyLoss()` | Multi-class classification |
| `BCELoss()` | Binary classification (with sigmoid) |
| `BCEWithLogitsLoss()` | Binary classification (raw logits) |
| `NLLLoss()` | With LogSoftmax output |

### Optimizers

| Optimizer | Description |
|-----------|-------------|
| `SGD(params, lr, momentum)` | Stochastic Gradient Descent |
| `Adam(params, lr, betas)` | Adaptive Moment Estimation |
| `AdamW(params, lr, weight_decay)` | Adam with decoupled weight decay |
| `RMSprop(params, lr, alpha)` | Root Mean Square Propagation |

### Data Loading

```python
from ml_framework_torch.utils.data import TensorDataset, DataLoader

dataset = TensorDataset(features, labels)
loader = DataLoader(dataset, batch_size=32, shuffle=True)

for batch_x, batch_y in loader:
    # train on batch
    pass
```

## Functional API

Stateless versions of all operations:

```python
import ml_framework_torch.nn.functional as F

y = F.relu(x)
y = F.linear(x, weight, bias)
loss = F.cross_entropy(logits, labels)
loss = F.mse_loss(pred, target)
```

## Installation

```bash
pip install coding-adventures-ml-framework-torch
```

Requires `coding-adventures-ml-framework-core`.

## Testing

```bash
mise exec -- uv run pytest tests/ -v --cov=ml_framework_torch
```

Coverage target: 95%+
