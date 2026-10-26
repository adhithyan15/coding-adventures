# Matrix (Ruby)

A pure-Ruby 2D matrix library for linear algebra. No external dependencies --
only Ruby's built-in `Math` module.

## Usage

```ruby
require_relative "lib/matrix_ml"

# Create matrices
m = Matrix.new([[1.0, 2.0], [3.0, 4.0]])
z = Matrix.zeros(3, 3)
i = Matrix.identity(3)
d = Matrix.from_diagonal([2.0, 3.0])

# Arithmetic
a + b           # element-wise addition
a - b           # element-wise subtraction
a * 2.0         # scalar multiplication
a.dot(b)        # matrix multiplication
a.transpose     # transpose

# Element access
m.get(0, 1)          # read element -> 2.0
m.set(0, 1, 99.0)    # new matrix with element replaced

# Reductions
m.sum          # sum of all elements -> 10.0
m.mean         # arithmetic mean -> 2.5
m.sum_rows     # column vector of row sums
m.sum_cols     # row vector of column sums
m.min          # smallest element
m.max          # largest element
m.argmin       # [row, col] of minimum
m.argmax       # [row, col] of maximum

# Element-wise math
m.map_elements { |v| v * 2 }   # apply block to every element
m.sqrt         # element-wise square root
m.abs          # element-wise absolute value
m.pow(2.0)     # element-wise exponentiation

# Shape operations
m.flatten                   # 1 x n row vector
m.reshape(rows, cols)       # reshape (total must match)
m.row(i)                    # extract row i
m.col(j)                    # extract column j
m.slice(r0, r1, c0, c1)    # sub-matrix [r0..r1), [c0..c1)

# Equality
m.equals(other)                  # exact equality
m.close(other, tolerance = 1e-9)  # within tolerance
```

## Design Principles

1. **Non-mutating arithmetic** -- operations return new matrices. Nested input
   arrays and the public `data` reader are not defensive copies; callers needing
   owned/validated state must validate rectangular finite data and copy it first.
2. **No external dependencies** -- pure Ruby math only.
3. **Consistent error handling** -- descriptive ArgumentError/IndexError.
4. **Native Ruby ownership** -- independent Ruby arithmetic, not the optional
   Rust binding or Ruby standard-library `matrix` implementation.

## Install and validate

```bash
bundle install
bundle exec standardrb
bundle exec rake
gem build coding_adventures_matrix.gemspec
```

The gem is `coding_adventures_matrix` 0.2.0; its entry point remains
`require "matrix_ml"` and its existing top-level `Matrix` API is retained. Do not
load it alongside another library defining that same constant.

Rake runs all numerical/boundary tests with a 95% library line-coverage floor,
then builds and installs the real gem into a temporary gem home. A separate Ruby
process requires the installed copy, verifies its source is not the checkout,
and checks a hand-sized dot product. The receipt is:

```json
{"contract":"NN44","native_library":"ruby/matrix","dot":[[19.0,22.0],[43.0,50.0]],"neural_training":false}
```

Temporary gem/install artifacts are cleaned on success or error; the smoke task
performs no publication/network install. Runtime dependencies are empty; Bundler
installs declared development/test tools. `BUILD_windows` excludes only Standard's
host-native `Layout/EndOfLine` check because the repository requires LF.

This [NN44 prerequisite](../../../specs/NN44-ruby-matrix-build-foundation.md)
does not implement neural training, checkpoint exchange or a GPU backend. Those
remain separate items in the
[framework roadmap](../../../learning/machine-learning/FRAMEWORK-ROADMAP.md).
