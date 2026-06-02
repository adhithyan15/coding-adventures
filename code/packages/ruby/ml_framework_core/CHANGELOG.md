# Changelog

## Unreleased

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
