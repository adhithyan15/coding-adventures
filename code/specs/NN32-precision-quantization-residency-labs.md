# NN32: Precision, Quantization, and Buffer Residency Labs

## Status

Version 1 is implemented by the language-neutral fixture under
[`fixtures/precision-residency-v1`](./fixtures/precision-residency-v1/README.md).

## Purpose

NN32 separates three performance choices that are often bundled together:

1. precision chooses how many bits represent a number;
2. quantization maps real numbers onto a small integer grid;
3. residency chooses whether buffers cross a device boundary again.

The V1 lab keeps the graph fixed at `y = x * 2`. Its inputs, `1.0004` and
`1.0006`, are easy to multiply but close enough to reveal rounding.

## Canonical Results

| format | encoded inputs | outputs | maximum absolute error |
| --- | --- | --- | --- |
| binary32 | `1.000399947...`, `1.000599980...` | `2.000799894...`, `2.001199960...` | `1.0567e-7` |
| binary16 | `1`, `1.0009765625` | `2`, `2.001953125` | `0.0008` |
| symmetric int8 | `100`, `100` | `2`, `2` | `0.0012` |

The int8 path uses `input_scale = 0.01`, `weight_scale = 0.5`, and zero point
`0`. Both close inputs become `100`; the weight becomes `4`; the integer
accumulator is `400`; and `400 * 0.01 * 0.5 = 2`.
Conversion uses round-to-nearest with ties going to the even integer.
Operands occupy one byte each, but multiplication accumulates in a four-byte
signed int32 lane so values such as `400` do not overflow int8.

## Residency Result

For binary32, `x`, `w`, and `b` occupy 16 bytes and `y` occupies 8 bytes.
Across three repeats:

- eager copies move `(16 + 8) * 3 = 72` bytes;
- resident buffers move `16 + 8 = 24` bytes.

The fixture counts transfers rather than promising speed. A real benchmark is
hardware-, driver-, workload-, and warmup-dependent.

## Fixture Contract

The closed V1 directory pins one lab JSON document and six raw payloads. The
validator recomputes all rounded values, quantized integers, accumulators,
errors, payload bytes, and transfer totals rather than trusting copied numbers.

## Cross-Language and Rust-Core Direction

Every language can implement the reference formulas first and replay the same
raw bytes. A Rust core can later expose binary16 conversion, int8 kernels, and
resident execution handles behind the planned stable C ABI. Bindings must keep
the format scales, byte order, buffer ownership, and explicit final download in
their contract. NN32 does not claim those kernels or that C ABI already exist.

## Non-Goals

- Claiming that lower precision is always accurate enough.
- Claiming resident buffers or int8 are always faster.
- Mixed-precision training, calibration datasets, or per-channel scales.
- Hiding copies made for a debugging trace.
