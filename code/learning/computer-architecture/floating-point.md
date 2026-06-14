# Floating-Point Arithmetic — How Computers Handle Real Numbers

Integers are straightforward: the number 42 is stored as the binary pattern
`101010`. But what about 3.14? Or 0.000001? Or 6.022 x 10^23? Computers
handle these with **floating-point** representation, standardized by
**IEEE 754** in 1985.

This document explains the IEEE 754 format, special values, rounding, and
the infamous `0.1 + 0.2 != 0.3` problem.

Reference implementation: `code/packages/python/fp-arithmetic/`

---

## Table of Contents

1. [The Idea: Scientific Notation in Binary](#1-the-idea-scientific-notation-in-binary)
2. [IEEE 754 Format — Sign, Exponent, Mantissa](#2-ieee-754-format--sign-exponent-mantissa)
3. [Encoding a Float Step by Step](#3-encoding-a-float-step-by-step)
4. [The Three Formats: FP32, FP16, BF16](#4-the-three-formats-fp32-fp16-bf16)
5. [Special Values](#5-special-values)
6. [Rounding Modes](#6-rounding-modes)
7. [Why 0.1 + 0.2 != 0.3](#7-why-01--02--03)
8. [The Python Implementation](#8-the-python-implementation)

---

## 1. The Idea: Scientific Notation in Binary

In decimal, scientists write large and small numbers using scientific
notation:

```
    6,022,000,000,000,000,000,000,000 = 6.022 x 10^23
    0.000000001 = 1.0 x 10^-9
```

The key idea: express any number as:

```
    sign  x  significand  x  base^exponent
```

IEEE 754 does the same thing, but in binary:

```
    Decimal scientific notation:  -6.022  x  10^23
    IEEE 754 (binary):            (-1)^s  x  1.mantissa  x  2^(exp - bias)
```

The "floating" in floating-point refers to the decimal (binary) point
**floating** to different positions depending on the exponent, allowing the
same number of bits to represent both very large and very tiny numbers.

---

## 2. IEEE 754 Format — Sign, Exponent, Mantissa

Every IEEE 754 number is stored as three bit fields packed into a fixed-width
word:

```
    +------+----------+------------------------+
    | sign | exponent |       mantissa         |
    +------+----------+------------------------+
    1 bit    E bits         M bits

    Total bits = 1 + E + M
```

### Sign Bit (1 bit)

```
    0 = positive
    1 = negative
```

Simple. One bit determines the sign.

### Exponent Field (E bits)

The exponent is stored with a **bias** — a constant added to the true
exponent so that the stored value is always non-negative.

```
    stored_exponent = true_exponent + bias

    For FP32: bias = 127
        true_exponent = 0   ->  stored = 127  (01111111 in binary)
        true_exponent = 1   ->  stored = 128  (10000000 in binary)
        true_exponent = -1  ->  stored = 126  (01111110 in binary)
```

Why use a bias instead of two's complement? Because biased exponents make
comparison easier — you can compare two floating-point numbers by comparing
their bit patterns as unsigned integers (for numbers with the same sign).

### Mantissa Field (M bits)

The mantissa stores the fractional part of the significand. For **normal**
numbers, there is an **implicit leading 1** that is not stored:

```
    Stored mantissa bits:  [1, 0, 1, 0, 0, ...]
    Actual significand:    1.10100...
                           ^-- implicit 1, not stored (free bit of precision!)
```

This trick gives us one extra bit of precision for free. The only exception
is **denormalized** numbers (exponent = all zeros), where the implicit bit
is 0 instead of 1.

### The Complete Layout (FP32)

```
    Bit 31:     Sign
    Bits 30-23: Exponent (8 bits, bias = 127)
    Bits 22-0:  Mantissa (23 bits, + 1 implicit = 24 bits of precision)

    +---+----------+---------------------------+
    | S | EEEEEEEE | MMMMMMMMMMMMMMMMMMMMMMM   |
    +---+----------+---------------------------+
     31   30    23    22                     0
```

### The Value Formula

```
    For normal numbers (exponent != 0 and exponent != all-1s):

        value = (-1)^sign  x  1.mantissa  x  2^(exponent - bias)

    For denormalized numbers (exponent = 0, mantissa != 0):

        value = (-1)^sign  x  0.mantissa  x  2^(1 - bias)
```

---

## 3. Encoding a Float Step by Step

Let's encode **-6.75** as FP32.

### Step 1: Determine the sign

```
    -6.75 is negative, so sign = 1
```

### Step 2: Convert the absolute value to binary

```
    6 in binary: 110
    0.75 in binary: 0.11

    How to convert 0.75:
        0.75 x 2 = 1.50  -> 1 (take the integer part)
        0.50 x 2 = 1.00  -> 1
        0.00 x 2 = 0.00  -> done

    So 0.75 = 0.11 in binary

    6.75 = 110.11 in binary
```

### Step 3: Normalize (scientific notation in binary)

```
    110.11 = 1.1011 x 2^2

    The binary point floats left until there's exactly one 1 before it.
    We moved it 2 places, so the exponent is 2.
```

### Step 4: Compute the stored exponent

```
    true_exponent = 2
    stored_exponent = 2 + 127 = 129 = 10000001 in binary
```

### Step 5: Extract the mantissa

```
    Significand: 1.1011
    Mantissa (without the implicit 1): 1011

    Pad to 23 bits: 10110000000000000000000
```

### Step 6: Assemble the fields

```
    Sign:     1
    Exponent: 10000001
    Mantissa: 10110000000000000000000

    Complete FP32: 1 10000001 10110000000000000000000
                   = 0xC0D80000
```

### Decoding Back

```
    Sign = 1 -> negative
    Exponent = 10000001 = 129 -> true exp = 129 - 127 = 2
    Mantissa = 1011... -> significand = 1.1011

    Value = -1 x 1.1011 x 2^2
          = -1 x 1.6875 x 4
          = -6.75
```

---

## 4. The Three Formats: FP32, FP16, BF16

```
    Format | Total | Sign | Exp  | Mantissa | Bias | Precision     | Range
    -------+-------+------+------+----------+------+---------------+------------------
    FP32   |  32   |  1   |  8   |   23     | 127  | ~7 digits     | ~1.2e-38 to 3.4e38
    FP16   |  16   |  1   |  5   |   10     |  15  | ~3-4 digits   | ~6.0e-8 to 65504
    BF16   |  16   |  1   |  8   |    7     | 127  | ~2-3 digits   | same as FP32
```

### FP32 (Single Precision) — The Workhorse

```
    +---+----------+---------------------------+
    | S | EEEEEEEE | MMMMMMMMMMMMMMMMMMMMMMM   |
    +---+----------+---------------------------+
     1      8                  23                  = 32 bits
```

Used by CPUs, GPUs, and as the default for most computation. ~7 decimal
digits of precision.

### FP16 (Half Precision) — GPU Training

```
    +---+-------+------------+
    | S | EEEEE | MMMMMMMMMM |
    +---+-------+------------+
     1     5         10          = 16 bits
```

Half the size of FP32. Used in GPU mixed-precision training to save memory
and bandwidth. Range is limited (~65504 max), but often sufficient for
neural network weights and activations.

### BF16 (Brain Float) — TPU Native

```
    +---+----------+---------+
    | S | EEEEEEEE | MMMMMMM |
    +---+----------+---------+
     1      8           7        = 16 bits
```

Invented by Google for TPU hardware. Has the same exponent size (and
therefore the same range) as FP32, but only 7 mantissa bits (vs 23).

**Why BF16 exists:** Machine learning needs range (gradients can be huge or
tiny) more than precision. BF16 gives you FP32's range in half the bits.
Conversion from FP32 to BF16 is trivial: just truncate the lower 16 bits.

### Comparison: Precision vs Range

```
    Value                | FP32       | FP16       | BF16
    ---------------------+------------+------------+-----------
    1.0                  | exact      | exact      | exact
    0.1                  | 0.10000000 | 0.09997559 | 0.1015625
    3.14159              | 3.1415927  | 3.140625   | 3.140625
    65536                | 65536.0    | Infinity!  | 65536.0
    1.0e-40              | 1.0e-40    | 0.0!       | 1.0e-40
```

Notice:
- FP16 overflows at 65536 (exponent too small), BF16 handles it fine
- FP16 underflows at 1e-40, BF16 handles it fine (same exponent as FP32)
- BF16 has less precision than FP16 for small numbers near 1.0

---

## 5. Special Values

IEEE 754 reserves certain bit patterns for special values. This is one of
its most elegant features — operations that would crash in integer
arithmetic produce well-defined results instead.

### +0 and -0

```
    +0:  sign=0, exponent=00000000, mantissa=00000000000000000000000
    -0:  sign=1, exponent=00000000, mantissa=00000000000000000000000
```

Both exponent and mantissa are all zeros. The sign bit distinguishes them.

```
    +0 == -0  is True in Python (and IEEE 754)

    But they're different bit patterns, and they matter:
        1.0 / (+0) = +Infinity
        1.0 / (-0) = -Infinity
```

Having -0 preserves sign information through operations. For example,
a very small negative number that underflows to zero becomes -0, preserving
the fact that it was negative.

### Infinity (+Inf, -Inf)

```
    +Inf:  sign=0, exponent=11111111, mantissa=00000000000000000000000
    -Inf:  sign=1, exponent=11111111, mantissa=00000000000000000000000
```

Exponent is all 1s, mantissa is all 0s.

```
    When does Infinity appear?

    1.0 / 0.0        = +Inf
    -1.0 / 0.0       = -Inf
    1e38 * 10         = +Inf   (overflow)
    Inf + 1           = +Inf   (absorbs finite values)
    Inf + Inf         = +Inf
    Inf * 2           = +Inf

    Inf - Inf         = NaN    (indeterminate)
    Inf / Inf         = NaN    (indeterminate)
    0 * Inf           = NaN    (indeterminate)
```

### NaN (Not a Number)

```
    NaN:  sign=0, exponent=11111111, mantissa=10000000000000000000000
                                              ^-- at least one bit set
```

Exponent is all 1s, mantissa is **non-zero**.

```
    When does NaN appear?

    0.0 / 0.0        = NaN    (undefined)
    Inf - Inf         = NaN    (indeterminate)
    sqrt(-1.0)        = NaN    (imaginary number in reals)
    NaN + anything    = NaN    (NaN propagates through all operations)
    NaN == NaN        = False  (NaN is not equal to anything, including itself!)
```

The NaN propagation rule is important: once NaN enters a computation, every
subsequent result is also NaN. This acts as a "poison" value that signals
something went wrong upstream.

### Denormalized Numbers (Subnormals)

```
    Denorm:  sign=0/1, exponent=00000000, mantissa=non-zero
```

Exponent is all 0s, mantissa is non-zero.

Normal numbers have an implicit leading 1 (significand = 1.mantissa).
Denormalized numbers have an implicit leading **0** (significand = 0.mantissa).
This allows representation of numbers very close to zero.

```
    Without denormals:
    Smallest positive normal FP32: 1.0 x 2^(-126) = ~1.18e-38
    Next smaller value: 0 (sudden jump!)

    With denormals:
    Smallest positive normal: 1.0 x 2^(-126)     = ~1.18e-38
    Largest denormal:         0.111...1 x 2^(-126) = ~1.18e-38 (just below normal)
    Smallest denormal:        0.000...1 x 2^(-126) = ~1.4e-45

    The denormals fill the gap between 0 and the smallest normal number,
    providing "gradual underflow" instead of a sudden jump to zero.
```

Visualized on a number line:

```
    0    denormals          smallest normal              ...
    |.....................|===========================>
    ^                     ^
    0.0                   ~1.18e-38

    Without denormals, everything between 0 and 1.18e-38 would be
    rounded to 0 (the "underflow gap").
```

### Special Values Summary

```
    Exponent   | Mantissa  | Value
    -----------+-----------+------------------
    All 0s     | All 0s    | +/- Zero
    All 0s     | Non-zero  | Denormalized number
    Normal     | Any       | Normal number
    All 1s     | All 0s    | +/- Infinity
    All 1s     | Non-zero  | NaN
```

---

## 6. Rounding Modes

Since floating-point has finite precision, most real numbers cannot be
represented exactly. The result of an operation must be **rounded** to the
nearest representable value.

IEEE 754 defines four rounding modes:

### Round to Nearest, Ties to Even (Default)

Round to the nearest representable value. When exactly halfway between two
values, round to the one with an even (0) least significant bit.

```
    Example (pretend we have 3-bit mantissa):

    Exact result: 1.0101  (halfway between 1.010 and 1.011)
    Round to even: 1.010  (ends in 0, which is even)

    Exact result: 1.0111  (closer to 1.100)
    Round: 1.100

    Exact result: 1.1101  (halfway between 1.110 and 1.111)
    Round to even: 1.110  (ends in 0, which is even)
```

Why "ties to even"? It prevents systematic upward bias. If you always
rounded up on ties, the average error would accumulate upward over many
operations. Rounding to even keeps the average error at zero.

### Round Toward Zero (Truncation)

Simply discard the extra bits. Always rounds toward zero.

```
    1.0101 -> 1.010  (truncated)
    -1.0101 -> -1.010  (truncated, toward zero)
```

### Round Toward +Infinity (Ceiling)

Round up (toward positive infinity).

```
    1.0101 -> 1.011  (rounded up)
    -1.0101 -> -1.010  (rounded up, i.e., toward zero for negatives)
```

### Round Toward -Infinity (Floor)

Round down (toward negative infinity).

```
    1.0101 -> 1.010  (rounded down)
    -1.0101 -> -1.011  (rounded down, i.e., away from zero for negatives)
```

### The Round, Guard, and Sticky Bits

When performing arithmetic, the hardware maintains three extra bits beyond
the mantissa width to ensure correct rounding:

```
    Mantissa:     M M M M M M M M M M M M M M M M M M M M M M M
    Extra bits:                                                   G R S
                                                                  ^ ^ ^
                                                Guard  Round  Sticky

    G = Guard bit (first bit beyond precision)
    R = Round bit (second bit beyond precision)
    S = Sticky bit (OR of all remaining bits)
```

The sticky bit is the OR of ALL bits beyond the guard and round bits. If
any of them is 1, sticky is 1. This tells the rounding logic whether the
exact result was above or below the midpoint.

---

## 7. Why 0.1 + 0.2 != 0.3

This is the most famous floating-point surprise. In Python:

```python
    >>> 0.1 + 0.2
    0.30000000000000004
    >>> 0.1 + 0.2 == 0.3
    False
```

### The Root Cause: Representation Error

The number 0.1 in decimal is a **repeating fraction** in binary, just like
1/3 = 0.333... is a repeating fraction in decimal.

```
    Converting 0.1 to binary:

    0.1 x 2 = 0.2  -> 0
    0.2 x 2 = 0.4  -> 0
    0.4 x 2 = 0.8  -> 0
    0.8 x 2 = 1.6  -> 1
    0.6 x 2 = 1.2  -> 1
    0.2 x 2 = 0.4  -> 0   <- cycle repeats!
    0.4 x 2 = 0.8  -> 0
    0.8 x 2 = 1.6  -> 1
    ...

    0.1 (decimal) = 0.0001100110011001100110011... (binary, repeating forever)
```

Since FP64 has only 52 mantissa bits, the repeating pattern is truncated:

```
    Stored value of 0.1:
    0.1000000000000000055511151231257827021181583404541015625

    Stored value of 0.2:
    0.200000000000000011102230246251565404236316680908203125

    Their sum:
    0.3000000000000000444089209850062616169452667236328125

    Stored value of 0.3:
    0.29999999999999998889776975374843459576368331909179687500
```

The sum of the stored 0.1 and stored 0.2 is slightly larger than the stored
0.3, so they compare as unequal.

### Visualizing the Error

```
    Number line (greatly exaggerated):

    Representable FP64 values near 0.3:
    ...---+-----+-----+-----+-----+---...
          |     |     |     |     |
        0.299  0.300  0.301  ...

    Exact 0.3:              x  (falls here)
    Stored "0.3":      *       (rounded to nearest representable)
    0.1 + 0.2 result:    *    (slightly different representable value)
```

### How to Compare Floats Correctly

Never use `==` for floating-point comparison. Instead, check if the
difference is smaller than a tolerance (epsilon):

```python
    # Wrong:
    if a == b: ...

    # Right:
    if abs(a - b) < 1e-9: ...

    # Or use math.isclose() in Python:
    import math
    if math.isclose(a, b, rel_tol=1e-9): ...
```

### Other Numbers That Can't Be Represented Exactly

Any decimal fraction whose denominator has a factor other than 2 will
repeat in binary:

```
    0.1  = repeating (denominator 10 = 2 x 5, the 5 causes trouble)
    0.2  = repeating
    0.3  = repeating
    0.4  = repeating
    0.5  = exact!     (1/2 = 0.1 in binary)
    0.25 = exact!     (1/4 = 0.01 in binary)
    0.75 = exact!     (3/4 = 0.11 in binary)
    0.6  = repeating
```

The fractions that ARE exact in binary are those expressible as k/2^n:
1/2, 1/4, 3/8, 7/16, etc.

---

## 8. The Python Implementation

The implementation lives at:

```
    code/packages/python/fp-arithmetic/
    |-- src/fp_arithmetic/
    |   |-- formats.py       # FloatFormat (FP32, FP16, BF16), FloatBits
    |   |-- ieee754.py       # float_to_bits(), bits_to_float(), special value detection
    |   |-- fp_adder.py      # Floating-point addition pipeline
    |   |-- fp_multiplier.py # Floating-point multiplication
    |   |-- fma.py           # Fused multiply-add
    |   |-- pipeline.py      # FP pipeline stages
    |   |-- _gates.py        # Local gate imports
```

### FloatFormat and FloatBits

The `FloatFormat` dataclass describes the shape of a format:

```python
    FP32 = FloatFormat(name="fp32", total_bits=32,
                       exponent_bits=8, mantissa_bits=23, bias=127)
    FP16 = FloatFormat(name="fp16", total_bits=16,
                       exponent_bits=5, mantissa_bits=10, bias=15)
    BF16 = FloatFormat(name="bf16", total_bits=16,
                       exponent_bits=8, mantissa_bits=7, bias=127)
```

The `FloatBits` dataclass holds the actual bit pattern:

```python
    @dataclass(frozen=True)
    class FloatBits:
        sign: int              # 0 or 1
        exponent: list[int]    # MSB-first
        mantissa: list[int]    # MSB-first
        fmt: FloatFormat
```

### Encoding and Decoding

`float_to_bits()` converts a Python float to its IEEE 754 bit-level
representation. For FP32, it uses Python's `struct` module to get the
hardware-exact bit pattern:

```python
    packed = struct.pack("!f", value)        # float -> 4 bytes (big-endian)
    int_bits = struct.unpack("!I", packed)[0]  # 4 bytes -> 32-bit unsigned int
    sign = (int_bits >> 31) & 1
    exp = (int_bits >> 23) & 0xFF
    mant = int_bits & 0x7FFFFF
```

For FP16 and BF16, it first encodes as FP32 (using struct), then manually
converts by adjusting the exponent bias and truncating the mantissa with
round-to-nearest-even.

`bits_to_float()` reverses the process. For FP32, it reconstructs the
32-bit integer and uses `struct.unpack("!f", ...)` for exact conversion.
For FP16/BF16, it manually computes the value using the formula.

### Special Value Detection

The module uses logic gates (AND, OR from `logic_gates`) to detect special
values, staying true to the "built from gates" philosophy:

```python
    def is_nan(bits):
        return _all_ones(bits.exponent) and not _all_zeros(bits.mantissa)

    def is_inf(bits):
        return _all_ones(bits.exponent) and _all_zeros(bits.mantissa)

    def is_zero(bits):
        return _all_zeros(bits.exponent) and _all_zeros(bits.mantissa)

    def is_denormalized(bits):
        return _all_zeros(bits.exponent) and not _all_zeros(bits.mantissa)
```

Where `_all_ones()` chains AND gates across all bits, and `_all_zeros()`
chains OR gates and inverts — exactly as hardware would implement these
checks.

### Running the Code

```bash
    cd code/packages/python/fp-arithmetic
    pip install -e .
    python -c "
    from fp_arithmetic.ieee754 import float_to_bits, bits_to_float
    from fp_arithmetic.formats import FP32
    bits = float_to_bits(3.14, FP32)
    print(f'Sign: {bits.sign}')
    print(f'Exponent: {bits.exponent}')
    print(f'Mantissa: {bits.mantissa}')
    print(f'Decoded: {bits_to_float(bits)}')
    "
```
