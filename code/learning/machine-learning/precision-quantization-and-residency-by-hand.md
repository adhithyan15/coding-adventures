# Precision, Quantization, and Buffer Residency, by Hand

A neural network still performs ordinary arithmetic when its numbers become
smaller or its buffers move to an accelerator. What changes is the set of
numbers the machine can store and how often those stored numbers travel.

This lab uses one neuron:

```text
x = [1.0004, 1.0006]
w = 2
b = 0
y = x * w + b
```

With unlimited paper precision, the answers are `2.0008` and `2.0012`.

## Precision Is a Grid

A floating-point format stores only selected points on the number line. Near
`1`, consecutive binary16 values are `1` and `1.0009765625`. Therefore:

```text
binary16(1.0004) = 1
binary16(1.0006) = 1.0009765625
```

Multiplying the rounded values by `2` gives:

```text
1 * 2            = 2
1.0009765625 * 2 = 2.001953125
```

The maximum difference from the paper answers is `0.0008`. Binary32 has a
finer grid, so its maximum difference is only about `0.0000001057` here.

Precision is not a label painted onto an exact number. It changes which number
enters the next operation.

## Quantization Is a Chosen Integer Grid

Symmetric int8 quantization chooses a scale and stores:

```text
q = round_ties_to_even(real / scale)
real-ish = q * scale
```

Use input scale `0.01` and weight scale `0.5`:

```text
round(1.0004 / 0.01) = 100
round(1.0006 / 0.01) = 100
round(2 / 0.5)       = 4
```

Both inputs collapse to the same integer. The integer multiply is `100 * 4 =
400`, and dequantization gives `400 * 0.01 * 0.5 = 2`. The largest output error
is `0.0012`.

The operands use one byte each, but the product accumulates in a four-byte
signed int32 lane. The value `400` would not fit in int8; small operands do not
mean every intermediate is also int8.

The scale and rounding rule are part of the model contract. This fixture uses
round-to-nearest with ties going to the even integer, matching IEEE arithmetic.
Bytes without that metadata do not tell you the original real values.

## Residency Is About Travel

Suppose the binary32 graph runs three times. The inputs plus weight and bias
occupy 16 bytes; the two outputs occupy 8 bytes.

An eager schedule uploads and downloads every time:

```text
(16 upload bytes + 8 download bytes) * 3 = 72 bytes
```

A resident schedule uploads once, keeps the live buffers on the device, and
downloads the final answer once:

```text
16 upload bytes + 8 download bytes = 24 bytes
```

Both schedules calculate the same outputs. The second schedule simply crosses
the boundary less often.

## What the Visualizer Lets You Change

Open the **Precision + Residency** workbench and:

1. switch among binary32, binary16, and symmetric int8;
2. inspect the encoded inputs, accumulator, output, and absolute error;
3. choose eager or resident buffers;
4. change the repeat count and watch the transfer equation update.

The checked-in fixture uses three repeats. Other repeat counts are visual
experiments derived from the same pinned **binary32** byte sizes, independent
of the selected arithmetic format.

## Cross-Language Recipe

A new language consumer should replay the experiment in this order:

1. parse the JSON without accepting duplicate keys or non-finite numbers;
2. decode the raw little-endian binary32 and binary16 payloads;
3. decode signed int8 bytes and apply the pinned scales;
4. reproduce every output, error, and transfer count;
5. only then replace native arithmetic with a Rust-core binding.

That order distinguishes a language adapter bug from a Rust-kernel bug. A
future C ABI should expose explicit buffer ownership and download operations;
otherwise a binding can silently erase the residency advantage.

## What to Remember

- Lower precision means a coarser representable-number grid.
- Quantization needs both integer bytes and scale metadata.
- Equal outputs do not imply equal transfer cost.
- Fewer bytes or copies are hypotheses about speed, not benchmark results.
