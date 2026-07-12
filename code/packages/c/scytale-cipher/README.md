# scytale-cipher (C)

A pure ISO **C17** implementation of the **Scytale** transposition cipher. A
faithful port of the Rust `scytale-cipher` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## What it is

The Scytale (Sparta, ~700 BCE) is a *transposition* cipher — it reorders
characters rather than replacing them. Encryption writes the text row-by-row
into a grid `key` columns wide, pads the last row with spaces, and reads the
grid column-by-column. Decryption rebuilds the grid and reads it row-by-row,
stripping the trailing pad spaces.

```
"HELLO WORLD", key 3    grid          read columns -> "HLWLEOODL R "
                        H E L
                        L O _
                        W O R
                        L D _
```

Like the crate, this port transposes whole **characters**: the input is split
into UTF-8 character units and those units are reordered, so multibyte
characters stay intact (malformed bytes become single-byte units, so any input
round-trips).

## API

```c
#include "scytale_cipher.h"

char *ct = scytale_encrypt("HELLO WORLD", 3);  /* "HLWLEOODL R " */
char *pt = scytale_decrypt(ct, 3);             /* "HELLO WORLD" */
free(ct); free(pt);

size_t count;
ScytaleBrute *r = scytale_brute_force(ct, &count);  /* keys 2..n/2 */
scytale_brute_free(r, count);
```

`scytale_encrypt` / `scytale_decrypt` return a malloc'd string (caller frees).
An empty text yields `""`; otherwise `NULL` means an invalid key (`< 2` or `>`
character count) or allocation failure. `scytale_brute_force` returns an array
of `{key, text}` (empty when the text has fewer than 4 characters).

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own vectors — encrypt/decrypt cases, key validation,
padding stripping, round trips over all valid keys, brute force, and a multibyte
UTF-8 round trip.
