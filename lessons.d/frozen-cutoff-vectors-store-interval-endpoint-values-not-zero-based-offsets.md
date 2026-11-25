---
category: Perl
---

# Frozen cutoff vectors store interval endpoint values, not zero-based offsets

When a differential test vector records the selected interval for the sample
`0 .. n-1`, its pair is already the endpoint values. Compare that pair
directly to the API result; subtracting one a second time turns the expected
values into unrelated array offsets and fails every otherwise-correct case.
