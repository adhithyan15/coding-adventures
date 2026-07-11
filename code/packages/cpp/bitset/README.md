# bitset (C++)

A growable **set of bits** packed into 64-bit words, in pure ISO C++17
(header-only). A faithful port of the Rust `bitset` crate.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "bitset.hpp"

ca::bitset b(8);
b.set(3);
b.set(100);                       // auto-grows
bool on = b.test(3);
std::size_t n = b.popcount();

auto x = ca::bitset::from_binary_string("1100");
auto y = ca::bitset::from_binary_string("1010");
auto z = x & y;                   // "1000"; also | ^ ~ and_not
std::optional<std::uint64_t> v = ca::bitset::from_integer(0xAB).to_integer();
```

Bit 0 is the least-significant bit. `set`/`toggle` auto-grow; the bitwise
operators return a new bitset; `to_integer` returns `std::optional` (nullopt if
a bit beyond index 63 is set).

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/bitset`. See also the [C port](../../c/bitset/README.md).
