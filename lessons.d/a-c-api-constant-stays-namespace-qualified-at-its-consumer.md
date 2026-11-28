---
category: Testing & coverage
---

# A C++ API constant stays namespace-qualified at its consumer

A strict C++ test referred to the package's `max_pairs` constant without its
`ca::microbenchmark` namespace. Both GCC and Clang rejected the test before
execution. Keep test and example references explicitly qualified so compiler
errors expose genuine interface problems rather than unimported symbols.
