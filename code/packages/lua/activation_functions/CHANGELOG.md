# Changelog — coding-adventures-activation-functions (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-29

### Added

- `sigmoid(x)` — logistic function 1/(1+e^(-x)), clamped for x outside [-709, 709]
- `sigmoid_derivative(x)` — σ(x)·(1−σ(x))
- `relu(x)` — max(0, x)
- `relu_derivative(x)` — sub-gradient: 1 if x > 0, else 0
- `tanh_activation(x)` — delegates to Lua's `math.tanh`
- `tanh_derivative(x)` — 1 − tanh(x)²
- `leaky_relu(x, alpha)` — x if x > 0 else alpha*x (default alpha=0.01)
- `leaky_relu_derivative(x, alpha)` — 1 if x > 0 else alpha
- `elu(x, alpha)` — x if x≥0 else alpha*(e^x−1) (default alpha=1.0)
- `elu_derivative(x, alpha)` — 1 if x≥0 else alpha*e^x
- `softmax(values)` — numerically stable softmax with max-subtraction trick
- `softmax_derivative(values)` — diagonal Jacobian entries s_i·(1−s_i)
- Comprehensive busted test suite with finite-difference gradient checks
