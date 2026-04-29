# NN00: Neural Graph VM and Bytecode

Status: draft

## Purpose

NN00 describes how a user-authored neural network graph becomes executable
programs for CPU, GPU, neural-engine, or future accelerator backends.

The user-facing model is a graph:

- Nodes represent values, parameters, functions, reductions, activations, losses,
  and optimizer boundaries.
- Edges represent directed dataflow between nodes.
- Parallel edges are allowed and have stable edge IDs.
- Graph, node, and edge property bags carry metadata.

The runtime-facing model is bytecode:

- A compiler validates the graph and lowers it into a small Neural Network IR.
- The IR is serialized as Neural Network VM bytecode.
- The VM interprets or JIT-lowers bytecode into backend work.
- Backends are responsible for optimized kernels such as matrix multiply,
  vectorized activation, reduction, and parameter updates.

This spec deliberately separates the graph authoring surface from execution.
Graph metadata is descriptive. Bytecode is operational.

## Layering

```text
DT00 graph
  graph/node/edge property bags

DT01 directed graph
  ordered edges and predecessor/successor traversal

DT02 multi-directed graph
  ordered edges plus stable edge IDs and parallel edges

NN00 neural graph compiler
  validates graph metadata and emits Neural Network IR

NN01 neural VM
  executes IR/bytecode for inference and training

Backend adapters
  CPU matrix backend, GPU matrix backend, neural-engine backend, future ASICs
```

## Graph Contract

The neural graph compiler consumes a multi-directed graph with this minimum API.
The graph itself is generic and domain-neutral; neural behavior comes from a
primitive library that writes reserved `nn.*` metadata onto graph, node, and edge
property bags.

```text
nodes() -> NodeId[]
edges() -> Edge[]
node_properties(node) -> PropertyBag
edge_properties(edge_id) -> PropertyBag
successors(node) -> NodeId[]
predecessors(node) -> NodeId[]
outgoing_edges(node) -> Edge[]
incoming_edges(node) -> Edge[]
topological_sort() -> NodeId[]
```

`Edge` contains:

```text
id: EdgeId
from: NodeId
to: NodeId
weight: number
```

The graph may contain parallel edges:

```text
edge e0: input -> sum, { "weight": 0.25, "channel": "x0" }
edge e1: input -> sum, { "weight": 0.75, "channel": "x1" }
```

Parallel edges are distinct trainable connections. The compiler must not collapse
them unless an optimization pass proves the collapse preserves gradients and edge
identity does not need to be exposed.

## Property Vocabulary

Property values are portable JSON-like scalars:

```text
PropertyValue = string | number | boolean | null
PropertyBag = map<string, PropertyValue>
```

### Graph Properties

Reserved keys:

| Key | Type | Meaning |
| --- | --- | --- |
| `nn.version` | string | Neural graph metadata version. |
| `nn.name` | string | Human-readable model name. |
| `nn.mode` | string | `inference`, `training`, or `both`. |
| `nn.default_dtype` | string | Default tensor dtype such as `fp32`, `fp16`, `bf16`, `int8`. |
| `nn.loss` | string | Optional loss node ID. |
| `nn.optimizer` | string | Optimizer name such as `sgd`, `adam`, `none`. |

### Node Properties

Reserved keys:

| Key | Type | Meaning |
| --- | --- | --- |
| `nn.op` | string | Node operation kind. Required for executable nodes. |
| `nn.shape` | string | Tensor shape expression, for example `[-1, 784]`. |
| `nn.dtype` | string | Tensor dtype override. |
| `nn.requires_grad` | boolean | Whether gradients are tracked for this node. |
| `nn.param` | string | Parameter ID for trainable state. |
| `nn.initializer` | string | Initializer such as `zeros`, `ones`, `xavier_uniform`. |
| `nn.activation` | string | Activation name for activation nodes. |
| `nn.axis` | number | Axis used by reductions or concatenations. |

Core v0 operation kinds:

| `nn.op` | Description |
| --- | --- |
| `input` | Runtime-fed tensor. |
| `constant` | Immutable scalar or tensor literal. |
| `parameter` | Trainable tensor. |
| `weighted_sum` | Sum incoming values after per-edge weights. |
| `bias` | Add a bias parameter or scalar. |
| `add` | Elementwise add. |
| `matmul` | Matrix multiply. |
| `activation` | Elementwise activation. |
| `loss` | Loss reduction. |
| `output` | Named output boundary. |

### Edge Properties

Reserved keys:

| Key | Type | Meaning |
| --- | --- | --- |
| `weight` | number | Canonical edge weight. |
| `nn.trainable` | boolean | Whether the edge weight is trainable. |
| `nn.param` | string | Optional parameter ID backing this edge. |
| `nn.gradient` | string | Optional gradient slot name. |
| `nn.channel` | string | Human-readable channel or port name. |
| `nn.port` | string | Input port name on the destination op. |

`weight` is part of the graph data-structure contract, not only the neural
contract. Setting the `weight` edge property must update the edge weight API.

## Primitive Library

Users should not have to hand-author raw metadata for common neural network
building blocks. Each language port should expose a small primitive layer on top
of the generic multi-directed graph.

Required v0 primitives:

```text
create_neural_graph(name?)
add_input(graph, node_id, input_name = node_id)
add_weighted_sum(graph, node_id, inputs)
add_activation(graph, node_id, input_node_id, activation)
add_output(graph, node_id, input_node_id, output_name = node_id)
```

The primitive layer is syntactic sugar plus validation. It creates ordinary graph
nodes and edges with reserved properties:

```text
add_activation(graph, "relu1", "dense1", "relu")

node_properties["relu1"]["nn.op"] = "activation"
node_properties["relu1"]["nn.activation"] = "relu"
edge dense1 -> relu1
```

Future primitive libraries may include `dense`, `conv2d`, `layer_norm`,
`dropout`, recurrent blocks, and optimizer definitions. Those primitives should
still lower to generic graph metadata before the compiler sees them.

## Compiler Pipeline

The graph compiler has deterministic passes:

1. Validate graph shape.
2. Validate reserved property keys and scalar value types.
3. Reject cycles for feed-forward inference graphs.
4. Resolve graph inputs, outputs, parameters, constants, and trainable edges.
5. Topologically order executable nodes.
6. Infer value slots and tensor shapes.
7. Lower graph nodes and edges into Neural Network IR operations.
8. Run graph-level optimizations.
9. Plan buffers and parameter slots.
10. Emit bytecode.

Training compiles two programs:

- Forward program: computes outputs and writes a tape.
- Backward program: consumes the tape and accumulates gradients.

The initial implementation may emit only forward bytecode, but the IR must reserve
IDs and instruction families for backward execution so the design does not paint
itself into a corner.

## IR Data Model

IR objects are stable and serializable:

```text
ModuleId       string
FunctionId     string
NodeId         string
EdgeId         string
ValueId        v0, v1, ...
ParamId        p0, p1, ...
TensorSlotId   t0, t1, ...
InstructionId  i0, i1, ...
```

An IR module contains:

```text
NeuralModule {
  version: 0
  name: string
  graph: GraphDebugInfo
  inputs: ValueBinding[]
  outputs: ValueBinding[]
  parameters: ParameterBinding[]
  constants: ConstantBinding[]
  functions: NeuralFunction[]
}
```

`GraphDebugInfo` preserves source node and edge IDs for visualization, error
messages, and gradient inspection.

## Bytecode Format

Version 0 uses a portable structured encoding before introducing a compact binary
format. This keeps tests readable and lets every language port target the same
semantics.

```text
BytecodeModule {
  magic: "CANN"
  version: 0
  constants: ConstantTable
  parameters: ParameterTable
  functions: FunctionTable
}
```

Each function contains a flat instruction list:

```text
Function {
  id: FunctionId
  kind: "forward" | "backward" | "optimizer"
  instructions: Instruction[]
}
```

The future binary encoding should map directly onto this structure:

```text
u32 magic
u16 major
u16 minor
u32 constant_table_offset
u32 parameter_table_offset
u32 function_table_offset
...
```

## Instruction Set

Instruction names are stable. Operand encoding may evolve.

| Opcode | Operands | Meaning |
| --- | --- | --- |
| `LOAD_INPUT` | `dst, input_name` | Bind runtime input tensor to a value slot. |
| `LOAD_PARAM` | `dst, param_id` | Load trainable parameter. |
| `LOAD_CONST` | `dst, const_id` | Load scalar or tensor constant. |
| `LOAD_EDGE_WEIGHT` | `dst, edge_id` | Load edge weight as scalar. |
| `MUL` | `dst, left, right` | Elementwise or scalar multiply. |
| `ADD` | `dst, inputs[]` | Elementwise add/reduce sum. |
| `WEIGHTED_SUM` | `dst, terms[]` | Sum incoming values multiplied by edge weights. |
| `MATMUL` | `dst, left, right` | Matrix multiply. |
| `ADD_BIAS` | `dst, input, bias` | Bias add with broadcasting. |
| `ACTIVATE` | `dst, input, activation` | Elementwise activation. |
| `LOSS` | `dst, predicted, target, loss_kind` | Loss reduction. |
| `STORE_OUTPUT` | `name, value` | Store named output. |
| `TAPE_SAVE` | `key, value` | Save value for backward pass. |
| `GRAD_SEED` | `dst, output` | Seed backward gradient. |
| `GRAD_OP` | `dsts[], op_ref, grad_in` | Compute op-local gradients. |
| `ACCUM_GRAD` | `param_id, grad` | Accumulate parameter gradient. |
| `APPLY_OPTIMIZER` | `optimizer, param_id, grad` | Update trainable state. |
| `BARRIER` | `scope` | Scheduling barrier for backend lowering. |
| `NOP` | none | No operation. |

## Lowering Examples

### Scalar Weighted Sum

Graph:

```text
x0 --e0(weight=0.25)--> sum
x1 --e1(weight=0.75)--> sum
sum --e2(weight=1.0)--> out
```

Node metadata:

```text
x0:  { "nn.op": "input" }
x1:  { "nn.op": "input" }
sum: { "nn.op": "weighted_sum" }
out: { "nn.op": "output" }
```

Forward bytecode:

```text
LOAD_INPUT      v0, "x0"
LOAD_INPUT      v1, "x1"
LOAD_EDGE_WEIGHT v2, "e0"
LOAD_EDGE_WEIGHT v3, "e1"
MUL             v4, v0, v2
MUL             v5, v1, v3
ADD             v6, [v4, v5]
STORE_OUTPUT    "out", v6
```

A backend may fuse this sequence into a dot product or a matrix operation.

### Dense Layer

Graph:

```text
input -> matmul
weights -> matmul
matmul -> bias
bias_param -> bias
bias -> relu
relu -> output
```

Forward bytecode:

```text
LOAD_INPUT   v0, "input"
LOAD_PARAM   v1, "dense.weight"
MATMUL       v2, v0, v1
LOAD_PARAM   v3, "dense.bias"
ADD_BIAS     v4, v2, v3
ACTIVATE     v5, v4, "relu"
STORE_OUTPUT "output", v5
```

## VM Execution Modes

### Inference

Inference executes only the forward function. It may skip tape instructions and
must not mutate parameters.

### Training

Training executes:

1. Forward function with tape saving enabled.
2. Loss function.
3. Backward function.
4. Optimizer function.

The VM must expose gradients for debugging before optimizer application.

### Determinism

The compiler must emit deterministic bytecode for the same graph:

- Node order uses topological order with stable tie-breaking.
- Edge order uses edge ID order.
- Property insertion order is not semantically meaningful.

## Backend Responsibilities

The VM should not hard-code matrix kernels. It dispatches backend-neutral ops:

```text
backend.matmul(a, b, dtype, layout)
backend.add(inputs)
backend.activate(input, activation)
backend.reduce(input, kind, axis)
backend.apply_optimizer(param, grad, optimizer_state)
```

Backends decide whether to:

- Interpret scalar ops directly.
- Fuse scalar weighted sums into vector dot products.
- Batch compatible matmuls.
- Lower to CPU loops, GPU kernels, neural-engine schedules, or future ASIC IR.

## Optimization Passes

Required v0 passes:

- Constant folding for scalar constants.
- Dead-output elimination.
- Linear chain fusion for `matmul -> bias -> activation`.
- Weighted-sum fusion into dot product when all inputs are scalars or vectors.

Future passes:

- Shape-specialized kernel selection.
- Layout conversion insertion.
- Mixed precision rewrite.
- Gradient checkpointing.
- Quantization-aware lowering.

## Error Model

Compiler errors must point back to graph IDs:

```text
NeuralGraphCompileError {
  code: string
  message: string
  node_id?: NodeId
  edge_id?: EdgeId
}
```

Examples:

- Missing `nn.op` on executable node.
- Unsupported `nn.op`.
- Non-numeric edge `weight`.
- Cycle in feed-forward graph.
- Shape mismatch.
- Missing required input port.
- Training requested for an op without a gradient rule.

## Minimal End-to-End Milestone

The first useful implementation should support:

1. A multi-directed graph package with stable edge IDs and property bags.
2. A neural graph compiler that lowers `input`, `weighted_sum`, `activation`, and
   `output` to forward bytecode.
3. A tiny VM interpreter that executes that bytecode on arrays/scalars.
4. Tests showing equivalent output for a hand-authored graph and direct numeric
   computation.
5. Debug output mapping bytecode instructions back to graph node and edge IDs.

Only after that milestone should training/backprop bytecode be implemented.
