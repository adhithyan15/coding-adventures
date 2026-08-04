# CPU, Rust Core, and Accelerated Backends, by Hand

A neural network is a recipe. A backend is the kitchen that follows it.
Changing kitchens should change how the work is scheduled, not what meal comes
out.

This lesson keeps one dense layer tiny enough to calculate on paper, then runs
the same meaning through a scalar CPU interpreter, a TypeScript matrix engine,
the Rust matrix core, and an optional browser accelerator.

## The Whole Model

We have three examples, one input feature, one weight, and one bias per row:

```text
X = [[1], [2], [3]]   W = [[2]]   B = [[1], [1], [1]]
```

The layer is:

```text
y = XW + B
```

Because the shapes are `3 x 1`, `1 x 1`, and `3 x 1`, the multiplication is
valid and the result has shape `3 x 1`.

## One Row by Hand

Take the middle row, where `x = 2`:

```text
product = x * w = 2 * 2 = 4
output  = product + bias = 4 + 1 = 5
```

Do the same for all rows:

| Row | `x` | `x * 2` | `x * 2 + 1` |
| ---: | ---: | ---: | ---: |
| 0 | 1 | 2 | 3 |
| 1 | 2 | 4 | 5 |
| 2 | 3 | 6 | 7 |

So every honest backend must return `[3, 5, 7]`.

## Lane 1: Scalar CPU

The scalar interpreter handles one row at a time:

```text
load x -> multiply by w -> add b -> store y
```

This path is deliberately plain. It is the easiest place to inspect an
individual instruction and therefore the reference for model meaning.

## Lane 2: TypeScript Matrix CPU

The matrix path keeps all three rows in one column:

```text
load x column -> scale the column by 2 -> add the bias column -> store y column
```

The schedule changed, but the row arithmetic did not. Grouping independent
rows is **vectorization**: one operation describes work over several values.

## Lane 3: Rust Matrix CPU

The Rust core receives a language-neutral MatrixIR graph:

```text
input X [3 x 1, f32]
constant W [1 x 1, f32]
product = MatMul(X, W)
constant B [3 x 1, f32]
output = Add(product, B)
```

Inputs and outputs cross the language boundary as little-endian `f32` bytes.
For example, `1.0` is `0000803f` in the fixture's lowercase-hex notation.
The native test decodes the graph, runs `matrix-cpu`, and requires the output
bytes to equal the checked-in encoding of `[3, 5, 7]`.

The browser does not load that Node native addon. It shows the result proven by
the native fixture test. That distinction matters: displaying Rust's expected
answer is not the same as executing Rust in a browser.

## Lane 4: WebGPU

When WebGPU is available, the async backend uploads `X` and performs the scale
and add on device buffers. It downloads `y` for the output, then downloads
`x`, `bias`, and `y` again because the educational runner returns a value trace:

```text
host X -> device X, product, bias, y -> host output y + traced x, bias, y
```

Those trace reads make the computation inspectable, but they are extra
transfers. A later residency experiment can measure and remove them.

The interactive probe has three honest outcomes:

- **executed:** a WebGPU adapter ran the plan and reported its output and error;
- **unavailable:** this browser exposes no usable WebGPU adapter;
- **failed:** WebGPU was present, but setup or execution returned an error.

An unavailable accelerator is not a failed lesson. CPU fallback is part of a
portable runtime contract.

## Why Precision Appears in the Contract

JavaScript numbers use binary64. The Rust and WebGPU lanes use binary32
(`f32`). Many decimal values round differently in those formats, so parity is
normally measured with an explicit tolerance.

This first example uses small integers exactly representable in both formats.
Its tolerance is still declared as `1e-6` so every consumer implements the same
comparison rule before the later precision experiments add harder numbers.

## Correctness Is Not Performance

Equal outputs prove semantic parity. They do not prove speed.

For a three-value column, upload and launch overhead can make a GPU slower than
a CPU loop. Acceleration becomes useful when enough work stays resident on the
device to repay those fixed costs. That is why the visualizer exposes buffer
residency next to arithmetic.

## The Portable Boundary

The complete oracle lives in
[`backend-parity-v1`](../../specs/fixtures/backend-parity-v1/README.md). A new
language can join in two steps:

1. Parse the fixture and reproduce `[3, 5, 7]` with a small native reference.
2. Send the same MatrixIR and byte buffers through a Rust-core binding and
   compare the result with the same oracle.

Node, Python, and Ruby bindings already demonstrate the second shape. A stable
Rust C ABI for more language families is still future work; this lesson does
not pretend that boundary exists yet.

## What to Remember

1. The graph owns meaning; a backend owns scheduling, storage, and kernels.
2. Parity requires actual outputs and a declared precision tolerance.
3. A hardware label is not proof that hardware executed.
4. Buffer movement can dominate tiny accelerated workloads.
5. One language-neutral graph and byte oracle can test native ports and Rust
   bindings without making machine learning belong to one host language.

Next, precision, quantization, and residency experiments will deliberately
change number formats and transfer schedules while keeping this parity habit.
