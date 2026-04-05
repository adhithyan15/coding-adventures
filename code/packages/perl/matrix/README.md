# CodingAdventures::Matrix (Perl)

Pure-Perl 2D matrix type with arithmetic and linear-algebra operations.

## What it does

Provides a blessed object with the following interface:

### Base Operations

| Operation | Method | Notes |
|-----------|--------|-------|
| Create all-zeros | `zeros($rows, $cols)` | Class method |
| Create from 2D arrayref | `from_2d($data)` | Class method, deep-copies |
| Create from 1D arrayref | `from_1d($data)` | Class method, creates 1×n |
| Create from scalar | `from_scalar($val)` | Class method, creates 1×1 |
| Dimension accessors | `rows()`, `cols()` | |
| Element access | `get($i, $j)`, `set($i, $j, $val)` | Zero-based; set returns new matrix |
| Element-wise add | `add($B)` | Returns `($matrix, $err)` |
| Add scalar | `add_scalar($s)` | Returns matrix |
| Element-wise subtract | `subtract($B)` | Returns `($matrix, $err)` |
| Scale | `scale($s)` | Returns matrix |
| Transpose | `transpose()` | Returns matrix |
| Matrix multiply | `dot($B)` | Returns `($matrix, $err)` |

### Extension Operations (ML03)

| Operation | Method | Notes |
|-----------|--------|-------|
| Sum | `sum()` | Sum of all elements |
| Sum rows | `sum_rows()` | m×1 column vector |
| Sum cols | `sum_cols()` | 1×n row vector |
| Mean | `mean()` | Arithmetic mean |
| Min / Max | `mat_min()` / `mat_max()` | Prefixed to avoid built-in clash |
| Argmin / Argmax | `argmin()` / `argmax()` | (row, col) of extremum |
| Map | `mat_map($fn)` | Apply coderef to every element |
| Sqrt | `mat_sqrt()` | Element-wise square root |
| Abs | `mat_abs()` | Element-wise absolute value |
| Pow | `mat_pow($exp)` | Element-wise exponentiation |
| Flatten | `flatten()` | 1×n row vector |
| Reshape | `reshape($rows, $cols)` | Must preserve total elements |
| Row / Col | `mat_row($i)` / `mat_col($j)` | Extract single row/col |
| Slice | `slice($r0, $r1, $c0, $c1)` | Sub-matrix [r0..r1), [c0..c1) |
| Equals | `equals($B)` | Exact equality |
| Close | `close($B, $tol)` | Within tolerance (default 1e-9) |
| Identity | `identity($n)` | Class method, n×n identity |
| From diagonal | `from_diagonal($values)` | Class method, diagonal matrix |

## How it fits in the stack

This is the Perl mirror of `code/packages/lua/matrix` and `code/packages/elixir/matrix`. It is a pure-math leaf package with no dependencies outside core Perl.

## Usage

```perl
use CodingAdventures::Matrix;

# Build a 2×3 weight matrix.
my $W = CodingAdventures::Matrix->from_2d([
    [0.1, 0.2, 0.3],
    [0.4, 0.5, 0.6],
]);

# Build a 3×1 input column vector.
my $x = CodingAdventures::Matrix->from_1d([1.0, 2.0, 3.0])->transpose;

# Forward pass: W · x  →  2×1 result
my ($out, $err) = $W->dot($x);
die $err if $err;

printf "out[0] = %.1f\n", $out->get(0, 0);  # 1.4
printf "out[1] = %.1f\n", $out->get(1, 0);  # 3.2
```

## Running the tests

```
cpanm --installdeps .
prove -l -v t/
```
