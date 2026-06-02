# ml_framework_core — Ruby ML framework on the Rust matrix-cpu engine

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Ruby](https://img.shields.io/badge/Ruby-%3E%3D%202.6.0-red)](https://www.ruby-lang.org/)
[![Tests](https://img.shields.io/badge/tests-148%20passing-brightgreen)]()

A small, PyTorch-shaped Ruby ML library.  All 15 differentiable ops, full
autograd, end-to-end MLP training — all in idiomatic Ruby.  Tensors
under 10k cells stay in pure Ruby; tensors above auto-dispatch through
the `matrix_rust_ruby` gem to the Rust `matrix-cpu` executor for
SIMD-accelerated f32 math.

## Quick start

```ruby
require "coding_adventures/ml_framework_core"
T = CodingAdventures::MLFrameworkCore::Tensor

# A 2-layer MLP, no bias: 1 input → 2 hidden ReLU → 1 output
w1 = T.new([[0.5, -0.3]]); w1.requires_grad = true
w2 = T.new([[0.4], [0.7]]); w2.requires_grad = true

# Synthetic data: regress y = 2x + 3 over 4 samples
x      = T.new([[0.0], [1.0], [2.0], [3.0]])
target = T.new([[3.0], [5.0], [7.0], [9.0]])

def sgd_step(p, lr)
  new_data = p.to_a.each_with_index.map { |v, i| v - lr * p.grad.to_a[i] }
  T.new(new_data, shape: p.shape).tap { |t| t.requires_grad = true }
end

30.times do
  pred = x.matmul(w1).relu.matmul(w2)
  loss = ((pred - target) * (pred - target)).mean
  loss.backward
  w1 = sgd_step(w1, 0.01)
  w2 = sgd_step(w2, 0.01)
end

# Loss drops 91% (36.5 → 3.2) in 30 SGD steps.
```

That whole snippet runs in pure Ruby today (no native ext required).
The test suite exercises this exact training loop in
`test/end_to_end_training_test.rb`.

## What's in the box

### `Tensor` (lib/.../tensor.rb)

```ruby
# Construction
T.new(nested_or_flat, shape: nil, dtype: :f32, requires_grad: false)

# Factories
T.zeros(2, 3)   T.ones(3)    T.full([2, 2], 7.5)
T.eye(3)        T.eye(2, 3)
T.arange(5)     T.arange(0, 10, 2)    T.arange(5, 0, -1)
T.randn(3, 4, seed: 42)
T.from_array([[1, 2], [3, 4]])
T.ones_like(t)  T.zeros_like(t)

# Shape ops
t.reshape(3, 4)   t.transpose   t.flatten   t.squeeze   t.unsqueeze(0)

# Operators (autograd-aware)
a + b   a - b   a * b   a / b   a**2   -a
a + 5   a * 2.0                                 # scalar broadcasts

# Named ops
a.matmul(b)
a.relu   a.sigmoid   a.tanh   a.gelu   a.softmax
a.sum    a.mean      a.abs

# Introspection
shape  dtype  ndim  numel  ==  eql?  hash  inspect  to_a  to_nested_a
requires_grad  grad  grad_fn
```

### Autograd (lib/.../autograd.rb)

```ruby
# Function base class (subclass to define new ops)
class MyOp < CodingAdventures::MLFrameworkCore::Function
  def forward(x)
    @saved_for_backward[:x] = x       # stash what backward needs
    # ... return a Tensor ...
  end

  def backward(grad)
    x = @saved_for_backward[:x]
    # ... return Array<Tensor, nil> ...
  end
end

# Tensor#backward — reverse-mode autodiff
loss.backward
loss.backward(custom_grad_tensor)     # explicit seed gradient
```

### Ops (lib/.../ops.rb)

| Op       | Rust dispatch?     | Notes                                    |
|----------|--------------------|------------------------------------------|
| Add/Sub  | ≥10k cells         | Elementwise                              |
| Mul/Div  | ≥10k cells         | Elementwise                              |
| Neg/Abs  | ≥10k cells         | Elementwise                              |
| Tanh     | ≥10k cells         | Elementwise activation                   |
| MatMul   | ≥10k cells         | 2-D only in v1.0                         |
| Sum/Mean | ≥10k cells         | Reduce-all (output shape `(1,)`)         |
| Pow      | pure Ruby          | Scalar exponent                          |
| ReLU     | pure Ruby          | Routes through Max+const in follow-up    |
| Sigmoid  | pure Ruby          | Multi-op graph in follow-up              |
| GELU     | pure Ruby          | Multi-op graph in follow-up              |
| Softmax  | pure Ruby          | Multi-op graph; numerically stable       |

## Installation

### From source (workspace)

```bash
cd code/packages/ruby/ml_framework_core
bundle install
bundle exec rake test
```

### From RubyGems (planned)

```bash
gem install coding_adventures_ml_framework_core
```

The gem itself is pure Ruby.  To enable Rust dispatch above the
10k-cell threshold, also install `coding_adventures_matrix_rust_ruby`
(which builds a native ext via `cargo`).

## Benchmark

```bash
cd code/packages/ruby/ml_framework_core
ruby -Ilib scripts/benchmark.rb
```

Example output (Apple M-series, Ruby 2.6, pure-Ruby fallback only):

```
| batch  | forward (ms) | backward (ms) | total (ms) | dispatch       |
|--------|--------------|---------------|------------|----------------|
|    100 |         0.15 |          0.23 |       0.38 | Ruby (no Rust) |
|   1000 |         1.23 |          2.20 |       3.44 | Ruby (no Rust) |
|   5000 |    (skipped) |     (skipped) |  (skipped) | Rust needed    |
|  10000 |    (skipped) |     (skipped) |  (skipped) | Rust needed    |
|  50000 |    (skipped) |     (skipped) |  (skipped) | Rust needed    |
```

Build the matrix_rust_ruby native ext (`cd ../matrix_rust_ruby &&
bundle exec rake compile`) to see the Rust-dispatch numbers.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  ml_framework_core (THIS GEM, v1.0.0)                            │
│    Tensor + autograd + 15 differentiable ops                     │
│    ↓ dispatch large tensors through ↓                            │
│  matrix_rust_ruby (Ruby gem)                                     │
│    MatrixRustRuby.run_graph_on_cpu(envelope_json)                │
│    ↓                                                              │
│  matrix_rust_ruby_native (Rust cdylib)                           │
│    ↓                                                              │
│  c-bridge (Rust workspace crate)                                 │
│    pure-Rust run_graph_on_cpu_via_json_envelope                  │
│    ↓                                                              │
│  matrix-ir-json → matrix-ir → matrix-runtime → matrix-cpu        │
└──────────────────────────────────────────────────────────────────┘
```

This stack — c-bridge → matrix_rust_ruby_native → matrix_rust_ruby →
ml_framework_core — is the **Ruby pilot** for a multi-language plan.
The same shape will be replicated for Lua, JS/TS, Go, and Swift in
future pilots: each language gets its own `<lang>-bridge` workspace
crate, then its own low-level binding, then its own idiomatic
ml_framework_core.

## Tests

148 tests, 251 assertions across 4 files:

```bash
ruby -Ilib -Itest test/tensor_test.rb               #  63 tests
ruby -Ilib -Itest test/autograd_test.rb             #  18 tests
ruby -Ilib -Itest test/ops_test.rb                  #  65 tests
ruby -Ilib -Itest test/end_to_end_training_test.rb  #   2 tests
```

Or all at once:

```bash
bundle exec rake test
```

## License

MIT.  See LICENSE in the repository root.
