# Precision and Buffer Residency Fixture V1

This closed fixture gives every language the same tiny NN32 experiment:

```text
x = [1.0004, 1.0006]
w = 2
b = 0
y = x * w + b
```

The two inputs are deliberately closer than one binary16 step near `1`. The
fixture records the rounded binary32 and binary16 values, a symmetric signed
int8 encoding using ties-to-even rounding and an int32 accumulator, and the
resulting output error. Raw payloads use little-endian
IEEE-754 bytes or signed two's-complement bytes so a consumer does not need a
language-specific JSON floating-point convention.

The residency half replays the same binary32 forward pass three times. The
eager schedule transfers 72 bytes; the resident schedule transfers 24 bytes.
This is a deterministic byte count, not a timing benchmark.

Validate it with:

```text
python code/scripts/validate_precision_residency_labs.py
pytest code/scripts/tests/test_precision_residency_labs.py -q
```

Consumers must reject unknown fields, duplicate JSON keys, non-finite or
unbounded values, path traversal, malformed payloads, changed quantization
parameters, dishonest output or error oracles, and incorrect transfer totals.
