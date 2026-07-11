# scytale-cipher (C++)

A pure ISO **C++17**, header-only implementation of the **Scytale** transposition
cipher, in namespace `ca::scytale`. A faithful port of the Rust `scytale-cipher`
crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What it is

The Scytale (Sparta, ~700 BCE) reorders characters rather than replacing them:
encryption writes the text row-by-row into a grid `key` columns wide (padding
the last row with spaces) and reads it column-by-column; decryption reverses
that and strips the trailing pad spaces.

Like the crate, this port transposes whole **characters** — the input is split
into UTF-8 character units and reordered, so multibyte characters stay intact.

## API

```cpp
#include "scytale_cipher.hpp"
namespace sc = ca::scytale;

auto ct = sc::encrypt("HELLO WORLD", 3);   // std::optional -> "HLWLEOODL R "
auto pt = sc::decrypt(*ct, 3);             // "HELLO WORLD"

std::vector<sc::BruteForceResult> guesses = sc::brute_force(*ct);  // keys 2..n/2
```

`encrypt` / `decrypt` return `std::optional<std::string>` — an empty text yields
`""`; `std::nullopt` means an invalid key (`< 2` or `>` character count).
`brute_force` returns `{key, text}` results (empty when the text has fewer than
4 characters).

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own vectors — encrypt/decrypt cases, key validation,
padding stripping, round trips over all valid keys, brute force, and a multibyte
UTF-8 round trip.
