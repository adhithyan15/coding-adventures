## 0.260.0 - 2026-08-27 (ALGOL nested checked integer exponents)

The ALGOL matrix now proves nested bounded power operands inside checked
integer exponent expressions on all seven standard backends. The expression
retains the integer unrolled-power path and preserves an integer selector;
unsafe or unsupported forms remain conservative.

