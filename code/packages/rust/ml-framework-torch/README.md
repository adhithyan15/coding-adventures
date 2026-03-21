# ml-framework-torch

PyTorch-compatible API built on top of ml-framework-core.

## Architecture

This package is a thin wrapper. Almost all computation is done by
ml-framework-core. This layer adds:

1. **nn::Module** -- base trait for layers with parameter registration
2. **nn::Linear**, **nn::ReLU**, etc. -- standard neural network layers
3. **nn::MSELoss**, **nn::CrossEntropyLoss** -- loss functions
4. **optim::SGD**, **optim::Adam** -- parameter update algorithms
5. **data::TensorDataset**, **data::DataLoader** -- batch training utilities

## Usage

```rust
use ml_framework_torch::*;

// Build a model
let model = nn::Sequential::new(vec![
    Box::new(nn::Linear::new(784, 128, true)),
    Box::new(nn::ReLU),
    Box::new(nn::Linear::new(128, 10, true)),
]);

// Create optimizer
let mut optimizer = optim::SGD::new(model.parameters(), 0.01, 0.0, 0.0);

// Training loop
let loss_fn = nn::MSELoss::new("mean");
let output = model.forward(&input);
let loss = loss_fn.forward(&output, &target);
loss.backward(None).unwrap();
optimizer.step();
optimizer.zero_grad();
```
