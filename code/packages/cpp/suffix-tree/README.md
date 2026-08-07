# suffix-tree (C++)

A pure ISO **C++17**, header-only suffix index over a string — a faithful port of
the Rust `suffix-tree` crate (DT15), in namespace `ca`.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## API

```cpp
#include "suffix_tree.hpp"

auto t = ca::suffix_tree::build("banana");
std::vector<std::size_t> pos = t.search("ana");     // {1, 3}
std::size_t n = t.count_occurrences("ana");         // 2
std::string lrs = t.longest_repeated_substring();   // "ana"
std::size_t nodes = t.node_count();                 // 7
std::vector<std::string> suffixes = t.all_suffixes();

std::string lcs = ca::longest_common_substring("xabxac", "abcabxabcd"); // "abxa"
```

`search` and `longest_common_substring` take `std::string_view`. Like the crate,
the "tree" is a simple root-with-one-leaf-per-suffix, so the class is really a
tidy bundle of string algorithms over the stored text.

## Notes

- **Byte-oriented** (like `std::string`): the crate counts Unicode scalar values,
  so results match for ASCII / single-byte text. Offsets are byte offsets.
- The longest-common-substring routine uses a two-row rolling DP.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests are pinned to the crate's own assertions plus empty-pattern,
over-long-pattern, and empty-text edge cases.
