# atbash-cipher (C)

A pure ISO **C17** implementation of the **Atbash cipher**. A faithful port of
the Rust `atbash-cipher` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## What it is

Atbash is one of the oldest known ciphers: it reverses the alphabet, mapping
A↔Z, B↔Y, …, M↔N. Case is preserved and non-letters pass through unchanged. For
a letter at position `p` (A=0 … Z=25) the substitute is at `25 - p`.

It is **self-inverse** — applying it twice returns the original text
(`25 - (25 - p) = p`) — so decryption is identical to encryption.

```c
#include "atbash_cipher.h"

char *ct = atbash_encrypt("Hello, World!");  /* "Svool, Dliow!" */
char *pt = atbash_decrypt(ct);               /* "Hello, World!" */
free(ct); free(pt);

char z = atbash_char('A');   /* 'Z' */
```

`atbash_encrypt` / `atbash_decrypt` return a malloc'd string (caller frees), or
`NULL` on allocation failure. The port operates byte-by-byte: only ASCII letters
are substituted, and every other byte — including those of a UTF-8 sequence —
passes through unchanged, matching the crate.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own vectors — single-character mappings, the full
alphabet, case and punctuation handling, non-alpha passthrough, the self-inverse
property, and the fact that no letter maps to itself.
