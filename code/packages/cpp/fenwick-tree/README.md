# fenwick-tree (C++)

A **Fenwick tree (Binary Indexed Tree)** over doubles, in pure ISO C++17
(header-only). A faithful port of the Rust `fenwick-tree` crate: O(log n)
`update`/`prefix_sum`, plus `range_sum`, `point_query`, and `find_kth`.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "fenwick_tree.hpp"

ca::fenwick_tree t(std::vector<double>{1, 3, 2, 4});
double s = t.prefix_sum(3);   // 6
t.update(2, 5.0);
s = t.range_sum(1, 2);        // 9
std::size_t k = t.find_kth(5.0);
```

Indexing is **1-based** (`prefix_sum` also accepts `0`). Out-of-range indices,
inverted ranges, and bad `find_kth` targets throw `std::out_of_range` /
`std::invalid_argument` — the idiomatic analogue of the crate's `Result` errors.

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/fenwick-tree`. See also the
[C port](../../c/fenwick-tree/README.md).
