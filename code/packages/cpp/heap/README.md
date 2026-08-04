# heap (C++)

A **binary heap (priority queue)**, in pure ISO C++17 (header-only). A faithful
port of the Rust `heap` crate: `ca::min_heap<T>` / `ca::max_heap<T>`, plus the
`heap_sort`, `nlargest`, and `nsmallest` helpers.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "heap.hpp"

ca::min_heap<int> h;
h.push(5); h.push(1); h.push(3);
int top = h.peek().value();          // 1
std::optional<int> v = h.pop();      // 1

auto sorted = ca::heap_sort(std::vector<int>{3, 1, 4, 1});  // {1,1,3,4}
auto top3   = ca::nlargest(std::vector<int>{5,1,8,3,9}, 3); // {9,8,5}
```

`min_heap`/`max_heap` are aliases of a shared `binary_heap<T, Compare>`;
`pop`/`peek` return `std::optional<T>` (empty ⇒ `std::nullopt`). Bulk
construction from a `std::vector<T>` heapifies in O(n).

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/heap`. See also the [C port](../../c/heap/README.md).
