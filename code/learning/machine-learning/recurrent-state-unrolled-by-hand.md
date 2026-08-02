# One Recurrent State, Unrolled by Hand

A feed-forward network receives an input, produces an output, and finishes. A
sequence network needs a way for an earlier input to affect a later step. The
smallest possible memory is one number called the **hidden state**.

This lesson runs one recurrent cell for only three time steps. The complete
language-neutral oracle is
[`00-three-step-relu-state.json`](../../specs/fixtures/recurrent-unroll-v1/labs/00-three-step-relu-state.json).
The formulas and conformance rules are in
[`NN09-recurrent-unroll-labs.md`](../../specs/NN09-recurrent-unroll-labs.md).

## One cell, used repeatedly

At every time step, the cell receives two values:

1. the new sequence value `x[t]`; and
2. the previous hidden state `h[t - 1]`.

It combines them with one shared parameter set:

```text
input contribution  = W_x * x[t]
memory contribution = W_h * h[t - 1]
a[t]                = input contribution + memory contribution + b
h[t]                = ReLU(a[t])
```

`a[t]` is the value before activation. ReLU keeps positive values and replaces
negative values with zero.

Use these tiny numbers:

```text
sequence x         = [1, 2, 0]
initial state h[-1] = 0
input weight W_x    = 2
memory weight W_h   = 0.5
bias b              = -1
```

The initial state is explicit. A production model might receive it from a
previous sequence chunk, initialize it to zeros, or let a caller supply it. It
must not appear from hidden runtime state.

## Time step 0

The first step sees `x[0] = 1` and the initial state `h[-1] = 0`:

```text
input contribution  = 2 * 1   = 2
memory contribution = 0.5 * 0 = 0
a[0]                = 2 + 0 - 1
                    = 1
h[0]                = ReLU(1)
                    = 1
```

The new state `1` is both this step's result and part of the next step's input.

## Time step 1

The second step sees new input `2` and carried state `1`:

```text
input contribution  = 2 * 2   = 4
memory contribution = 0.5 * 1 = 0.5
a[1]                = 4 + 0.5 - 1
                    = 3.5
h[1]                = ReLU(3.5)
                    = 3.5
```

Nothing new was learned between time steps. `W_x`, `W_h`, and `b` have exactly
the same values as they had at step 0. Reusing them is what makes this one
recurrent layer rather than three unrelated layers.

## Time step 2: no new signal, but memory remains

The final sequence input is zero:

```text
input contribution  = 2 * 0     = 0
memory contribution = 0.5 * 3.5 = 1.75
a[2]                = 0 + 1.75 - 1
                    = 0.75
h[2]                = ReLU(0.75)
                    = 0.75
```

The final output is positive even though the new input is zero. Earlier inputs
changed `h[1]`, and `h[1]` crossed the recurrent link into the final step. That
is the entire memory mechanism in this tiny network.

The complete state sequence is:

```text
initial -> step 0 -> step 1 -> step 2
   0         1        3.5       0.75
```

## What “unrolling” means

The implementation can be a loop containing one cell:

```text
state = initial_state
for input in sequence:
    state = ReLU(W_x * input + W_h * state + b)
```

To inspect dependencies, draw one copy of that cell for each time step:

```text
h[-1] -> [cell t=0] -> h[0] -> [cell t=1] -> h[1] -> [cell t=2] -> h[2]
             ^                       ^                       ^
            x[0]                    x[1]                    x[2]
```

This drawing is the **unrolled graph**. The boxes are executions at different
times, not independently parameterized cells. Every box points back to the
same `W_x`, `W_h`, and `b`.

Unrolling turns a cycle described over time into an acyclic chain for one
finite sequence. The next lesson can walk backward through that chain and show
how shared parameters collect gradient contributions from several steps.

## Break the recurrent link

To isolate memory, keep all inputs and parameters fixed but replace every
memory contribution with zero:

```text
time:                 0    1    2
state with memory:    1   3.5  0.75
state without memory: 1   3    0
difference:           0   0.5  0.75
```

Without recurrence, each step becomes an independent calculation:

```text
h[t] = ReLU(2 * x[t] - 1)
```

The zero final input then produces `ReLU(-1) = 0`. This counterfactual does not
claim the recurrent state stores every detail forever. It proves which part of
this result came through the memory path.

## What this model can and cannot remember

One scalar state compresses all earlier inputs into one number. It cannot
recover the original sequence `[1, 2]` from `3.5`; different histories could
produce the same state. Larger RNNs use vectors, but the compression problem
remains.

ReLU also changes the memory behavior. A negative preactivation resets this
cell to zero, while a large positive recurrent weight could make states grow.
Later lessons will expose the resulting vanishing and exploding gradients, and
then compare gated cells such as GRUs and LSTMs.

## Try the interactive unroll

The
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
draws all three executions on one time axis. Select a step to inspect its input
term, carried-state term, bias, preactivation, and ReLU result. Toggle the
recurrent link to compare the same sequence with no memory.

## Cross-language checkpoint

An implementation is ready for the next sequence lesson when it can load the
same JSON fixture and reproduce:

- every input and recurrent product;
- all three preactivations and states;
- the explicit initial-to-final state chain;
- one parameter set shared across every time step; and
- the no-recurrence states and per-step differences.

Rust can execute V1 with a scalar loop or an explicitly unrolled acyclic neural
graph. Other languages can implement the same loop directly or call a future
Rust sequence core through a stable C ABI. In either case, initial-state input,
final-state output, parameter ownership, and optional trace buffers must remain
explicit so “memory” never means invisible process state.
