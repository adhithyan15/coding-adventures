# Changelog

## Unreleased

### Added — v1.0.0: complete Ruby ML framework on the Rust matrix-cpu engine

PR #8 of 8 (FINAL) in the Ruby pilot.  v1.0.0 marks the completion of
the multi-PR plan: a PyTorch-shaped Ruby ML framework with all 15 ops
(forward + backward), autograd engine, end-to-end MLP training, and a
benchmark script for performance characterization.

#### What v1.0.0 means

The complete stack works:

```ruby
require "coding_adventures/ml_framework_core"
T = CodingAdventures::MLFrameworkCore::Tensor

# Build a model
w1 = T.new([[0.5, -0.3]]); w1.requires_grad = true
w2 = T.new([[0.4], [0.7]]); w2.requires_grad = true

# Train (loss drops 91% in 30 SGD steps on the test suite's synthetic data)
30.times do
  pred = x.matmul(w1).relu.matmul(w2)
  loss = ((pred - target) * (pred - target)).mean
  loss.backward
  w1 = sgd_step(w1, lr: 0.01)
  w2 = sgd_step(w2, lr: 0.01)
end
```

All math is pure Ruby for tensors under 10k cells.  At 10k+ cells the
ops auto-dispatch through the matrix_rust_ruby gem to the Rust
matrix-cpu executor (SIMD-accelerated f32).  No Rust toolchain is
required to use the gem at small/medium sizes.

#### New in v1.0.0

- **`scripts/benchmark.rb`** — performance characterization.  Runs
  forward + backward on a 2-layer MLP at batch sizes 100, 1000, 5000,
  10_000, 50_000 and prints a markdown table of median timings.
  Gracefully handles the case where matrix_rust_ruby isn't built
  (skips Rust-only rows with a clear "Rust needed" label).

- **Gemspec polish**:
  - Tightened description to highlight what's in the box (15 ops, autograd, dispatch)
  - Added `homepage_uri`, `changelog_uri`, `bug_tracker_uri` to metadata

- **README polish**: Quick-start section with the end-to-end MLP
  example moved to the top; benchmark section with example output.

#### Layered architecture (now complete)

```
ml_framework_core (THIS GEM, v1.0.0)
  Tensor + autograd + 15 differentiable ops (forward + backward)
  ↓ dispatch large tensors through ↓
matrix_rust_ruby gem (v0.1)
  Ruby wrapper around matrix_rust_ruby_native
  ↓
matrix_rust_ruby_native (Rust cdylib, v0.1)
  Defines MatrixRustRuby.run_graph_on_cpu
  ↓
c-bridge (Rust workspace crate, v0.1)
  Pure-Rust run_graph_on_cpu_via_json_envelope
  ↓
matrix-ir-json → matrix-ir → matrix-runtime → matrix-cpu
```

#### Test coverage (unchanged from v0.4.0 — all still passing)

- `test/tensor_test.rb`            63 runs, 94 assertions
- `test/autograd_test.rb`          18 runs, 31 assertions
- `test/ops_test.rb`               65 runs, 122 assertions
- `test/end_to_end_training_test`  2 runs, 4 assertions
- Total: 148 tests, 251 assertions, 0 failures

#### What's next (for the project, not the gem)

The user will pick the next language pilot — Lua, JS/TS, Go, or Swift.
The architecture pattern established by the Ruby pilot transfers
directly: per-language Rust cdylib bridge → per-language low-level
binding → per-language idiomatic ml_framework_core.

### Added — v0.4.0: backward dispatch + end-to-end MLP training test

PR #7 of 8 in the Ruby pilot.  Adds `backward(output_grad)` to all 15
Function subclasses from v0.3.0, plus an end-to-end test that trains a
2-layer MLP and asserts loss decreases monotonically.  The full
PyTorch-shaped forward+backward+SGD loop now works in pure Ruby.

#### Backward formulas (mirroring `code/packages/python/ml-framework-core/src/ml_framework_core/autograd.py`)

| Op       | Saves in forward    | Backward formula                                  |
|----------|---------------------|---------------------------------------------------|
| Add      | (nothing)           | `[g, g]`                                          |
| Sub      | (nothing)           | `[g, -g]`                                         |
| Mul      | inputs `a`, `b`     | `[g * b, g * a]`                                  |
| Div      | inputs `a`, `b`     | `[g / b, -g * a / b²]`                            |
| Neg      | (nothing)           | `[-g]`                                            |
| Abs      | input `a`           | `[g * sign(a)]`  (sign(0) = 0, PyTorch convention)|
| Pow      | input `a`, scalar e | `[g * e * a^(e-1)]`  (scalar e gets no grad)      |
| MatMul   | inputs `A`, `B`     | `[g @ B^T, A^T @ g]`                              |
| ReLU     | input `a`           | `[g * (a > 0)]`                                   |
| Sigmoid  | output `y`          | `[g * y * (1 - y)]`                               |
| Tanh     | output `y`          | `[g * (1 - y²)]`                                  |
| GELU     | input `a`           | `[g * (0.5*(1+tanh_v) + 0.5*x*sech²*d_inner)]`    |
| Softmax  | output `y`          | `[y * (g - Σ(g*y))]`  per-row over last axis      |
| Sum      | input shape         | `[broadcast(g[0], input_shape)]`                  |
| Mean     | input shape, numel  | `[broadcast(g[0]/numel, input_shape)]`            |

All backward implementations are pure Ruby for v0.4.0.  Routing through
Rust (mirroring v0.3.0's forward dispatch) would require new envelope
shapes per backward op — deferred to a follow-up; the pure-Ruby
versions are correct and fast enough for the small-tensor case that
typifies autograd training (gradients are usually parameter-shaped,
which is far smaller than batch-shaped data).

#### Critical autograd fix: filter non-Tensor parents

PR #6 set `fn.parents = inputs.dup` unconditionally, which meant ops
like `Pow` (whose second input is a Numeric scalar) put a `Float` in
the parents Array.  `Tensor#backward`'s topological walk then crashed
trying to read `.grad_fn` on a non-Tensor.

Fixed in `Function.apply` by filtering parents to Tensors only:
`fn.parents = inputs.select { |i| i.is_a?(Tensor) }`.  Non-Tensor args
(like Pow's exponent) are still passed to `forward` but no longer
appear in the autograd graph, which is correct — autograd only
tracks Tensor inputs.  Subclasses can stash non-Tensor args in
`@saved_for_backward` to recover them in backward (Pow does this).

#### End-to-end MLP test

New file `test/end_to_end_training_test.rb` runs a real training loop:

```ruby
# 2-layer MLP, no bias: x → linear(W₁) → ReLU → linear(W₂) → MSE loss
30.times do
  pred = x.matmul(w1).relu.matmul(w2)
  loss = ((pred - target) * (pred - target)).mean
  loss.backward
  w1 = sgd_step(w1, lr: 0.01)
  w2 = sgd_step(w2, lr: 0.01)
end
assert final_loss < initial_loss * 0.25   # 75%+ drop
```

Synthetic dataset (4 samples, regress `y = 2x + 3`) — converges from
loss 36.5 → 3.2 in 30 steps (91% drop).  Companion test:
1-layer linear regression converges `w` from 0.5 to ≈2.0 in 20 steps.

#### Tests added (17 new minitests, plus 2 end-to-end)

**`BackwardCorrectnessTest`** (17 tests in test/ops_test.rb):
- One test per op for the simple analytical cases:
  Add, Sub, Mul, Div, Neg, Abs, Pow, MatMul, ReLU, Sigmoid, Tanh,
  Softmax (uniform input → uniform output → zero grad), Sum, Mean
- Numerical-gradient checking via finite differences for the
  transcendentals (GELU at x=1, Softmax at x=[1,2,3])
- Chained-op backward: x → Mul → Add → Sum → backward; assert
  x.grad propagates through the chain correctly

**`EndToEndTrainingTest`** (2 tests in new file):
- `test_two_layer_mlp_trains_loss_decreases`: MLP loss drops 75%+
- `test_linear_regression_converges`: w converges near true value

All existing tests still pass:
- `test/tensor_test.rb`: 63/63
- `test/autograd_test.rb`: 18/18
- `test/ops_test.rb`: 65/65 (47 from v0.3.0 + 17 new backward + 1 leftover)

Total: 148 tests, 251 assertions, 0 failures.

#### What's next

- PR #8: benchmark script + RubyGems publishing polish.  Bumps to v1.0.0
  — full Ruby ML framework on top of the Rust matrix-cpu engine.

### Added — v0.3.0: forward op dispatch (15 Function subclasses)

PR #6 of 8 in the Ruby pilot.  Adds the 15 differentiable operations on
top of the v0.2 autograd engine.  Every op is a `Function` subclass with
a `forward` method; the autograd graph builds automatically when any
input has `requires_grad`.  Backward implementations land in PR #7.

#### The 15 ops

| Op       | Class      | Rust dispatch?     | matrix-ir kind                   |
|----------|------------|--------------------|----------------------------------|
| Add      | `AddOp`    | yes (≥ 10k cells)  | `Add (lhs/rhs/output)`           |
| Sub      | `SubOp`    | yes                | `Sub`                            |
| Mul      | `MulOp`    | yes                | `Mul`                            |
| Div      | `DivOp`    | yes                | `Div`                            |
| Neg      | `NegOp`    | yes                | `Neg (input/output)`             |
| Abs      | `AbsOp`    | yes                | `Abs`                            |
| Tanh     | `TanhOp`   | yes                | `Tanh`                           |
| MatMul   | `MatMulOp` | yes (2-D only)     | `MatMul (a/b/output)`            |
| Sum      | `SumOp`    | yes (reduce-all)   | `ReduceSum (axes/keep_dims)`     |
| Mean     | `MeanOp`   | yes                | `ReduceMean`                     |
| Pow      | `PowOp`    | pure Ruby          | —  (Rust Pow takes 2 tensors)    |
| ReLU     | `ReLUOp`   | pure Ruby          | —  (Max + zero-tensor constant)  |
| Sigmoid  | `SigmoidOp`| pure Ruby          | —  (4-op multi-graph)            |
| GELU     | `GELUOp`   | pure Ruby          | —  (large multi-op graph)        |
| Softmax  | `SoftmaxOp`| pure Ruby          | —  (multi-op graph)              |

The "pure Ruby" five have working Rust paths in the Python reference;
left in pure Ruby for v0.3.0 to keep this PR focused.  Follow-ups can
lift them over individually.

#### Dispatch threshold

`Ops::DISPATCH_THRESHOLD = 10_000` cells.  Below it, every op uses
pure Ruby — fast at small sizes, avoids the JSON-build + hex-encode
+ FFI overhead.  Above it, the eligible ops dispatch into the Rust
executor via `MatrixRustRuby.run_graph_on_cpu` (matches the Python
reference's `_ELEMENTWISE_RUST_THRESHOLD`).

#### Lazy require for matrix_rust_ruby

`coding_adventures/matrix_rust_ruby` is `require`d LAZILY inside
`Ops.run_envelope` — only when we actually need to dispatch to Rust.
This keeps small-tensor / pure-Ruby workflows runnable even when the
matrix_rust_ruby gem's native ext isn't built (e.g. on CI machines
without a Rust toolchain, or during early dev).

#### Hex packing helpers

```ruby
Ops.pack_f32_hex([1.0, 2.5])              # → "0000803f00002040"
Ops.unpack_f32_hex("0000803f", 1)         # → [1.0]
```

Uses Ruby's `pack("e*")` (little-endian f32) + `unpack1("H*")` —
matches the Python reference's `struct.pack("<{n}f", ...)` byte-for-byte
so envelopes built here are bit-compatible with those built in Python.

#### Tensor extensions

```ruby
# Operator overloads now route through Function.apply
a + b   a - b   a * b   a / b   a**2   -a

# Scalar broadcasting (small-tensor only for v0.3.0)
a + 5   a * 5.0   ...

# Named op methods
a.matmul(b)
a.relu  a.sigmoid  a.tanh  a.gelu  a.softmax
a.sum   a.mean
a.abs
```

The original v0.1 element-wise overloads are REPLACED by autograd-aware
versions; numeric results are unchanged (Tensor#== is value-based), but
each call now builds a graph node if any input has `requires_grad`.

#### Architecture choices

- **Function.apply always, threshold in forward**: every op goes through
  `Function.apply` (so the autograd graph builds uniformly); each
  individual `forward` chooses Rust vs. Ruby internally based on the
  threshold.  Simpler than two-tier dispatch.
- **Pure-Ruby `_binary_elementwise` / `_unary_elementwise` helpers** on
  the MLFrameworkCore module: shared dispatch shape for the 4 binary
  + 3 unary ops, so each op subclass is a 3-line definition.
- **Envelope shapes mirror Python `_rust_backend.py` byte-for-byte**:
  same `"kind"` strings, same tensor/op/inputs/outputs JSON layout.
  Cross-language wire compatibility is the contract.

#### Tests added (47 minitests, 90 assertions, all passing)

- `HexHelpersTest` (4): pack/unpack round-trip + known-value spot checks
- `ForwardSmallTest` (20): every op's small-tensor (pure-Ruby) path —
  Add/Sub/Mul/Div/Neg/Abs/Pow/MatMul/ReLU/Sigmoid/Tanh/GELU/Softmax/Sum/Mean
  with numerical correctness assertions + edge cases (sigmoid at 0,
  tanh approaches 1, softmax sums to 1, softmax numerical stability
  with 1000-magnitude inputs, GELU at 1 ≈ 0.8413, matmul argument
  validation)
- `AutogradWiringTest` (5): every op produces a tensor with the right
  `grad_fn` class when input has `requires_grad`; no grad_fn when no
  input requires grad
- `OperatorOverloadsTest` (5): `+ - * / ** -` route through their Op
  classes; scalar broadcasting; unsupported operand raises
- `TensorNamedOpMethodsTest` (9): `t.relu`, `t.sigmoid`, etc. each
  dispatch correctly
- `DispatchPathBranchingTest` (2): threshold constant + small tensor
  doesn't trigger MatrixRustRuby require

Existing tests:
- `test/tensor_test.rb` — 63/63 pass (no regressions)
- `test/autograd_test.rb` — 18/18 pass (no regressions)

#### What's next

- PR #7: backward dispatch — implement `backward(output_grad)` on each
  of the 15 Function subclasses using the analytical gradient formulas
  from `code/packages/python/ml-framework-core/src/ml_framework_core/autograd.py`.
  End-to-end test: train a 2-layer MLP on a tiny synthetic dataset for
  N steps; assert loss decreases monotonically.
- PR #8: benchmark script + RubyGems publishing polish.

### Added — v0.2.0: autograd engine (Function.apply + Tensor#backward)

PR #5 of 8 in the Ruby pilot.  Adds reverse-mode automatic differentiation
on top of v0.1's Tensor class.  All math still pure Ruby; PR #6 layers on
the ~15 concrete Function subclasses that route large ops through Rust.

#### New API

```ruby
# Base class for every differentiable op
class CodingAdventures::MLFrameworkCore::Function
  # Subclasses override these two:
  def forward(*inputs)            # → Tensor
  def backward(output_grad)       # → Array<Tensor, nil>

  # Class method that wires up the autograd graph and runs forward:
  def self.apply(*inputs)         # → Tensor
end

# Built-in subclass for testing the machinery (PR #6 adds the real ops):
class Identity < Function
end

# Tensor extensions:
x.backward(grad = nil)            # kicks off backprop; mutates leaf .grad
Tensor.ones_like(t)               # shape-matching ones tensor
Tensor.zeros_like(t)              # shape-matching zeros tensor
```

#### How it works

`Function.apply(*inputs)`:

  1. Instantiates the Function (so it can hold state for backward).
  2. Calls `forward(*inputs)` — subclass-defined; returns Tensor.
  3. If any input has `requires_grad`, sets `output.requires_grad = true`
     and `output.grad_fn = function_instance`.
  4. Returns the output Tensor.

`Tensor#backward(grad = nil)`:

  1. Defaults `grad` to ones-like(self).
  2. DFS post-order to build the topological list of Tensors upstream
     through the `grad_fn` chain.
  3. Walks topo list in REVERSE; for each non-leaf, calls
     `grad_fn.backward(node_grad)` to get per-input grads, accumulates
     them into a per-parent `grad_map`.
  4. For each leaf with `requires_grad`, copies the accumulated grad
     into the leaf's public `.grad` slot (accumulating on top if a
     previous backward already wrote there — supports repeated
     backward without zero_grad between).

Algorithm is O(V + E) where V is the number of operations and E the
number of tensor edges.  Mirrors PyTorch's reference algorithm and the
Python reference at
`code/packages/python/ml-framework-core/src/ml_framework_core/autograd.py`.

#### Architecture choices

- **Tensor reopened in autograd.rb** rather than edited in tensor.rb.
  Keeps the v0.1 Tensor file storage-only; concentrates autograd-related
  additions in one reviewable file.
- **`grad_map` keyed on `object_id`**, so the same Tensor reaching a
  Function via two paths (e.g. `Add.apply(x, x)`) gets its gradients
  summed correctly.
- **Identity subclass** ships for testing the machinery without
  introducing op-math; the real ops land in PR #6.

#### Tests added (18 minitests, 31 assertions, all passing)

- `AutogradApplyTest` (5):
  - requires_grad propagates through Identity
  - no grad_fn when no input requires grad
  - apply() returns a NEW tensor (object identity ≠ value equality)
  - Function subclass must implement forward (NotImplementedError)
  - Function subclass must implement backward (NotImplementedError)
- `TensorBackwardSanityTest` (3):
  - backward on non-grad tensor raises
  - backward with mismatched grad shape raises
  - backward returns nil
- `AutogradEndToEndTest` (6):
  - identity backward writes ones to leaf grad
  - identity backward with explicit seed grad
  - backward twice accumulates into leaf grad
  - chain of identities propagates
  - shared parent accumulates from both paths (x → Id → a; x → Id → b)
  - leaf node without requires_grad is skipped
- `AutogradFunctionIntrospectionTest` (2):
  - Function#inspect shows class name + parent count
  - Function#initialize has empty parents + saved_for_backward
- `TensorHelperFactoriesTest` (2):
  - ones_like / zeros_like

Existing 63 Tensor tests continue to pass — no regressions.

#### What's next

- PR #6: `ops.rb` — 15+ Function subclasses (Add, Sub, Mul, Div, Neg,
  Abs, Pow, MatMul, ReLU, Sigmoid, Tanh, GELU, Softmax, Sum, Mean).
  Each `forward` builds a matrix-ir-json envelope matching the Python
  `_rust_backend.py` shapes and dispatches large tensors through
  `MatrixRustRuby.run_graph_on_cpu`.
- PR #7: backward dispatch on every op subclass + end-to-end MLP test.
- PR #8: benchmark + RubyGems polish.

### Added — v0.1.0: pure-Ruby Tensor class

First release.  Ships the bottom layer of the Ruby ML framework stack: a
PyTorch-shaped `Tensor` class implemented entirely in Ruby.  No Rust
calls, no native ext.  Layered design — PRs #5-#8 will add the autograd
engine, forward + backward op dispatch, and benchmarks on top.

#### Public API

```ruby
require "coding_adventures/ml_framework_core"

T = CodingAdventures::MLFrameworkCore::Tensor
```

- **Construction**: `T.new(data, shape: nil, dtype: :f32, requires_grad: false)`
- **Factories**: `zeros`, `ones`, `full`, `eye` (square or rectangular),
  `arange` (1/2/3-arg, supports negative step), `randn` (deterministic
  with `seed:`, via Box-Muller), `from_array`
- **Shape ops**: `reshape`, `transpose` (2-D only in v0.1), `flatten`,
  `squeeze` (all-1 or specific axis), `unsqueeze` (negative axes OK)
- **Operator overloads**: `+`, `-`, `*`, `/`, `**`, unary `-` — tensor⊗tensor
  (same shape) and tensor⊗scalar
- **Conversions**: `to_a` (flat copy), `to_nested_a` (shape-respecting)
- **Introspection**: `shape`, `dtype`, `ndim`, `numel`, `==`, `eql?`,
  `hash`, `inspect`
- **Autograd-prep slots**: `requires_grad`, `grad`, `grad_fn` — present
  as storage for PR #5 to wire up; no autograd logic in this PR

#### Architecture choices

- **f64 in memory, :f32 dtype**: Ruby's `Float` is f64; using it as the
  in-memory representation avoids per-op packing.  The lossy f32
  conversion happens at the Rust dispatch boundary (PR #6).
- **Same-shape only for binary ops**: no NumPy-style broadcasting in
  v0.1.  Adding it now would couple Tensor to the shape-broadcasting
  algorithm; we pull it in when ops dispatch lands.
- **No indexing or slicing**: PyTorch's `__getitem__` is 50+ lines of
  arg-shape handling.  Deferred to after PR #8.
- **2-D-only transpose**: higher-rank transpose needs generic strided
  index math; we don't need it for any PR #5-#7 use case.
- **Pure-Ruby `randn` via Box-Muller**: two textbook lines of trig;
  avoids pulling in a distribution library.

#### File layout

```
ml_framework_core/
├── coding_adventures_ml_framework_core.gemspec
├── Gemfile                                       # gemspec + path override
├── Rakefile                                      # minitest task
├── lib/coding_adventures/
│   ├── ml_framework_core.rb                      # entry point
│   └── ml_framework_core/
│       ├── version.rb
│       └── tensor.rb                             # the Tensor class
├── test/tensor_test.rb                           # ~50 minitests, 6 sections
├── BUILD / BUILD_windows                         # bundle install + rake test
├── required_capabilities.json                    # empty (no FFI/net/fs)
├── README.md
└── CHANGELOG.md
```

#### Test coverage (six minitest sections, ~50 tests)

1. **Construction**: flat, nested (auto-shape), scalar, deep-nested,
   ragged-rejection, explicit-shape length validation, dtype validation,
   `requires_grad`/`grad` defaults.
2. **Factories**: every factory exercised, including arange edge cases
   (zero step → raise, negative step, wrong arity → raise) and randn
   determinism + mean sanity check on 1000 samples.
3. **Shape ops**: reshape (including back-round-trip), flatten,
   transpose (default, explicit perm, double = identity, invalid perm
   rejection, higher-rank NotImplementedError), squeeze (all dims,
   specific axis, negative axis, non-unit axis rejection), unsqueeze
   (positive, end, negative, round-trip).
4. **Arithmetic**: every op (+, -, *, /, **, unary -), tensor⊗tensor and
   tensor⊗scalar paths, shape-mismatch raise, unsupported-operand-type raise.
5. **Equality + inspect**: shape-aware ==, hash equality, inspect shape
   and dtype, inspect truncation for long data.
6. **Round-trip**: `to_nested_a` round-trips 2-D and 3-D, reshape
   preserves `to_a`.

#### Runtime dependency

- `coding_adventures_matrix_rust_ruby >= 0.1` — declared in the gemspec
  so the dependency graph is correct from day one, even though v0.1
  doesn't call it.  PR #6 wires it up.

#### What's next

- PR #5: `autograd.rb` — `Function` base class, `apply`, topological
  sort, `Tensor#backward`.  Mirrors `code/packages/python/ml-framework-core/src/ml_framework_core/autograd.py`.
- PR #6: `ops.rb` — 15+ Function subclasses (Add, Sub, Mul, Div, Neg,
  Abs, Pow, MatMul, ReLU, Sigmoid, Tanh, GELU, Softmax, Sum, Mean).
  Each dispatches large tensors through `MatrixRustRuby.run_graph_on_cpu`.
- PR #7: backward dispatch + end-to-end MLP training test.
- PR #8: benchmark script + RubyGems publishing polish.
