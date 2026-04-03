# coding_adventures_polynomial (Ruby)

Polynomial arithmetic over real numbers. Polynomials are represented as
frozen arrays where index = degree:

```ruby
[3, 0, 2]  # 3 + 0·x + 2·x²
```

## Usage

```ruby
require_relative "lib/polynomial"

Polynomial.add([1, 2, 3], [4, 5])     # → [5, 7, 3]
Polynomial.multiply([1, 2], [3, 4])   # → [3, 10, 8]
Polynomial.evaluate([3, 1, 2], 4)     # → 39

q, r = Polynomial.divmod_poly([5, 1, 3, 2], [2, 1])
# q = [3.0, -1.0, 2.0], r = [-1.0]
```

## API

- `normalize(p)`, `degree(p)`, `zero`, `one`
- `add(a, b)`, `subtract(a, b)`, `multiply(a, b)`
- `divmod_poly(a, b)` → `[quotient, remainder]`
- `divide(a, b)`, `mod(a, b)`
- `evaluate(p, x)` — Horner's method
- `gcd(a, b)` — Euclidean algorithm
