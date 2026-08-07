# bitset (C)

A growable **set of bits** packed into 64-bit words, in pure ISO C17. A faithful
port of the Rust `bitset` crate: set/clear/toggle/test with auto-grow, the
bitwise set operations (and/or/xor/not/and_not), population count, and
integer / binary-string conversions.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "bitset.h"

bitset b;
bitset_init(&b, 8);
bitset_set(&b, 3);
bitset_set(&b, 100);          /* auto-grows */
int on = bitset_test(&b, 3);  /* 1 */
size_t n = bitset_popcount(&b);

bitset a, c, out;
bitset_from_binary_str(&a, "1100");
bitset_from_binary_str(&c, "1010");
bitset_and(&a, &c, &out);     /* out = "1000"; caller frees out */
bitset_free(&out);
bitset_free(&b); bitset_free(&a); bitset_free(&c);
```

Bit 0 is the least-significant bit. `set`/`toggle` auto-grow; `clear`/`test`
treat out-of-range indices as unset. The bitwise ops allocate a new bitset
through their out-parameter. Every constructor pairs with `bitset_free`.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/bitset`; the word layout, capacity doubling, and
trailing-bit cleanup match the crate. `from_integer` takes the 128-bit value as
two `uint64_t` (low, high) since ISO C has no 128-bit integer. See also the
[C++ port](../../cpp/bitset/README.md).
