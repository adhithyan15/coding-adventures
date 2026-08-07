# segment-tree (C++)

A **segment tree** with a caller-supplied associative combine operation, in pure
ISO C++17 (header-only). A faithful port of the Rust `segment-tree` crate.
O(log n) range queries and point updates.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "segment_tree.hpp"

auto t = ca::segment_tree<int>::sum_tree({1, 3, 5, 7, 9, 11}); // or min_tree/max_tree
int s = t.query(1, 3);        // 3+5+7 = 15 (inclusive)
t.update(2, 10);
s = t.query(1, 3);            // 20

// Or any custom associative op + identity:
ca::segment_tree<int> g({12, 18, 6}, [](const int& a, const int& b){ return std::gcd(a,b); }, 0);
```

Ranges are **inclusive and 0-based**; out-of-range/inverted queries return the
identity. `T` may be any type with the operation you supply (the `sum`/`min`/
`max` factories require an arithmetic `T`).

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/segment-tree`. See also the
[C port](../../c/segment-tree/README.md).
