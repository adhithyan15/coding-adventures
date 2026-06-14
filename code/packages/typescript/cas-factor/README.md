# cas-factor

Pure TypeScript integer polynomial factoring for CAS packages.

Polynomials are coefficient lists in ascending degree order. For example,
`[-1n, 0n, 1n]` represents `x^2 - 1`.

The package exposes content extraction, integer-root factoring, Kronecker
splitting, bounded monic `bzhFactor` fallback factoring, and
`factorIntegerPolynomial`.
