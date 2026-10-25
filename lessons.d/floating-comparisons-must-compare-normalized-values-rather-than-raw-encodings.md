---
category: Compiler / VM / language pipeline
---

# Floating comparisons must compare normalized values rather than raw encodings

System/360 hexadecimal floating-point loads preserve raw encodings, including
unnormalized fractions. Comparing exponent and fraction fields lexicographically
therefore reported numerically equal values such as `0x4110...` and `0x4201...`
as unequal.

Value operations must not assume that transport has canonicalized the input.
Before comparison, handle signed zero explicitly and normalize both nonzero
magnitudes to a common leading hexadecimal digit, adjusting their exponents in
lockstep. Compare the normalized exponent/fraction keys and reverse the result
for negative operands.
