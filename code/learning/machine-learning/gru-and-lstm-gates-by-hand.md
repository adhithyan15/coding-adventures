# GRU and LSTM Gates, by Hand

A plain recurrent cell always mixes its new input and previous hidden state in
the same way. A gated cell learns numbers between `0` and `1` that control how
much information is kept, written, forgotten, or exposed.

This lesson sends the same scalar memory through a GRU and an LSTM. The
language-neutral oracle is
[`00-gru-lstm-gates.json`](../../specs/fixtures/gated-recurrent-v1/labs/00-gru-lstm-gates.json).
The complete contract is
[`NN11-gated-recurrent-labs.md`](../../specs/NN11-gated-recurrent-labs.md).

## What a gate is

A gate is normally produced by a learned affine calculation followed by a
sigmoid:

```text
gate = sigmoid(weighted inputs + bias)
```

Sigmoid maps every finite number between `0` and `1`. Multiplying a signal by
the gate makes it a soft valve:

```text
gate = 0     blocks the signal
gate = 0.25  passes one quarter
gate = 1     passes all of it
```

The cells below use preactivations chosen to produce simple gate values:

```text
sigmoid(0)       = 0.5
sigmoid(-ln(3))  = 0.25
sigmoid( ln(3))  = 0.75
tanh(atanh(0.6)) = 0.6
```

The same input arrives at both cells:

```text
x                 = 1
previous hidden h = 0.8
previous cell c   = 0.8  (LSTM only)
candidate         = 0.6
```

## GRU: reset, then mix one state

A gated recurrent unit keeps one state, `h`. This lesson uses `z` as the share
of the new candidate:

```text
r = reset gate
z = update gate
n = candidate

n = tanh(input product + U * (r * h_previous) + bias)
h = (1 - z) * h_previous + z * n
```

Some libraries name `z` in the opposite direction. Always inspect the equation,
not only the gate name.

### Reset gate constructs the candidate

Use reset gate `r = 0.5`:

```text
reset previous state = 0.5 * 0.8
                     = 0.4

candidate preactivation = 0 + 1 * 0.4 + 0.29314718056
                        = 0.69314718056

n = tanh(0.69314718056)
  = 0.6
```

The reset gate changes what old information is available while constructing
the candidate. It does not directly choose the final mixture.

### Update gate mixes old and new

Use update gate `z = 0.25`:

```text
retained old state = (1 - 0.25) * 0.8
                   = 0.75 * 0.8
                   = 0.6

candidate write = 0.25 * 0.6
                = 0.15

next hidden state = 0.6 + 0.15
                  = 0.75
```

The GRU exposes the same number it stores: `h_next = 0.75`.

## LSTM: maintain a cell, then choose what to expose

A long short-term memory cell has two state values:

- `c`, the cell state carried along the memory path; and
- `h`, the hidden state exposed to the rest of the network.

Its scalar equations are:

```text
f = forget gate
i = input gate
o = output gate
g = candidate

c_next = f * c_previous + i * g
h_next = o * tanh(c_next)
```

Use `f = 0.5`, `i = 0.25`, `o = 0.75`, and `g = 0.6`.

### Forget and input gates update the private cell

```text
retained cell = 0.5 * 0.8
              = 0.4

candidate write = 0.25 * 0.6
                = 0.15

next cell state = 0.4 + 0.15
                = 0.55
```

Notice that this update has the same two numerical contributions as the GRU
example. The architectural difference appears next.

### Output gate controls the public hidden state

```text
squashed cell = tanh(0.55)
              = 0.50052021119

next hidden state = 0.75 * 0.50052021119
                  = 0.375390158393
```

The LSTM remembers `c_next = 0.55` internally while exposing
`h_next = 0.375390158393`. Hiding part of the cell does not erase it.

## The comparison in one table

| question | GRU | LSTM |
| --- | --- | --- |
| Stored state | one hidden state | cell state plus hidden state |
| Candidate control | reset gate | candidate has its own tanh path |
| Old-memory control | update mixture | forget gate |
| New-write control | update mixture | input gate |
| Exposure control | no separate gate | output gate |
| Canonical result | `h = 0.75` | `c = 0.55`, `h = 0.375390158393` |

Neither cell is automatically better. A GRU has fewer gates and state buffers;
an LSTM separates long-lived cell memory from what is exposed at each step.
Data, sequence length, compute constraints, and training behavior decide which
tradeoff matters.

## Change one gate at a time

Counterfactuals make each responsibility visible.

### GRU update extremes

```text
z = 0: h_next = 1 * 0.8 + 0 * 0.6 = 0.8  (preserve old)
z = 1: h_next = 0 * 0.8 + 1 * 0.6 = 0.6  (replace with candidate)
```

### GRU reset off

With `r = 0`, the candidate cannot see the previous hidden state:

```text
n = tanh(0 + 0 + 0.29314718056)
  = 0.285028898194

h_next = 0.75 * 0.8 + 0.25 * 0.285028898194
       = 0.671257224548
```

### LSTM forget, input, and output off

```text
f = 0: c_next = 0 + 0.15 = 0.15
i = 0: c_next = 0.4 + 0 = 0.4
o = 0: c_next stays 0.55, but h_next = 0
```

Turning the output gate back to `1` exposes `tanh(0.55) = 0.50052021119`
without changing the stored cell.

## Common implementation bugs

- Mixing two opposite GRU update-gate conventions without an adapter.
- Applying the GRU reset gate after candidate construction instead of inside it.
- Treating the LSTM cell state and hidden state as one buffer.
- Multiplying the output gate into `c_next` and thereby erasing private memory.
- Forgetting the candidate's tanh or the final `tanh(c_next)` in the LSTM.
- Reusing one gate's parameters for another gate accidentally when packing
  matrices for a fast kernel.
- Hiding recurrent state inside a runtime object, which breaks chunked and
  cross-language execution.

## Try the gate comparator

The
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
aligns the GRU and LSTM memory lanes. Select a gate to see its canonical value
and its zero-or-one counterfactual without changing the other signals. The LSTM
lane keeps cell and hidden state visibly separate.

## Cross-language checkpoint

An NN11 consumer is conformant when it reproduces every gate from its
preactivation, both canonical cells, and all seven one-gate counterfactuals.

Implement the scalar fixture directly in every language first. A future Rust
core can fuse vector gate projections for performance, but its trace mode and C
ABI should expose gate buffers and caller-owned recurrent state. GRU needs one
state buffer; LSTM needs an explicit `(hidden, cell)` pair. That distinction
must survive every binding rather than becoming Rust-only knowledge.
