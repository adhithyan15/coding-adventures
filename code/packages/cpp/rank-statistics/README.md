# ca::rank_statistics

A dependency-free C++17 header library for sample medians and exact
distribution-free confidence intervals for a population median.

```cpp
#include <ca/rank_statistics.hpp>
#include <vector>

const auto estimate = ca::rank_statistics::median({8.0, 2.0, 4.0});
const auto interval = ca::rank_statistics::median_interval({2.0, 4.0, 8.0}, 95);
```

Inputs must be non-empty and finite. The median does not mutate its input and
uses an overflow-safe midpoint. Confidence is an integer from 1 through 99;
the exact binomial cutoff is computed with private small-limb arithmetic. The
interval operation accepts at most 10,000 observations to bound work; this
is an explicit C++ profile cap. The cross-language collector's reports are
already limited to that size, while the median operation itself is uncapped.
No external arbitrary-precision library is required.

Install with CMake as `ca::rank_statistics`, or include the header directly.
See [VER07h](../../../specs/VER07h-cpp-microbenchmarks.md) for its role in the
paired benchmark lane.
