# Hopfield Associative Memory, by Hand

A usual neural network sends information forward through layers. A Hopfield
network does something different: every neuron reads the current pattern and
the network repeatedly cleans that pattern up. A stored memory is not a row in
a database. It is a state that the dynamics prefer to settle into.

This lesson stores one four-bit memory, flips one bit, and restores it using
arithmetic small enough to check with a pencil. The matching language-neutral
contract is [NN20](../../specs/NN20-hopfield-associative-memory-labs.md), and
the canonical fixture is
[`00-one-bit-recall.json`](../../specs/fixtures/hopfield-associative-memory-v1/labs/00-one-bit-recall.json).

## 1. Replace bits with bipolar states

Hopfield networks traditionally use **bipolar** values: every neuron is either
`+1` or `-1`. Bipolar means “having two poles.” Here the two poles are the two
allowed states.

Save this pattern:

```text
p = [+1, -1, +1, -1]
```

You can imagine `+1` as a bright square and `-1` as a dark square. With only
four positions, the pattern is tiny, but the storage rule is the same one used
for longer patterns.

## 2. Store the pattern in connections

The Hebbian idea is often summarized as “units that are active together wire
together.” Donald Hebb proposed the underlying learning principle in 1949.
For bipolar values, equal signs produce a positive connection and opposite
signs produce a negative connection.

For four neurons, divide each outer-product entry by `4`:

```text
w_ij = p_i * p_j / 4   when i != j
w_ii = 0
```

An **outer product** pairs every entry of one vector with every entry of the
other. We use the saved pattern twice, so every neuron is paired with every
neuron.

The first row is:

```text
to neuron 0 from neuron 0: 0
to neuron 0 from neuron 1: (+1 * -1) / 4 = -0.25
to neuron 0 from neuron 2: (+1 * +1) / 4 = +0.25
to neuron 0 from neuron 3: (+1 * -1) / 4 = -0.25
```

The complete matrix is:

```text
W = [
  [ 0,    -0.25,  0.25, -0.25],
  [-0.25,  0,    -0.25,  0.25],
  [ 0.25, -0.25,  0,    -0.25],
  [-0.25,  0.25, -0.25,  0   ]
]
```

Two properties matter:

- The diagonal is zero, so a neuron does not vote for itself.
- The matrix is symmetric: `w_ij = w_ji`.

Symmetry will let us define one energy score for the whole network.

## 3. Damage one bit

Flip the first saved bit:

```text
saved p = [+1, -1, +1, -1]
cue   s = [-1, -1, +1, -1]
```

The Hamming distance is `1` because one position differs. Richard Hamming
developed the distance while studying error-correcting codes: it simply counts
positions that disagree.

We can also measure normalized overlap:

```text
overlap = sum_i(p_i * s_i) / 4
        = ((+1 * -1) + (-1 * -1) + (+1 * +1) + (-1 * -1)) / 4
        = (-1 + 1 + 1 + 1) / 4
        = 0.5
```

An overlap of `1` is a perfect match. The damaged cue starts halfway between a
perfect match and complete disagreement.

## 4. Give each neighbor one vote

Recall is asynchronous: update one neuron and immediately use its new state for
the next neuron. The fixture fixes the order as `0, 1, 2, 3` so every language
produces the same trace.

For neuron `0`, multiply its weight row by the damaged state:

```text
from 0:  0    * -1 = 0
from 1: -0.25 * -1 = +0.25
from 2: +0.25 * +1 = +0.25
from 3: -0.25 * -1 = +0.25
                          ------
local field                 +0.75
```

The **local field** is the total weighted vote arriving at one neuron. Apply a
sign activation:

```text
positive field -> +1
negative field -> -1
zero field     -> preserve the previous state
```

The field is positive, so neuron `0` changes from `-1` to `+1`:

```text
[-1, -1, +1, -1] -> [+1, -1, +1, -1]
```

The memory has already been restored.

The explicit zero-field rule matters. Some libraries define `sign(0)` as `0`,
but zero is not a valid bipolar neuron state. Preserving the previous state
keeps the model deterministic and bipolar.

## 5. Watch energy move downhill

The Hopfield energy is:

```text
E(s) = -1/2 * sum_i sum_j(w_ij * s_i * s_j)
```

The double sum counts every connection twice, once in each direction. The
factor `1/2` removes that double counting. The leading minus sign makes aligned
weighted relationships low energy.

For the damaged cue, the three pairs involving the flipped bit contribute
`-0.25` while the other three pairs contribute `+0.25`. They cancel:

```text
E(damaged cue) = 0
```

For the restored memory, all six unordered pairs contribute `+0.25`:

```text
directed sum = 2 * (6 * 0.25) = 3
E(memory)    = -1/2 * 3 = -1.5
```

The first update therefore changes energy from `0` to `-1.5`. Lower is better
in this energy landscape.

## 6. Finish the sweep

The remaining neurons see the restored state.

Neuron `1`:

```text
-0.25*(+1) + 0*(-1) + -0.25*(+1) + 0.25*(-1)
= -0.75 -> -1
```

Neuron `2`:

```text
0.25*(+1) + -0.25*(-1) + 0*(+1) + -0.25*(-1)
= +0.75 -> +1
```

Neuron `3`:

```text
-0.25*(+1) + 0.25*(-1) + -0.25*(+1) + 0*(-1)
= -0.75 -> -1
```

None changes. The completed trace is:

| Moment | State | Energy | Overlap |
| --- | --- | ---: | ---: |
| damaged cue | `[-1, -1, +1, -1]` | `0` | `0.5` |
| update neuron 0 | `[+1, -1, +1, -1]` | `-1.5` | `1` |
| update neuron 1 | `[+1, -1, +1, -1]` | `-1.5` | `1` |
| update neuron 2 | `[+1, -1, +1, -1]` | `-1.5` | `1` |
| update neuron 3 | `[+1, -1, +1, -1]` | `-1.5` | `1` |

The stored pattern is a **fixed point**: applying the update rule leaves it
unchanged. It is also an **attractor** for this damaged cue because the dynamics
move the cue into that fixed point.

## 7. What is guaranteed, and what is not

With symmetric weights, a zero diagonal, and asynchronous updates, Hopfield
energy cannot increase. The network eventually reaches a fixed point because a
finite bipolar network has only finitely many states.

That does not mean every fixed point is a memory you intended to store. Larger
Hopfield networks can have spurious attractors, and storing too many patterns
causes interference. This first lab deliberately stores one pattern so the
mechanism is visible before capacity questions arrive.

Parallel updates are also different. If all neurons compute from the same old
state and change together, the simple energy-descent story can break or enter a
two-state oscillation. NN20 therefore requires asynchronous in-place updates.

## 8. Run the deterministic corpus

From the repository root:

```text
python code/scripts/validate_hopfield_associative_memory_labs.py
```

The validator rejects unknown or duplicate keys, non-finite values,
non-bipolar states, malformed update orders, trace mismatches, energy increases,
and failure to recover the saved pattern.

## 9. Cross-language and Rust-core path

Implement this four-neuron loop directly in every host language first. The
fixture gives each implementation a shared oracle for:

- the learned weight matrix;
- every incoming weighted vote;
- each asynchronous state transition;
- energy and normalized overlap;
- the final fixed point.

A performant Rust core can later batch outer products and recall sweeps. Keep
the boundary simple: flat bipolar state buffers, flat row-major weights, and an
explicit integer update-order buffer. A stable C ABI can serve C-family and FFI
consumers; WASM can serve browsers; higher-level languages can add idiomatic
wrappers. Optimized calls may return only the fixed point, but a trace mode must
remain available so speed never puts the “magic” back into the lesson.

## 10. Next experiment

Store a second pattern and inspect where its outer product reinforces or
cancels the first. Then corrupt two bits and test whether the same update order
still reaches the intended memory. Those experiments introduce capacity,
interference, and spurious attractors without changing the underlying rules.
