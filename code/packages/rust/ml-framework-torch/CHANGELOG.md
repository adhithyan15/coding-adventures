# Changelog

## 0.1.0 — 2026-03-21

### Added
- `tensor()`, `zeros()`, `ones()`, `randn()`, `eye()`, `arange()`, `full()` factory functions
- `nn::Module` trait for all neural network layers with parameter registration
- `nn::Linear` fully connected layer with Xavier initialization
- `nn::Sequential` container that chains layers in order
- `nn::ReLU`, `nn::GELU`, `nn::Sigmoid`, `nn::Tanh`, `nn::Softmax`, `nn::LogSoftmax` activations
- `nn::MSELoss`, `nn::CrossEntropyLoss`, `nn::BCELoss`, `nn::L1Loss`, `nn::NLLLoss` losses
- `nn::Dropout` regularization layer
- `nn::BatchNorm1d`, `nn::LayerNorm` normalization layers
- `nn::Embedding` lookup table layer
- `nn::Flatten` reshape layer
- `nn::functional` module with stateless versions of all operations
- `optim::Optimizer` trait with zero_grad/step interface
- `optim::SGD` with momentum and weight decay
- `optim::Adam` and `optim::AdamW` optimizers
- `optim::RMSprop` optimizer
- `data::TensorDataset` and `data::DataLoader` for batch iteration
- Comprehensive test suite
