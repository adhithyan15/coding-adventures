# skip-list (C++)

An **ordered map** with skip-list-style reported parameters, in pure ISO C++17
(header-only). A faithful port of the Rust `skip-list` crate (internally a
`std::map`-backed ordered map).

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "skip_list.hpp"

ca::skip_list<int, int> s;
s.insert(5, 50);
s.insert(1, 10);

std::optional<int> v = s.search(5);          // 50
std::optional<std::size_t> r = s.rank(5);    // 1
auto in = s.range(1, 5, /*inclusive=*/true); // sorted vector<pair>
s.erase(5);
```

Order statistics (`rank`/`by_rank`), `min`/`max`, ordered `entries`, and range
queries; `current_max` is `ceil(log_{1/p}(n))` clamped to `[1, max_level]`,
computed without `<cmath>`.

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/skip-list`. See also the [C port](../../c/skip-list/README.md).
