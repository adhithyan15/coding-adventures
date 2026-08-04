# hkdf (C++)

**HKDF** — the HMAC-based key derivation function (RFC 5869), in pure ISO C++17
(header-only). A faithful port of the Rust `hkdf` crate, built on the sibling
header-only `hmac`.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "hkdf.hpp"
#include "sha256.hpp"

auto sha = [](const std::vector<std::uint8_t>& d) {
    auto h = ca::sha256(d.data(), d.size());
    return std::vector<std::uint8_t>(h.begin(), h.end());
};
auto okm = ca::hkdf(sha, /*block*/64, /*digest*/32, salt, ikm, info, 42);
```

`hkdf_extract` / `hkdf_expand` are also available separately; `expand` throws
`std::invalid_argument` on a zero or too-large length.

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/hkdf`. Verified against the **RFC 5869** HKDF-SHA256
vectors. See also the [C port](../../c/hkdf/README.md).
