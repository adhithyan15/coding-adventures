# zeroize (C++)

**Secure in-memory wiping for secrets**, header-only in pure ISO C++17
(namespace `ca::zeroize`) — a faithful port of the Rust `zeroize` crate.

## The problem it solves

A compiler may **delete** a "clear the secret" store when it proves nothing
reads the zero afterward, leaving the secret in RAM. The fix: **volatile stores**
(observable behavior the compiler may not elide) plus a compiler fence
(`std::atomic_signal_fence` — the direct equivalent of Rust's
`compiler_fence(SeqCst)`).

## What you get

- `zeroize_bytes(ptr, len)` — the primitive.
- `zeroize(T&)` overloads for integers, `std::array<uint8_t, N>`,
  `std::vector<uint8_t>`, `std::string`, and `std::optional<T>` (found by ADL,
  so your own types can add an overload).
- **`Zeroizing<T>`** — an RAII wrapper whose destructor wipes the value, the C++
  analogue of Rust's `Drop`. Every exit path — including an exception unwind —
  scrubs the secret.

```cpp
#include "zeroize.hpp"
namespace z = ca::zeroize;

{
    z::Zeroizing<std::array<std::uint8_t, 32>> key(load_master_key());
    use(*key);
}   // key's bytes are wiped here, before the stack slot is reused

std::string pw = read_password();
z::zeroize(pw);                 // wiped and emptied
```

## Divergence from the Rust crate

`Zeroizing::into_inner()` moves the value out without wiping (the caller opts
out). Unlike Rust — which scrubs a `Vec`/`String`'s full *capacity* via raw
pointers — the `std::vector`/`std::string` overloads scrub only the live
`size()` bytes, because the capacity tail holds no live objects and touching it
is undefined in C++ (and flagged by sanitizers). For capacity scrubbing use a
`std::array` or the C `ZrBytes` buffer.

## Building

```sh
sh BUILD    # builds & runs the tests under every C++ compiler present
```

Pure ISO C++17. Builds clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness); the test suite also runs clean under
AddressSanitizer + UndefinedBehaviorSanitizer.
