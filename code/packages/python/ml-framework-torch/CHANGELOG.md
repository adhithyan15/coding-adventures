# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- **nn.Module** — Base class for all neural network layers with automatic parameter registration via `__setattr__`, train/eval mode switching, state dict serialization, and recursive parameter iteration.
- **nn.Linear** — Fully connected layer (`y = x @ W.T + b`) with Xavier initialization and autograd-tracked bias broadcasting via matmul.
- **nn.Sequential** — Container that chains layers in order.
- **nn.ReLU, nn.GELU, nn.Sigmoid, nn.Tanh** — Activation layers wrapping ml-framework-core Functions.
- **nn.Softmax, nn.LogSoftmax** — Probability distribution layers with numerical stability via log-sum-exp trick.
- **nn.Dropout** — Regularization with inverted dropout scaling.
- **nn.BatchNorm1d** — Batch normalization with running statistics.
- **nn.LayerNorm** — Layer normalization (per-sample, no running stats).
- **nn.Embedding** — Token embedding lookup table.
- **nn.Flatten** — Reshape multi-dimensional input to 2-D.
- **nn.MSELoss, nn.L1Loss** — Regression losses.
- **nn.CrossEntropyLoss** — Multi-class classification loss (LogSoftmax + NLL).
- **nn.BCELoss, nn.BCEWithLogitsLoss** — Binary classification losses.
- **nn.NLLLoss** — Negative log-likelihood loss.
- **nn.functional** — Stateless functional API (F.relu, F.linear, F.cross_entropy, etc.).
- **optim.SGD** — Stochastic Gradient Descent with momentum and weight decay.
- **optim.Adam** — Adaptive Moment Estimation with bias correction.
- **optim.AdamW** — Adam with decoupled weight decay.
- **optim.RMSprop** — Root Mean Square Propagation with optional momentum.
- **utils.data.Dataset** — Abstract dataset base class.
- **utils.data.TensorDataset** — Dataset wrapping pre-loaded tensors.
- **utils.data.DataLoader** — Batch iterator with shuffle and drop_last support.
- **Top-level API** — `torch.tensor()`, `torch.zeros()`, `torch.ones()`, `torch.randn()`, `torch.eye()`, `torch.arange()`, `torch.full()`.
- **192 tests** covering all modules with 97% code coverage.
