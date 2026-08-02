# NN08: Residual Paths and Receptive Fields

## Status

Draft specification for deterministic, language-neutral residual-block and
receptive-field traces.

## Purpose

NN08 explains two ideas that make deep spatial networks easier to reason about:

- stacked convolutions let one output depend on a wider input region; and
- an identity skip gives that output a direct path to its matching input.

The V1 fixture uses five scalar positions, two same-padded three-tap layers,
an identity skip, and ReLU. All weights are one. The center main-path value is
`8`; the skip contributes input value `2`; the residual output is `10`.

## V1 Same-Padded Correlation

For an input with `N` values and the three-tap kernel `[1, 1, 1]`:

```text
correlate(input)[i] = input[i - 1] + input[i] + input[i + 1]
```

Out-of-range values are zero. The kernel is not reversed. V1 applies this
operation twice:

```text
hidden = correlate(input)
main   = correlate(hidden)
```

Because the fixture kernel is symmetric, mathematical convolution and
cross-correlation produce the same numbers. The convention still remains
explicit so later asymmetric fixtures cannot silently flip weights.

## Residual Addition

The identity skip preserves shape and adds the original input position by
position before ReLU:

```text
preactivation[i] = main[i] + input[i]
output[i]        = max(0, preactivation[i])
```

V1 stores the main output, skip contribution, residual sum, and activated
output for every position. This makes it possible to disable the skip in a
visualizer without changing the main path.

## Receptive-Field Trace

For one selected output, V1 stores:

- the hidden positions read by layer two;
- the input positions read by each of those hidden positions;
- how many computational paths connect each input to the output;
- each input value multiplied by its path count; and
- the union of in-range input indices in the receptive field.

For center output `2`, the three hidden positions expand as:

```text
hidden[1] <- input[0, 1, 2]
hidden[2] <- input[1, 2, 3]
hidden[3] <- input[2, 3, 4]
```

The input path counts are `[1, 2, 3, 2, 1]`. Multiplying by input
`[1, 0, 2, 0, 1]` gives `[1, 0, 6, 0, 1]`, which sums to main output `8`.
The structural receptive field is five positions wide even though zero-valued
inputs make some numerical contributions zero.

At boundaries, same padding preserves output length but the in-range receptive
field is clipped. The first and last outputs each reach three actual input
positions; the center reaches all five.

## Fixture Layout

The V1 corpus lives in:

```text
code/specs/fixtures/residual-receptive-v1/
  schema.json
  labs/*.json
```

Validate and execute it with:

```text
python code/scripts/validate_residual_receptive_labs.py
```

## Conformance Levels

- **Main-path conformance:** reproduce both same-padded convolution layers.
- **Residual conformance:** reproduce skip contributions, sums, and ReLU.
- **Path conformance:** reproduce hidden paths, path counts, and numerical input
  contributions for every output.
- **Receptive-field conformance:** reproduce the in-range index union at the
  center and boundaries.
- **Inspectable conformance:** let a learner select an output, toggle the skip,
  and follow both the short identity route and every main-path dependency.

## Native and Rust Direction

The existing Rust `dsp-conv::conv1d` primitive with `BoundaryMode::Zero`
directly reproduces each V1 main-path layer because the kernel is symmetric and
the primitive is centered and same-size. A Rust consumer can run it twice,
then use a small vector loop for identity addition and ReLU. The NN08 fixture
should be the parity oracle for that adapter.

A future neural Rust core should expose same-padded correlation without relying
on symmetry, residual addition, activation, and an optional inspectable trace
over caller-owned contiguous buffers. A stable C ABI should pass pointers,
lengths, kernel sizes, padding modes, and output buffers explicitly. Accelerated
or fused residual blocks remain conformant only if a reference/debug mode can
recover the main path, skip path, and receptive-field metadata pinned here.
