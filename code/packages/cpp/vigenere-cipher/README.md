# vigenere-cipher (C++)

A pure ISO **C++17**, header-only implementation of the **Vigenere cipher** with
full cryptanalysis, in namespace `ca::vigenere`. A faithful port of the Rust
`vigenere-cipher` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only — no libm.

## What it is

The Vigenere cipher (Bellaso, 1553) is a polyalphabetic substitution: each
plaintext letter is shifted by a different amount from a repeating keyword.
Letters keep their case; non-alphabetic characters pass through and do not
advance the key. Two statistical tools, both included, break it:

- **Index of Coincidence** recovers the **key length**.
- **Chi-squared** frequency analysis recovers the **key**.

## API

```cpp
#include "vigenere_cipher.hpp"
namespace vig = ca::vigenere;

auto ct = vig::encrypt("ATTACKATDAWN", "LEMON");  // std::optional -> "LXFOPVEFRNHR"
auto pt = vig::decrypt(*ct, "LEMON");             // "ATTACKATDAWN"

std::size_t klen = vig::find_key_length(cipher, 20);  // e.g. 5
std::string key  = vig::find_key(cipher, klen);       // e.g. "LEMON"

vig::BreakResult r = vig::break_cipher(cipher);       // r.key, r.plaintext
```

`encrypt` / `decrypt` return `std::optional<std::string>` — `std::nullopt` if the
key is invalid (empty or non-alphabetic); `key_valid` checks a key up front.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own vectors — encrypt/decrypt cases, case and punctuation
handling, invalid-key rejection, round trips, and the cryptanalysis: recovering
key lengths 3/5/6 and the keys `KEY`/`LEMON`/`SECRET` from a long English
sample, plus a full automatic break.
