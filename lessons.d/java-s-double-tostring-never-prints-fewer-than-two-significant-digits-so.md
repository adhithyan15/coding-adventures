---
category: Compiler / VM / language pipeline
---

# Java's Double.toString never prints fewer than two significant digits, so the deepest subnormals differ from Python and JavaScript

JDK 19+'s `Double.toString` (Giulietti's algorithm) prints the shortest
round-trip decimal **among those with at least two digits**. When the true
shortest has one digit and is inexact, Java prints the closest two-digit one
instead. That only happens for the deepest subnormals: `Double.MIN_VALUE` is
`4.9E-324` and `1e-323` is `9.9E-324`, where Python and JavaScript print
`5e-324` and `1e-323`.

LANG83 PR6a's JVM float writer re-laid out `Double.toString`'s digits, and a
real-JVM test against a `python3`-generated table caught the difference on its
first run.

What to do: when re-using a host formatter as a "shortest digits" source,
check its documented minimum length, not just its rounding rule, and check
which runtime versions actually have that rounding rule. `Double.toString` is
shortest only on JDK 19+ (see the lesson on CI's preinstalled Windows JDK). The
JVM helper therefore no longer uses it. It searches
`new BigDecimal(|x|).round(MathContext(p, FLOOR | CEILING))` for p = 1, 2, …
until one reads back, which handles one-digit results like `5e-324` with no
special case. Include subnormals like `5e-324` in every float-output table.
