# polynomial (C++)

A pure ISO **C++17**, header-only library for polynomial arithmetic over the
reals, in namespace `ca::polynomial`. A faithful port of the Rust `polynomial`
crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only — no libm.

## Representation

`ca::polynomial::poly` is `std::vector<double>` in little-endian order (`p[i]` is
the coefficient of `xⁱ`); the empty vector is the zero polynomial. Trailing
near-zero coefficients are stripped by `normalize`.

## API

```cpp
#include "polynomial.hpp"
namespace poly = ca::polynomial;

poly::poly a = {1, 2, 3};   // 1 + 2x + 3x^2
poly::poly b = {2, 1};      // 2 + x

auto product = poly::multiply(a, b);
double v     = poly::evaluate(a, 2.0);        // 17
auto qr      = poly::divmod(a, b);            // {quotient, remainder}
auto g       = poly::gcd(a, b);
```

`normalize`, `degree`, `add`, `subtract`, `multiply`, `divmod` (→
`std::pair<poly, poly>`), `divide`, `modulo`, `evaluate`, `gcd`, plus `zero()`
and `one()`. Division by the zero polynomial throws `std::invalid_argument` (the
crate panics).

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use integer coefficients (exact results) and check arithmetic identities,
long-division reconstruction, Horner evaluation, and GCD.
