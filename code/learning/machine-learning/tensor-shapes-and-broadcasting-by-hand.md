# Tensor Shapes and Broadcasting, by Hand

Neural-network formulas often jump from one number to whole batches of numbers:

```text
preactivation = input @ weight + bias
```

The `+ bias` looks like ordinary addition, but a batch may have many rows while
the bias has only one value per output feature. A tensor library uses
**broadcasting** to decide which bias value belongs to each batch row.

This lesson makes that decision visible. We will use addition because it keeps
the arithmetic tiny. The same index rule applies to elementwise subtraction,
multiplication, division, comparisons, and many activation-support operations.

## 1. A tensor is a buffer plus a shape

For this lesson, a **tensor** is simply a rectangular block of numbers. Its
shape says how many positions exist along each axis.

```text
shape [2, 3]

[[11, 21, 31],
 [12, 22, 32]]
```

The shape has two dimensions, so its **rank** is two. Stored in row-major order,
the same tensor is one flat buffer:

```text
[11, 21, 31, 12, 22, 32]
```

The shape restores the rows and columns. It is not decorative metadata: it is
what lets the library turn coordinate `[1, 2]` into flat buffer slot `5`.

A scalar has shape `[]` and rank zero. A vector such as `[10, 20, 30]` has
shape `[3]` and rank one. A batch of two such rows has shape `[2, 3]` and rank
two.

## 2. The broadcasting rule

To combine two tensors element by element:

1. line their shapes up on the right;
2. compare one aligned dimension at a time;
3. accept the axis when the dimensions are equal or either one is `1`;
4. use the larger dimension for the output; and
5. reject the operation when neither dimension is `1` and they differ.

Missing leading dimensions behave like dimensions of size one. Therefore a
shape `[3]` lines up with `[2, 3]` as `[1, 3]`:

```text
matrix  [2, 3]
vector  [1, 3]  <- one leading dimension is implied
result  [2, 3]
```

The vector's single row is reused for both matrix rows.

## 3. The complete hand example

Let the left tensor be a column:

```text
left shape [2, 1]

[[1],
 [2]]
```

Let the right tensor be a row:

```text
right shape [1, 3]

[[10, 20, 30]]
```

Align the shapes:

```text
axis                 0       1
left dimension       2       1
right dimension      1       3
output dimension     2       3
```

On axis 0, the right tensor's dimension `1` expands to `2`. On axis 1, the
left tensor's dimension `1` expands to `3`. The output shape is `[2, 3]`.

Now compute every cell:

```text
output[0, 0] = left[0, 0] + right[0, 0] = 1 + 10 = 11
output[0, 1] = left[0, 0] + right[0, 1] = 1 + 20 = 21
output[0, 2] = left[0, 0] + right[0, 2] = 1 + 30 = 31

output[1, 0] = left[1, 0] + right[0, 0] = 2 + 10 = 12
output[1, 1] = left[1, 0] + right[0, 1] = 2 + 20 = 22
output[1, 2] = left[1, 0] + right[0, 2] = 2 + 30 = 32
```

So:

```text
[[1],      [[10, 20, 30]]      [[11, 21, 31],
 [2]]   +                    =   [12, 22, 32]]
```

Nothing magical was copied into a new parameter. Each output coordinate maps
back to one existing left coordinate and one existing right coordinate.

## 4. One index under the microscope

Open output coordinate `[1, 1]`.

```text
output index       [1, 1]
left shape         [2, 1]
left axis 1 is 1, so its coordinate is forced to 0
left source        [1, 0] -> 2

right shape        [1, 3]
right axis 0 is 1, so its coordinate is forced to 0
right source       [0, 1] -> 20

output[1, 1] = 2 + 20 = 22
```

That “force an expanded axis to coordinate zero” rule is enough to implement
broadcasting without first materializing repeated copies.

## 5. Backward is the reverse of reuse

Suppose a later computation sends this upstream gradient into the output:

```text
upstream =
[[1, 2, 3],
 [4, 5, 6]]
```

Addition sends each output gradient unchanged to both of its source values.
But source values were reused. Their contributions must be added together.

The first left value supplied the whole first output row:

```text
dscore/dleft[0, 0] = 1 + 2 + 3 = 6
```

The second supplied the whole second row:

```text
dscore/dleft[1, 0] = 4 + 5 + 6 = 15
```

Therefore:

```text
left gradient shape [2, 1]
[[ 6],
 [15]]
```

The first right value supplied the first output column:

```text
dscore/dright[0, 0] = 1 + 4 = 5
dscore/dright[0, 1] = 2 + 5 = 7
dscore/dright[0, 2] = 3 + 6 = 9
```

Therefore:

```text
right gradient shape [1, 3]
[[5, 7, 9]]
```

This is often called **reducing to the input shape**. The rule is easier to
remember in plain language:

> Forward reuses coordinates. Backward sums all gradients that return to each
> reused coordinate.

## 6. Check the gradient independently

Define one scalar score by multiplying each output cell by its upstream value
and adding:

```text
score = sum(upstream * output)
```

To check `dscore/dleft[0, 0]`, perturb that one value by epsilon `1e-5`:

```text
numerical gradient
  = (score(left + epsilon) - score(left - epsilon)) / (2 * epsilon)
  = 5.999999996220139

analytical gradient = 6
absolute error       < 4e-9
```

The fixture repeats this central finite difference for all five input values.

## 7. Scalar and leading-rank cases

A scalar has shape `[]`. When added to a `[2, 2]` matrix, it behaves as padded
shape `[1, 1]`, so the scalar coordinate `[]` supplies every output cell.

```text
2 + [[1, 2],       [[3, 4],
     [3, 4]]   =    [5, 6]]
```

A rank-one shape `[3]` added to `[2, 3]` is first viewed as `[1, 3]`. This is
why a neural-network bias with one number per feature can be added across a
whole batch.

## 8. A mismatch should stop early

Try `[2, 3] + [2]`. Right alignment gives:

```text
left   [2, 3]
right  [1, 2]
```

The trailing dimensions are `3` and `2`. They differ, and neither is `1`, so
the operation is invalid. A library should reject it before walking either
buffer. Silent recycling would hide a model bug and can mean different things
in different languages.

## 9. From this lesson to autograd

The next bridge will build a dynamic computation graph. Broadcasting gives it
two facts to save for an elementwise operation:

1. the original input shapes; and
2. which axes expanded.

During backward, the graph uses those facts to reduce the output-shaped
gradient back to each input's exact shape. Later, gradient accumulation adds
another kind of sum: contributions from distinct graph paths. Keeping these
two ideas separate now makes that later behavior much easier to inspect.

## 10. Cross-language and Rust-core path

Every language implementation should reproduce the
[`tensor-broadcasting-v1`](../../specs/fixtures/tensor-broadcasting-v1/README.md)
fixture before optimizing. The common contract is:

- explicit ranks and integer dimension arrays;
- row-major buffers and checked element counts;
- right-aligned inference with deterministic mismatch errors;
- output-to-input index maps for trace mode;
- reverse reduction into caller-owned input-gradient buffers; and
- central finite-difference parity.

A Rust core can own the bounded shape inference and hot index walk behind a C
ABI or WASM export. Python, Ruby, JavaScript, Go, Java, and other hosts can keep
their natural tensor and autograd APIs while calling the same kernel. An
optimized kernel may avoid materializing expanded copies, but trace mode should
still expose the logical source coordinate for every output cell.

Continue with the interactive Tensor + Autograd workbench in the
[ML Learning Visualizer](../../programs/typescript/ml-learning-visualizer/README.md),
then move to the next roadmap lesson: saved values in a dynamic computation
graph.
