# NN13: Attention Softmax and Causal-Mask Labs

## Status

Draft specification for deterministic, language-neutral stable-softmax,
causal-mask, and weighted-value traces.

## Purpose

NN13 continues the three-token NN12 example. It turns each row of scaled
query-key scores into weights and uses those weights to mix value vectors. A
causal version prevents a token from reading keys to its right.

## Input Contract

V1 takes three token identifiers, a `3 x 3` scaled score matrix, and three
two-number value rows. The checked-in lab reuses NN12's values and scores, but
the fixture is standalone so consumers do not need to load another document.

## Stable Softmax Contract

For each query row, first remove disallowed key positions. Never encode their
conceptual negative infinity as a JSON non-finite number; trace them as `null`.
Then compute:

```text
row_max = max(allowed scores)
shifted[j] = score[j] - row_max
exponential[j] = exp(shifted[j])
denominator = sum(exponential)
weight[j] = exponential[j] / denominator
```

Masked positions have `null` shifted scores, zero exponentials, and zero
weights. Subtracting the row maximum is numerically important even though it
does not change the normalized result.

## Causal-Mask Contract

Tokens are ordered left to right. Query row `i` may read key column `j` only
when:

```text
j <= i
```

The first token can read only itself. The second can read the first two. The
third can read all three. An unmasked trace is also included as a controlled
comparison.

## Value-Mixing Contract

Every output context row is the weighted sum of value rows:

```text
contribution[j] = weight[j] * value[j]
context = sum(contribution rows)
```

For the causal blue query, the future purple key is blocked. The two remaining
scores are equal, so the weights are `[0.5, 0.5, 0]` and the context is
`[1, 0.5]`.

## Fixture Layout

```text
code/specs/fixtures/attention-softmax-v1/
  schema.json
  labs/00-three-token-causal-softmax.json
```

Consumers reject duplicate keys, non-finite numbers, unknown fields, incorrect
shapes, unsupported operations, and trace values outside
`absolute_tolerance`.

## Conformance Levels

1. **Mask:** reproduce every allowed position and `null` masked score.
2. **Softmax:** reproduce row maxima, shifts, exponentials, denominators, and
   weights for masked and unmasked modes.
3. **Mix:** reproduce every weighted value contribution and output context.
4. **Trace:** retain token identities so a learner can connect a weight to its
   query row, key column, and value payload.

## Cross-Language and Rust-Core Direction

Every language should implement the three-row scalar loop first. A Rust core
can later expose stable masked-softmax and weighted-reduction kernels without
changing the fixture semantics.

A future C ABI should pass explicit row/column sizes, strides, a mask buffer,
score and value buffers, and caller-owned weight/context outputs. Trace mode
should optionally return row maxima and normalization denominators. An
implementation may fuse softmax and value reduction internally, but it must
still offer reproducible weights and trace intermediates for teaching and
cross-language parity.
