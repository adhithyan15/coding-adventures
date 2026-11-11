---
category: Testing & coverage
---

# Equal percentile endpoints must preserve the sample value exactly

On macOS CI for PR #19125, a bootstrap confidence interval fell outside the
observed sample bounds. Linear interpolation between two equal floating-point
values can round away from the shared value when each product rounds separately.
A constant sample of `8.335937499999998` produced a lower endpoint of
`8.335937499999996`.

Return the shared value directly when the interpolation endpoints compare
equal. Use a fixed fractional sample to test this property independently of
the clock.
