# Changelog

## 0.1.0

- Normalize integer samples to finite binary64 before sorting and reject integer conversion overflow.
- Add sample median and exact median confidence interval functions.
- Handle subnormal midpoints and maximum finite values without lost rounding or overflow.
- Include the shared paired-observation fixture in package conformance tests.
- Declare the pure in-memory runtime capability profile and support the pinned Elixir 1.18.4 compatibility target.
