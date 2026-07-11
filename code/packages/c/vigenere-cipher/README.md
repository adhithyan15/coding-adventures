# vigenere-cipher (C)

A pure ISO **C17** implementation of the **Vigenere cipher** with full
cryptanalysis. A faithful port of the Rust `vigenere-cipher` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only —
no libm.

## What it is

The Vigenere cipher (Bellaso, 1553) is a polyalphabetic substitution: each
plaintext letter is shifted by a different amount taken from a repeating
keyword. Letters keep their case; non-alphabetic characters pass through and do
not advance the key.

It resisted cryptanalysis for 300 years until two statistical tools broke it,
both included here:

- **Index of Coincidence** finds the **key length** — splitting the ciphertext
  by the true key length makes each group a Caesar cipher on English (IC
  ~0.067), which stands out from random text (~0.038).
- **Chi-squared** frequency analysis finds the **key** — for each position group
  (a Caesar cipher), the shift whose decrypted frequencies best match English
  wins.

## API

```c
#include "vigenere_cipher.h"

char *ct = vigenere_encrypt("ATTACKATDAWN", "LEMON");  /* "LXFOPVEFRNHR" */
char *pt = vigenere_decrypt(ct, "LEMON");              /* "ATTACKATDAWN" */
free(ct); free(pt);

size_t klen = vigenere_find_key_length(cipher, 20);    /* e.g. 5 */
char  *key  = vigenere_find_key(cipher, klen);         /* e.g. "LEMON" */
free(key);

VigenereBreak r;
if (vigenere_break(cipher, &r)) {
    /* r.key and r.plaintext are recovered automatically */
    vigenere_break_free(&r);
}
```

`vigenere_encrypt` / `vigenere_decrypt` / `vigenere_find_key` return a malloc'd
string (caller frees), or `NULL` if the key is invalid (empty or non-alphabetic)
or on allocation failure. `vigenere_key_valid` checks a key up front.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own vectors — encrypt/decrypt cases, case and punctuation
handling, invalid-key rejection, round trips, and the cryptanalysis: recovering
key lengths 3/5/6 and the keys `KEY`/`LEMON`/`SECRET` from a long English
sample, plus a full automatic break.
