# Changelog

## 0.1.0 — 2026-03-21

### Added
- `Tensor` struct with flat `Vec<f64>` storage in row-major order
- Factory methods: `zeros`, `ones`, `full`, `randn`, `eye`, `arange`, `from_slice`
- Element-wise arithmetic: add, sub, mul, div, neg, pow
- Matrix multiplication (`matmul`)
- Shape operations: reshape, transpose
- Reduction operations: sum, mean
- Element-wise math: exp, log, relu, sigmoid, tanh, gelu, softmax, abs, clamp
- Comparison operations: eq, gt, lt
- Reverse-mode automatic differentiation (backpropagation)
- `Parameter` wrapper for learnable weights (always requires_grad)
- `DeviceManager` for device-to-backend mapping
- `no_grad` flag to disable gradient tracking
- Comprehensive test suite covering all operations and autograd
