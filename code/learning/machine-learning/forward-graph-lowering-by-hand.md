# Forward Graph Lowering, by Hand

A neural network starts life as a picture:

```text
x0 --0.25--\
              weighted sum -> ReLU -> prediction
x1 --0.75--/
bias -- -1-/
```

That picture explains **what depends on what**. A computer still needs an
ordered program that says what to load, multiply, add, and store. Turning the
graph into that more operational form is called **lowering**.

Lowering is not approximation. It is translation. If the graph predicts `6`,
every correct lowered form must also predict `6`.

This lesson follows one tiny graph through three levels:

1. the author-facing neural graph;
2. NN00 NeuralIR, represented by portable `CANN` forward instructions; and
3. NN01 MatrixIR, represented today by the neural VM's fused `CANM` matrix
   plan.

The final section connects that matrix plan to the Rust MX01 `matrix-ir` tensor
DAG. The names are close, but their jobs are different.

## The whole idea in one sentence

The graph preserves meaning, NeuralIR makes execution order explicit, and
MatrixIR groups scalar work into batch-shaped operations that a backend can
run efficiently.

## Vocabulary before arithmetic

- **Intermediate representation**, or **IR**: a program form between the
  friendly source and the machine that executes it.
- **Lowering**: translating a higher-level representation into a lower-level,
  more operational one.
- **Topological order**: an ordering in which every input is produced before an
  operation reads it.
- **Value ID**: a stable name such as `v6` for an intermediate result.
- **SSA**, or **static single assignment**: the convention that each value ID is
  written once. New results receive new IDs instead of overwriting old ones.
- **Fusion**: replacing several small operations with one larger operation that
  has the same observable result.
- **Provenance**: the connection from a lowered operation back to the graph
  node, edge, or earlier instruction that produced it.
- **Parity**: agreement between independent execution paths.

## 1. The graph

Our graph has six nodes.

| Node | Kind | Meaning |
| --- | --- | --- |
| `x0` | input | first runtime number |
| `x1` | input | second runtime number |
| `bias` | constant | the number `1` |
| `sum` | weighted sum | add three weighted incoming values |
| `relu` | activation | replace negative values with `0` |
| `out` | output | publish the result as `prediction` |

Its five edges are:

| Edge | From | To | Weight |
| --- | --- | --- | ---: |
| `w0` | `x0` | `sum` | `0.25` |
| `w1` | `x1` | `sum` | `0.75` |
| `bias_to_sum` | `bias` | `sum` | `-1` |
| `sum_to_relu` | `sum` | `relu` | `1` |
| `relu_to_out` | `relu` | `out` | `1` |

The last two weights describe graph connectivity. The activation and output
nodes consume their single predecessors directly, so those weights do not need
separate arithmetic instructions.

## 2. Calculate the graph on paper

Use:

```text
x0 = 4
x1 = 8
bias = 1
```

The three contributions to `sum` are:

```text
bias contribution =  1 x -1    = -1
x0 contribution   =  4 x 0.25  =  1
x1 contribution   =  8 x 0.75  =  6
```

Add them:

```text
z = -1 + 1 + 6 = 6
```

Apply ReLU:

```text
prediction = max(0, z) = max(0, 6) = 6
```

That `6` is the reference result. A compiler is wrong if either lowered form
produces anything else.

## 3. Pick a deterministic order

Several nodes are ready at the beginning: `bias`, `x0`, and `x1` have no
incoming dependencies. A compiler still needs one stable answer, so it uses a
topological order with a stable tie-break:

```text
bias, x0, x1, sum, relu, out
```

Incoming weighted edges are also sorted by edge ID:

```text
bias_to_sum, w0, w1
```

This does not change the math. It makes fixture files, caches, diffs, and every
language port agree byte for byte on the same program shape.

## 4. Lower the graph into NeuralIR

NN00 uses a portable forward instruction stream. Each instruction either writes
a fresh value ID or publishes an output.

| ID | NeuralIR instruction | What it does |
| --- | --- | --- |
| `i0` | `LOAD_CONST v0, 1` | materialize `bias` |
| `i1` | `LOAD_INPUT v1, x0` | bind the first input |
| `i2` | `LOAD_INPUT v2, x1` | bind the second input |
| `i3` | `LOAD_EDGE_WEIGHT v3, bias_to_sum` | load `-1` |
| `i4` | `MUL v4, v0, v3` | compute the bias contribution |
| `i5` | `LOAD_EDGE_WEIGHT v5, w0` | load `0.25` |
| `i6` | `MUL v6, v1, v5` | compute the `x0` contribution |
| `i7` | `LOAD_EDGE_WEIGHT v7, w1` | load `0.75` |
| `i8` | `MUL v8, v2, v7` | compute the `x1` contribution |
| `i9` | `ADD v9, [v4, v6, v8]` | add all contributions |
| `i10` | `ACTIVATE v10, v9, relu` | apply ReLU |
| `i11` | `STORE_OUTPUT prediction, v10` | publish the answer |

Replay those instructions with our numbers:

```text
v0 = 1       v3 = -1     v4 = -1
v1 = 4       v5 = 0.25   v6 = 1
v2 = 8       v7 = 0.75   v8 = 6
v9 = -1 + 1 + 6 = 6
v10 = ReLU(6) = 6
prediction = 6
```

Notice the SSA rule: `v4`, `v6`, and `v8` remain the three visible
contributions. The addition writes a new value, `v9`.

## 5. Lower NeuralIR into MatrixIR

The scalar program is wonderfully inspectable, but a backend should not launch
seven separate kernels merely to load three weights, multiply three columns,
and add them. NN01 recognizes that pattern and fuses `i3` through `i9` into one
matrix operation.

| ID | MatrixIR operation | Comes from |
| --- | --- | --- |
| `m0` | `LOAD_CONST_MATRIX v0, 1` | `i0` |
| `m1` | `LOAD_INPUT_MATRIX v1, x0` | `i1` |
| `m2` | `LOAD_INPUT_MATRIX v2, x1` | `i2` |
| `m3` | `WEIGHTED_SUM_MATRIX v9` | `i3` through `i9` |
| `m4` | `ACTIVATE_MATRIX v10, v9, relu` | `i10` |
| `m5` | `STORE_OUTPUT_MATRIX prediction, v10` | `i11` |

The fused operation still carries the three edge IDs and weights:

```text
terms = [
  (v0, bias_to_sum, -1),
  (v1, w0,           0.25),
  (v2, w1,           0.75),
]
```

Fusion removed instruction overhead. It did not erase provenance.

## 6. Why matrices help even for this tiny graph

Now run two rows together:

```text
x0 column = [4, 8]
x1 column = [8, 16]
bias      = [1, 1]
```

The fused weighted sum works columnwise:

```text
row 0: 1 x -1 + 4 x 0.25 +  8 x 0.75 =  6
row 1: 1 x -1 + 8 x 0.25 + 16 x 0.75 = 13
```

Both values are positive, so ReLU preserves them:

```text
prediction column = [6, 13]
```

The graph did not change. The NeuralIR program did not change. Only the runtime
input columns became longer.

## 7. Three independent parity checks

The NN29 fixture evaluates every example three ways:

1. directly from graph semantics;
2. by interpreting the NeuralIR instruction stream once per row; and
3. by executing the fused MatrixIR plan over whole columns.

For the canonical examples:

| Example | Direct graph | NeuralIR | MatrixIR |
| --- | --- | --- | --- |
| one row | `[6]` | `[6]` | `[6]` |
| two rows | `[6, 13]` | `[6, 13]` | `[6, 13]` |

Agreement is the important property. Counting fewer matrix operations is only
useful after correctness is preserved.

## 8. Where Rust `matrix-ir` fits

The TypeScript neural VM currently calls its NN01 output a **matrix plan** with
magic `CANM`. It still knows neural concepts such as weighted sums and
activations.

The Rust MX01 crate named `matrix-ir` sits one level lower. It is a pure,
backend-neutral tensor DAG with operations such as:

```text
Const, Mul, Add, Max, MatMul, ReduceSum, ...
```

A future Rust bridge can translate the six-operation neural matrix plan into
that typed tensor DAG. For this example, `WEIGHTED_SUM_MATRIX` becomes constants,
elementwise multiplies, and adds; ReLU becomes `Max(value, zero)`.

Keep the boundary honest:

- the neural compiler owns graph meaning, stable source IDs, and fusion rules;
- MX01 `matrix-ir` owns immutable tensor IDs, dtypes, static shapes, and pure
  tensor algebra;
- a planner owns placement and buffer residency; and
- CPU, GPU, WebGPU, or accelerator executors own kernels.

Rust should never have to guess what an unnamed tensor means. The host bridge
must provide explicit shapes, dtypes, constants, inputs, outputs, and provenance
metadata before crossing the boundary.

## 9. A language-neutral implementation recipe

Every language can implement the same small compiler without sharing runtime
objects:

1. parse and validate the NN29 JSON fixture;
2. reject unknown node kinds, dangling edges, duplicate IDs, cycles, unsupported
   activations, non-finite numbers, and oversized batches;
3. compute stable topological node order;
4. sort incoming weighted edges by ID;
5. allocate `v0`, `v1`, ... exactly once;
6. emit the normalized `CANN` instruction objects;
7. recognize the weight-load, multiply, add pattern;
8. emit the normalized `CANM` operations with every source instruction ID;
9. execute direct, NeuralIR, and MatrixIR paths; and
10. compare all outputs with the canonical tolerance.

The JSON fixture owns values and expected traces. Rust, Python, Go, TypeScript,
Swift, Java, and every other consumer should not copy numbers from this prose.

## 10. What lowering does not do yet

This tranche is forward-only. It does not lower:

- saved values for backward;
- gradient accumulation;
- optimizer state or updates;
- device placement;
- mixed precision or quantization; or
- buffer reuse.

Those are separate roadmap steps because each introduces new observable state.

## Exercises

1. Change the first row to `x0 = 0`, `x1 = 0`. Which value does ReLU change?
2. Swap the order of the node and edge records. Why should the emitted IR stay
   identical?
3. Remove `w1`. Which graph validation or output changes should occur?
4. Add a second output that reads `sum` before ReLU. Which NeuralIR instruction
   can both outputs reuse?
5. Sketch the MX01 tensor operations for `m3`. Where must constants be expanded
   because MX01 V1 does not allow implicit binary broadcasting?

The central habit is simple: never accept a compiler optimization because it
looks faster. First prove that the graph, NeuralIR, and MatrixIR still compute
the same small arithmetic.
