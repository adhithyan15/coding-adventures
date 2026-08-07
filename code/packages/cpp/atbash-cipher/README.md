# atbash-cipher (C++)

A pure ISO **C++17**, header-only implementation of the **Atbash cipher**, in
namespace `ca::atbash`. A faithful port of the Rust `atbash-cipher` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What it is

Atbash reverses the alphabet — A↔Z, B↔Y, …, M↔N — preserving case and passing
non-letters through unchanged. For a letter at position `p` (A=0 … Z=25) the
substitute is at `25 - p`. It is **self-inverse**, so `decrypt` is `encrypt`.

```cpp
#include "atbash_cipher.hpp"
namespace atb = ca::atbash;

atb::encrypt("Hello, World!");  // "Svool, Dliow!"
atb::decrypt("Svool, Dliow!");  // "Hello, World!"
atb::atbash_char('A');          // 'Z'
```

`encrypt` / `decrypt` take and return `std::string`. The port operates
byte-by-byte: only ASCII letters are substituted; every other byte passes
through unchanged, matching the crate.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own vectors — single-character mappings, the full
alphabet, case and punctuation handling, non-alpha passthrough, the self-inverse
property, and that no letter maps to itself.
