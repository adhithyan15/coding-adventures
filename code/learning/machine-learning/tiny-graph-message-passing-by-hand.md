# Tiny Graph Message Passing, by Hand

Images live on grids and sentences live in sequences. Some data instead lives
on a **graph**: a collection of nodes connected by edges. A node might be a
person, atom, road intersection, or web page. An edge says which nodes can
exchange information.

This lesson performs one complete message-passing round on three nodes. The
matching language-neutral contract is
[NN21](../../specs/NN21-tiny-message-passing-labs.md), with its canonical
fixture in
[`00-three-node-path.json`](../../specs/fixtures/tiny-message-passing-v1/labs/00-three-node-path.json).

## 1. Start with a path

Use three nodes and two undirected edges:

```text
node 0 ----- node 1 ----- node 2
  1            2           -1
```

The number under each node is its scalar feature. “Scalar” means one number.
An undirected edge has no arrow, so information may travel both ways.

## 2. Expand edges into directed messages

For computation, expand each undirected edge into two directions:

```text
0 -> 1
1 -> 0
1 -> 2
2 -> 1
```

Use one shared message rule:

```text
message(source -> target) = 0.5 * old_feature[source]
```

The messages are:

```text
1 -> 0: 0.5 *  2 =  1
0 -> 1: 0.5 *  1 =  0.5
2 -> 1: 0.5 * -1 = -0.5
1 -> 2: 0.5 *  2 =  1
```

The same weight applies everywhere. This sharing lets the rule work on graphs
with different node counts and layouts.

## 3. Aggregate each inbox

**Aggregate** comes from Latin roots meaning “bring into a flock.” In a graph
network it means combining all messages that arrive at one node. Use a sum:

```text
node 0 inbox: 1                 -> aggregate 1
node 1 inbox: 0.5 + (-0.5)      -> aggregate 0
node 2 inbox: 1                 -> aggregate 1
```

The middle node has two neighbors. Their messages cancel exactly. A sum is
permutation invariant: swapping the arrival order does not change the result.
That matters because a graph has no natural “first neighbor.”

## 4. Keep a self route

A node should not lose its own old feature while listening to neighbors. Give
the old feature a shared weight `0.25`, then add the inbox and bias `-0.5`:

```text
preactivation = 0.25 * old_feature + aggregate - 0.5
new_feature   = ReLU(preactivation)
```

ReLU keeps positive values and replaces negative values with zero.

Node 0:

```text
self route = 0.25 * 1 = 0.25
neighbor   = 1
bias       = -0.5
preact     = 0.25 + 1 - 0.5 = 0.75
output     = ReLU(0.75) = 0.75
```

Node 1:

```text
self route = 0.25 * 2 = 0.5
neighbor   = 0.5 + (-0.5) = 0
bias       = -0.5
preact     = 0.5 + 0 - 0.5 = 0
output     = ReLU(0) = 0
```

Node 2:

```text
self route = 0.25 * -1 = -0.25
neighbor   = 1
bias       = -0.5
preact     = -0.25 + 1 - 0.5 = 0.25
output     = ReLU(0.25) = 0.25
```

One round transforms:

```text
[1, 2, -1] -> [0.75, 0, 0.25]
```

## 5. Why the round is synchronous

All four messages use the original snapshot `[1, 2, -1]`. Node 0's new `0.75`
does not replace its old `1` while node 1 is still computing. The outputs become
the next round's inputs only after every node finishes.

This is **synchronous** computation: everyone reads the same old clock tick and
writes the same new clock tick. It differs deliberately from NN20's
asynchronous Hopfield sweep, where later updates read earlier changes.

## 6. The general message-passing shape

More expressive graph networks keep the same three stages:

1. **Message:** transform a source node, target node, and possibly edge data.
2. **Aggregate:** combine all messages arriving at each target.
3. **Update:** mix the aggregate with the target's previous state.

Our example uses one scalar, a source-only linear message, sum aggregation, and
a shared affine-plus-ReLU update. Later graph convolution and attention labs can
change those functions without hiding this skeleton.

## 7. Validate the corpus

From the repository root:

```text
python code/scripts/validate_tiny_message_passing_labs.py
```

The validator rejects duplicate or unknown keys, non-finite features, invalid
or repeated edges, self edges, isolated nodes, and any mismatch in messages,
inboxes, affine terms, or outputs.

## 8. Cross-language and Rust-core path

Implement this three-node round directly in every host language first. Each
consumer can compare every directed message and node update with the same JSON
oracle.

A Rust core can later consume COO or CSR edge-index buffers plus dense feature
buffers, perform deterministic segmented reductions, and fuse shared updates.
A stable C ABI can serve native FFI consumers and WASM can serve browsers. Even
when an optimized kernel fuses message, aggregate, and update, trace mode should
retain source and target identities so learners can still unpack the round.

## 9. Next experiment

Replace the fixed message weight with degree normalization, then let each target
assign different weights to its neighbors. Those two changes lead directly to
graph convolution and graph attention.
