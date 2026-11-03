---
category: Testing & coverage
---

# An evaluation-order test needs an operand that changes what another operand reads, or every order prints the same answer

The first #18482 test for "`x += e` reads `x` before evaluating `e`" was
`x = 3; x += x * 2`.  It printed 9 under either order, because `x * 2`
does not change `x`, so it could never fail.  The security review replaced
it with `self.n += self.bump()`, where `bump` sets `self.n` to 100 and
returns 5: CPython prints 6, the other order 105.  That program found a real
bug at once: the C backend rendered both operands as C call arguments,
whose evaluation order C leaves unspecified, and gcc ran `bump()` first.

What to do: an ordering test needs an operand with a side effect on what
another operand reads, and an expected value that differs under every
other order.  Check it would fail by evaluating the alternative order by
hand before trusting it.
