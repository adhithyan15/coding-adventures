# NN27 Dynamic Autograd and Saved-Value Labs

Status: Draft

NN27 connects the NN26 tensor view to reverse-mode automatic differentiation.
The graph is not a model declared ahead of time. It is the record of operations
that actually ran. Each executed operation becomes a node, its input tensors
become parent edges, and a branch contributes only the operation selected at
runtime.

The corpus contains three bounded scalar graphs:

1. `(x * w + b)^2` with `x = 2`, `w = 3`, and `b = 1`;
2. `abs(x)^2` with `x = -2`, where the executed branch is `negate`; and
3. `x * w` followed by a live mutation from `w = 3` to `w = 100`.

The first graph is small enough to calculate completely by hand:

```text
m = 2 * 3 = 6
z = 6 + 1 = 7
loss = 7^2 = 49

dL/dz = 1 * (2 * saved(z)) = 14
dL/dm = 14 * 1 = 14       dL/db = 14 * 1 = 14
dL/dx = 14 * saved(w) = 42
dL/dw = 14 * saved(x) = 28
```

`multiply` saves immutable snapshots of both inputs. `square` saves its input.
`add`, `identity`, and `negate` have constant local derivatives and save
nothing. The mutation case proves why the snapshots matter: changing live `w`
to `100` after the forward pass does not change `d(x*w)/dx`, which remains the
saved forward value `3`.

Every case pins the executed node roster, actual operations, topological and
reverse-topological orders, forward values, branch choices, saved snapshots,
post-forward live inputs, local derivatives, parent contributions, analytical
leaf gradients, and central finite differences. The fixture epsilon is `1e-5`
and the canonical absolute tolerance is `1e-8`.

## Cross-Language and Rust-Core Direction

Language consumers should build this graph with their native objects and
identity semantics, then reproduce the fixture trace exactly. Dynamic control
flow belongs in the host: only operations that execute enter the graph, and
saved snapshots must not alias mutable user buffers.

A Rust core can accelerate bounded forward and backward kernels, but the first
C ABI should not expose host object pointers. Use validated operation tags,
opaque graph or buffer handles, explicit arity and lengths, caller-owned output
buffers, and status codes. The host retains graph ownership and branch control;
Rust receives finite scalar/tensor snapshots and returns values or local
gradient contributions. A later compilation tranche can capture a completed
forward graph, lower its operations to NeuralIR/MatrixIR, and synthesize a
backward graph with the same saved-value contract.
