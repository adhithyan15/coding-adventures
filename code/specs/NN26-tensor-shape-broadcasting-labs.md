# NN26 Tensor Shape and Broadcasting Labs

Status: Draft

NN26 is the first bridge from scalar neural-network arithmetic to tensors and
autograd. In this contract, a tensor is a finite row-major buffer plus a shape.
Elementwise addition aligns shapes from the right. Two aligned dimensions are
compatible when they are equal or either dimension is `1`; the result uses the
larger dimension. Missing leading dimensions behave as dimensions of size `1`.

The corpus includes four deliberately small cases:

1. `[2, 1] + [1, 3] -> [2, 3]`, where both inputs expand;
2. `[2, 3] + [3] -> [2, 3]`, where right alignment pads the vector to
   `[1, 3]`;
3. `[] + [2, 2] -> [2, 2]`, where one scalar reaches every cell; and
4. `[2, 3] + [2]`, which is rejected because aligned dimensions `3` and `2`
   are neither equal nor `1`.

Every compatible case records the padded shapes, expanded axes, row-major
output buffer, and one mapping for every output coordinate. A mapping names the
exact left and right coordinates reused by that output cell.

The reverse rule makes the connection to autograd without introducing a
dynamic graph yet. If a forward input coordinate was reused, all upstream
gradients from those output cells are summed back into that coordinate:

```text
output[i, j] = left[i, 0] + right[0, j]
dscore/dleft[i, 0] = sum over j of dscore/doutput[i, j]
dscore/dright[0, j] = sum over i of dscore/doutput[i, j]
```

Central finite differences independently check every compatible input value.
The fixture uses epsilon `1e-5` and absolute tolerance `1e-8`.

## Cross-Language and Rust-Core Direction

Direct language consumers should first reproduce shape inference, every
row-major index mapping, mismatch rejection, and reduction-to-input-shape from
the fixture. A shared Rust core can then expose two bounded operations through a
stable C ABI and WASM surface:

1. infer or validate the output shape from explicit rank and dimension arrays;
2. execute broadcast add and optionally return a trace or reduce an upstream
   buffer back to either input shape.

The ABI should use integer dimensions with checked element-count arithmetic,
explicit buffer lengths, caller-owned outputs, a declared row-major layout, and
status codes instead of panics. Hosts retain their idiomatic tensor objects and
autograd graphs while delegating the hot index walk to Rust. MatrixIR V1 forbids
implicit broadcasting, so a later compiler bridge should lower this contract
to explicit `BroadcastTo` or equivalent shape materialization followed by
elementwise `Add` and explicit reduction axes in the backward graph.
