# NN20 Hopfield Associative Memory Labs

Status: Draft

NN20 defines a deterministic, language-neutral contract for the smallest useful
Hopfield recall loop. It makes storage, corruption, asynchronous updates, and
energy descent visible without requiring a tensor framework.

## Learning Goal

A consumer must show that a memory can be represented as a low-energy state:

1. encode one bipolar pattern with a normalized Hebbian outer product;
2. remove self-connections by zeroing the diagonal;
3. start from a cue with one flipped bit;
4. update neurons in the fixture order using the newest state immediately;
5. verify energy never increases and the stored pattern is recovered.

## V1 Operation

For a saved pattern `p` of length `N`:

```text
w_ij = p_i * p_j / N  when i != j
w_ii = 0
```

For one asynchronous neuron update:

```text
field_i = sum_j(w_ij * state_j)
state_i = +1 when field_i > 0
state_i = -1 when field_i < 0
state_i = previous state_i when field_i = 0
```

The zero-field rule is part of the contract. It removes a common cross-language
disagreement about whether `sign(0)` returns zero or a bipolar value.

## Audit Quantities

The Hopfield energy is:

```text
E(state) = -1/2 * sum_i sum_j(w_ij * state_i * state_j)
```

The normalized overlap with the stored pattern is:

```text
overlap = sum_i(p_i * state_i) / N
```

The fixture records every incoming weighted vote, field, state transition,
energy, and overlap. The matrix must be symmetric, its diagonal must be zero,
and each asynchronous update must have `energy_after <= energy_before` within
the document tolerance.

## Determinism

V1 stores exactly one pattern and saves a complete update-order permutation.
Consumers must update in place: later neurons see the state produced by earlier
neurons. There is no randomness and no parallel-update interpretation.

## Corpus

The canonical corpus is
`code/specs/fixtures/hopfield-associative-memory-v1`. Each JSON document is
strict: duplicate keys, unknown keys, non-finite numbers, non-bipolar states,
non-permutation update orders, asymmetric weights, nonzero diagonals, and
mismatched traces are errors.

## Cross-Language and Rust-Core Direction

Every host language should first implement this four-neuron loop directly and
compare its full trace with the fixture. A future Rust core can batch outer
products, energy evaluation, overlap, and deterministic recall sweeps. Stable
bindings should accept flat bipolar buffers plus an explicit update-order
buffer, expose a C ABI for native languages, and reuse that ABI from WASM and
higher-level language adapters. Trace mode must remain available beside any
optimized fixed-point-only path.
