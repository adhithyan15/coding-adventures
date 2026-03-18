# coding-adventures-fp-arithmetic

IEEE 754 floating-point arithmetic built entirely from logic gates. This is the
shared foundation for both the CPU stack (FPU) and all three accelerator stacks
(GPU/TPU/NPU) in the coding-adventures project.

## Layer position

```
Layer 11: Logic Gates (AND, OR, XOR, NAND)
    |
Layer 10: FP Arithmetic  <-- THIS PACKAGE
    |
    +---> CPU: FPU in cpu-simulator
    +---> GPU: CUDA core (FP32 ALU)
    +---> TPU: Processing element (MAC unit)
    +---> NPU: MAC unit
```

## Supported formats

| Format | Bits | Exponent | Mantissa | Bias | Use case |
|--------|------|----------|----------|------|----------|
| FP32   | 32   | 8        | 23       | 127  | CPU, GPU default |
| FP16   | 16   | 5        | 10       | 15   | GPU mixed precision |
| BF16   | 16   | 8        | 7        | 127  | TPU native, ML training |

## Usage

```python
from fp_arithmetic import float_to_bits, bits_to_float, fp_add, fp_mul, fp_fma, FP32

# Encode a Python float to IEEE 754 bits
a = float_to_bits(3.14, FP32)
b = float_to_bits(2.71, FP32)

# Add using logic gates
result = fp_add(a, b)
print(bits_to_float(result))  # ~5.85

# Multiply
product = fp_mul(a, b)
print(bits_to_float(product))  # ~8.5094

# Fused multiply-add: a * b + c with single rounding
c = float_to_bits(1.0, FP32)
fma_result = fp_fma(a, b, c)
print(bits_to_float(fma_result))  # ~9.5094
```

## Dependencies

None -- this package is fully self-contained. It includes internal reimplementations
of the logic gate primitives (AND, OR, XOR, NOT) and ripple carry adder that
conceptually come from the lower layers of the computing stack. This makes
fp-arithmetic usable without pulling in transitive dependencies from either
the CPU or accelerator paths.

## Development

```bash
uv venv --clear --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --cov=fp_arithmetic --cov-report=term-missing
```
