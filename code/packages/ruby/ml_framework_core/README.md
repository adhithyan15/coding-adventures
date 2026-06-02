# ml_framework_core — idiomatic Ruby Tensor + autograd

A small, PyTorch-shaped Ruby ML library that runs the heavy stuff on the
Rust `matrix-cpu` engine and falls back to pure Ruby for small or
debugging workloads.

```ruby
require "coding_adventures/ml_framework_core"
include CodingAdventures::MLFrameworkCore       # or use MLFrameworkCore alias

x = Tensor.new([[1, 2, 3], [4, 5, 6]])
y = Tensor.eye(3)

x.matmul(y) == x                # forthcoming (PR #6)
(x + 1.0).to_nested_a           # works today
```

## Where this v0.1 sits in the multi-language stack

```text
┌─────────────────────────────────────────────────────────────────┐
│  ml_framework_core (this gem)        ← v0.1 = Tensor only       │
│    Tensor + factories + shape ops + operator overloads          │
│    PR #5: autograd engine                                       │
│    PR #6: forward op dispatch via matrix_rust_ruby              │
│    PR #7: backward dispatch + end-to-end MLP test               │
│    PR #8: benchmark + RubyGems publishing polish                │
│    ↓                                                             │
│  matrix_rust_ruby (Ruby gem)               ← PR #3 (merged)     │
│    MatrixRustRuby.run_graph_on_cpu(envelope)                    │
│    ↓                                                             │
│  matrix_rust_ruby_native (Rust cdylib)     ← PR #2 (merged)     │
│    ↓                                                             │
│  c-bridge (Rust workspace crate)           ← PR #1 (merged)     │
│    ↓                                                             │
│  matrix-cpu execution engine                                    │
└─────────────────────────────────────────────────────────────────┘
```

## What v0.1 ships

This PR (#4 of 8 in the Ruby pilot) is intentionally pure Ruby — no
native ext, no Rust calls.  That keeps it small, reviewable, and
independently testable.

### `Tensor` class

```ruby
T = CodingAdventures::MLFrameworkCore::Tensor

# Construction
T.new([1, 2, 3])                        # 1-D from flat
T.new([[1, 2], [3, 4]])                 # 2-D from nested (shape inferred)
T.new([1, 2, 3, 4], shape: [2, 2])      # flat + explicit shape

# Factories
T.zeros(2, 3)
T.ones(3)
T.full([2, 2], 7.5)
T.eye(3)                                # 3x3 identity
T.eye(2, 3)                             # rectangular
T.arange(5)                             # 0, 1, 2, 3, 4
T.arange(2, 10, 2)                      # 2, 4, 6, 8
T.randn(3, 4, seed: 42)                 # standard-normal via Box-Muller
T.from_array([[1, 2], [3, 4]])          # alias for new(nested)

# Shape ops
t.reshape(3, 4)
t.transpose                             # 2-D only in v0.1
t.flatten
t.squeeze(axis = nil)
t.unsqueeze(axis)

# Operator overloads — element-wise, same shape only in v0.1
a + b   a - b   a * b   a / b   a**2   -a
a + 5   a - 5   a * 5   a / 5   a**2          # scalar broadcasts

# Conversions + introspection
t.shape          # => [2, 3]
t.dtype          # => :f32
t.ndim           # => 2
t.numel          # => 6
t.to_a           # flat Array<Float>
t.to_nested_a    # nested Array matching shape

# Autograd-prep slots (PR #5 wires them up; here they're just storage)
t.requires_grad        # => false by default
t.requires_grad = true
t.grad                 # => nil until backward() runs
t.grad_fn              # => nil for leaf tensors
```

## What's intentionally NOT in v0.1

| Feature           | Lands in | Why deferred                         |
|-------------------|----------|--------------------------------------|
| Broadcasting      | PR #6    | Couples Tensor to shape algebra      |
| Indexing/slicing  | post-#8  | 50+ lines of arg-shape handling      |
| `sum` / `mean`    | PR #6    | These will be Function subclasses    |
| Autograd `apply`  | PR #5    | Needs its own focused PR             |
| Rust dispatch     | PR #6    | Needs autograd graph first           |
| Higher-rank `transpose` | post-#8 | Strided index math not needed yet |

## Storage model

- `@data` is a flat `Array<Float>` (Ruby Floats are f64).
- `@shape` is an `Array<Integer>`.
- `@dtype` is `:f32` (matches matrix-cpu; in-memory f64 just happens to be
  Ruby's only float primitive).

The lossy f64→f32 conversion only happens at the Rust dispatch boundary
(PR #6), where we pack each f64 into 4 bytes of little-endian f32 before
hex-encoding for the JSON envelope.

## Running the test suite

```bash
cd code/packages/ruby/ml_framework_core
bundle install
bundle exec rake test
```

The suite has ~50 tests covering:

- Construction (flat, nested, scalar, explicit shape, ragged-rejection)
- Every factory (zeros, ones, full, eye, arange, randn, from_array)
- Every shape op (reshape, transpose, flatten, squeeze, unsqueeze)
- Every operator overload (+, -, *, /, **, unary -, scalar broadcast)
- Equality, hash, inspect
- Round-trip properties (`reshape(t.shape) == t`,
  `unsqueeze(0).squeeze(0) == t`, `transpose.transpose == t`,
  `to_nested_a → new → to_nested_a` is identity)
- Version constant + `MLFrameworkCore` short-alias
