# trie (C++)

A **trie (prefix tree)** mapping string keys to values, in pure ISO C++17
(header-only). A faithful port of the Rust `trie` crate.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "trie.hpp"

ca::trie<int> t;
t.insert("cat", 1);
t.insert("car", 2);

std::optional<int> v = t.search("car");          // 2
bool p = t.starts_with("ca");                     // true
auto words = t.words_with_prefix("ca");           // sorted {(car,2),(cat,1)}
auto m = t.longest_prefix_match("cards");         // optional {("car",2)}
t.erase("car");
```

Children live in a `std::map`, so `all_words`/`keys`/`words_with_prefix`
enumerate in ascending key order (matching the crate's sorted iteration).

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/trie`. See also the [C port](../../c/trie/README.md).
