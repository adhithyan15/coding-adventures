# hmac (C++)

**HMAC** — keyed-hash message authentication (RFC 2104), in pure ISO C++17
(header-only). A faithful port of the Rust `hmac` crate's generic construction.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "hmac.hpp"
#include "sha256.hpp"

auto sha = [](const std::vector<std::uint8_t>& d) {
    auto h = ca::sha256(d.data(), d.size());
    return std::vector<std::uint8_t>(h.begin(), h.end());
};
std::vector<std::uint8_t> mac = ca::hmac(sha, /*block*/64, key, message);

bool ok = ca::hmac_verify(mac, expected);   // constant-time
```

`ca::hmac` is a function template over any hash callable
`std::vector<uint8_t>(const std::vector<uint8_t>&)`.

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/hmac`. Verified against the **RFC 4231 HMAC-SHA256**
vectors. See also the [C port](../../c/hmac/README.md).
