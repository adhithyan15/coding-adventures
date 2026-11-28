---
category: Security boundaries
---

# Public signed checksum helpers must reject overflow in every traversal path

The indexed-array benchmark's fixed positive values cannot overflow, but the
public batch helpers accept any caller-provided signed 64-bit array. A custom
array containing `INT64_MAX, 1` or `INT64_MIN, -1` made plain `sum += value`
invoke signed-overflow undefined behavior. Both indexed and iterator helpers
must use the same checked signed addition for per-operation and accumulated
checksums, and report overflow with `std::overflow_error`.

Test the helpers directly with mutated arrays, both overflow signs, exact
`INT64_MIN`/`INT64_MAX`, multi-operation accumulation, and a cancellation case
that crosses neither bound. Document that the symmetric checks are part of
callback cost, since these helpers can also run inside a timed benchmark.
