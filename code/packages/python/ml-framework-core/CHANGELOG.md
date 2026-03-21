# Changelog

## 0.1.0 (2026-03-20)

### Added
- Tensor class: n-dimensional array with automatic differentiation
- Autograd engine: computation graph, topological sort, backward()
- 20+ differentiable Functions: Add, Sub, Mul, Div, MatMul, Pow, Sum, Mean,
  Exp, Log, Abs, Clamp, ReLU, Sigmoid, Tanh, GELU, Softmax, Reshape, Transpose
- Parameter class: learnable tensor (always requires_grad=True)
- DeviceManager: maps device strings to BLAS backends
- no_grad() context manager for inference mode
- Factory methods: zeros, ones, randn, eye, arange, from_list, full
- Shape operations: reshape, transpose, flatten, squeeze, unsqueeze
- BLAS bridge: _to_blas_matrix(), _to_blas_vector(), _from_blas_matrix()
