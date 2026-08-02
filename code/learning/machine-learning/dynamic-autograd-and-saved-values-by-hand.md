# Dynamic Autograd and Saved Values, by Hand

Broadcasting showed how one tensor operation moves values forward and gradients
backward. A useful neural network needs many operations connected together. An
**autograd** system records those connections while the program runs, then
reverses them when you ask for a gradient.

The name is a compact description:

- **auto** means automatic;
- **grad** is short for gradient; and
- a **graph** is a collection of nodes connected by edges.

There is no magic derivative oracle. Each operation knows one small local
derivative. The graph remembers how those local rules connect.

## 1. Build the smallest complete graph

Use three input values:

```text
x = 2
w = 3
b = 1
```

Run this expression:

```text
loss = (x * w + b)^2
```

Give every intermediate result a name:

```text
m = x * w = 2 * 3 = 6
z = m + b = 6 + 1 = 7
loss = z^2 = 7^2 = 49
```

The executed graph is:

```text
x = 2 ─┐
       ├─ multiply ─ m = 6 ─┐
w = 3 ─┘                    ├─ add ─ z = 7 ─ square ─ loss = 49
b = 1 ──────────────────────┘
```

The inputs are **leaf nodes**: the graph has no earlier operation that produced
them. `multiply`, `add`, and `square` are operation nodes. Edges point from an
operation to the values it consumed.

## 2. The graph is dynamic

**Dynamic** means the graph records what actually happened during this run.
Imagine computing an absolute value with ordinary control flow:

```text
if x >= 0:
    y = identity(x)
else:
    y = negate(x)
loss = square(y)
```

For `x = -2`, only `negate` executes:

```text
x = -2 -> negate -> y = 2 -> square -> loss = 4
```

The unused `identity` branch is source code, not an executed graph node. With a
positive input, the opposite graph would be built. This is why eager autograd
can follow loops, conditionals, recursion, and ordinary host-language code.

## 3. Topological order prevents time travel

A **topological order** lists every node after its parents. One valid order for
the first graph is:

```text
x, w, m, b, z, loss
```

`m` appears after `x` and `w`; `z` appears after `m` and `b`; `loss` appears
after `z`. A depth-first walk can construct this order even when the forward
program created independent leaves in a different sequence.

Backward traverses the order in reverse:

```text
loss, z, b, m, w, x
```

Children are therefore processed before the parents that need their gradient
contributions.

## 4. Save only what the local rule needs

The backward formula for multiplication needs both forward inputs:

```text
m = x * w
dm/dx = w
dm/dw = x
```

The backward formula for a square needs its forward input:

```text
loss = z^2
dLoss/dz = 2 * z
```

Those operations save immutable snapshots during forward:

| Operation | Saved values | Why backward needs them |
| --- | --- | --- |
| `multiply(x, w)` | `x = 2`, `w = 3` | each input is the other input's derivative |
| `add(m, b)` | nothing | both local derivatives are always `1` |
| `square(z)` | `z = 7` | the local derivative is `2z` |

“Saved” does not mean “keep a pointer to whatever the user may later mutate.”
It means preserve the forward-time value required by the derivative contract.

## 5. Reverse the graph by hand

Seed the output gradient with one:

```text
dLoss/dLoss = 1
```

### Reverse `square`

Read saved `z = 7`:

```text
local derivative = 2 * saved(z) = 14
dLoss/dz = upstream * local = 1 * 14 = 14
```

### Reverse `add`

Addition has two constant local derivatives:

```text
dz/dm = 1
dz/db = 1

dLoss/dm = 14 * 1 = 14
dLoss/db = 14 * 1 = 14
```

### Reverse `multiply`

Read the saved forward inputs:

```text
dm/dx = saved(w) = 3
dm/dw = saved(x) = 2

dLoss/dx = 14 * 3 = 42
dLoss/dw = 14 * 2 = 28
```

The final leaf gradients are:

```text
dLoss/dx = 42
dLoss/dw = 28
dLoss/db = 14
```

Each backward step has the same shape:

```text
parent contribution = upstream gradient * local derivative
```

## 6. Why a saved snapshot must be immutable

Consider only:

```text
x = 2
w = 3
product = x * w = 6
```

Forward saves `x = 2` and `w = 3`. Now imagine user code changes the live
weight before backward:

```text
w = 100
```

Backward must still differentiate the computation that produced `6`:

```text
dProduct/dx = saved(w) = 3
dProduct/dw = saved(x) = 2
```

Using the live `100` would answer a different question about a computation
that never happened. Production frameworks often detect unsafe in-place
mutation with version counters; the teaching fixture makes the simpler
snapshot contract explicit.

## 7. Check the answer without trusting backward

For one input `p`, central finite differences use two fresh forward runs:

```text
numerical gradient ≈ [f(p + epsilon) - f(p - epsilon)] / (2 * epsilon)
epsilon = 0.00001
```

For `x` in `(3x + 1)^2`:

```text
f(2 + epsilon) - f(2 - epsilon)
-------------------------------- ≈ 42
          2 * epsilon
```

Repeat for `w` and `b`. The numerical answers match `42`, `28`, and `14`
within `1e-8`. This independently checks graph wiring, traversal order, saved
values, and local derivative formulas.

## 8. What this tranche deliberately postpones

One node can eventually receive gradient contributions along several paths,
and repeated calls to backward can add into an existing leaf gradient. Those
are **gradient accumulation** behaviors. They are the next roadmap tranche.
Here every canonical graph stays tree-shaped so we can isolate graph creation,
saved snapshots, and one reverse traversal first.

## 9. A portable implementation boundary

Every language can implement the learning contract with a small node record:

```text
node id
executed operation
parent ids
forward value
saved snapshots
```

The host language should own dynamic control flow and object identity. A shared
Rust core can later execute bounded operation kernels through opaque handles,
explicit lengths, finite numeric buffers, and status codes. Do not pass raw
garbage-collected host pointers across a C ABI. After a graph is captured, a
compiler can lower the forward operations and their backward rules to
NeuralIR/MatrixIR while preserving the same saved-value semantics.

## Try it yourself

1. Change `b` from `1` to `-1` and recompute every forward and backward value.
2. Run the absolute-value graph with `x = 2`; identify which branch disappears.
3. Mutate live `x` after `x*w` and explain which saved snapshot computes
   `dProduct/dw`.
4. Draw a graph for `(x + 1)^2`, write one valid topological order, and reverse
   it.

The [NN27 fixture](../../specs/fixtures/dynamic-autograd-v1/README.md) and the
[interactive visualizer](../../programs/typescript/ml-learning-visualizer/README.md)
use these exact graphs.
