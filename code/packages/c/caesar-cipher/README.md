# caesar-cipher (C)

The **Caesar cipher** — encrypt, decrypt, ROT13 — plus a **frequency-analysis
attack**, in pure ISO C17. A faithful port of the Rust `caesar-cipher` crate.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "caesar_cipher.h"

char out[64];
caesar_encrypt("Hello, World!", 3, out, sizeof out);   /* "Khoor, Zruog!" */
caesar_decrypt(out, 3, out, sizeof out);               /* back to "Hello, World!" */
caesar_rot13("Hello", out, sizeof out);                /* "Uryyb" */

/* Break a cipher without knowing the shift: */
char plain[64];
int shift = caesar_frequency_analysis(out, plain, sizeof plain);
```

C has no growable string, so the transforming functions write a NUL-terminated
result into a caller-supplied buffer and return the character count (or `-1` if
the buffer is too small). Since the cipher is a 1:1 character mapping, a buffer
of `strlen(text) + 1` bytes is always enough.

### API

| Function | Purpose |
| --- | --- |
| `caesar_encrypt(text, shift, out, n)` | shift letters forward by `shift` (mod 26) |
| `caesar_decrypt(text, shift, out, n)` | inverse of encrypt |
| `caesar_rot13(text, out, n)` | shift by 13 (its own inverse) |
| `caesar_letter_counts(text, counts)` | tally A–Z (case-insensitive) into `counts[26]` |
| `caesar_chi_squared(text)` | fit vs English letter frequencies (lower = better) |
| `caesar_frequency_analysis(ct, out, n)` | best shift 1..25 + its plaintext |

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

## Relationship to the Rust crate

Ports `code/packages/rust/caesar-cipher`. The shift normalisation, chi-squared
scoring, and English frequency table match it exactly. Brute-force (returning
all 25 candidates as a growable list) is provided in the
[C++ port](../../cpp/caesar-cipher/README.md); in C it is a trivial loop over
`caesar_decrypt` for shifts 1..25.
