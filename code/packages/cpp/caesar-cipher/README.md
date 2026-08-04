# caesar-cipher (C++)

The **Caesar cipher** — encrypt, decrypt, ROT13 — plus **brute-force** and
**frequency-analysis** attacks, in pure ISO C++17 (header-only). A faithful port
of the Rust `caesar-cipher` crate.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "caesar_cipher.hpp"
using namespace caesar_cipher;

std::string ct = encrypt("Hello, World!", 3);   // "Khoor, Zruog!"
std::string pt = decrypt(ct, 3);                // "Hello, World!"
std::string r  = rot13("Hello");               // "Uryyb"

// Break a cipher without knowing the shift:
auto [shift, plaintext] = frequency_analysis(ct);

// Or inspect every candidate:
for (const auto& c : brute_force(ct)) { /* c.shift, c.plaintext */ }
```

### API

| Function | Returns |
| --- | --- |
| `encrypt(text, shift)` / `decrypt(text, shift)` / `rot13(text)` | `std::string` |
| `letter_counts(text)` | `std::array<std::size_t, 26>` (A–Z, case-insensitive) |
| `chi_squared(text)` | `double` — fit vs English frequencies (lower = better) |
| `brute_force(ct)` | `std::vector<BruteForceResult>` — all 25 candidate decryptions |
| `frequency_analysis(ct)` | `std::pair<int, std::string>` — best shift + plaintext |

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

## Relationship to the Rust crate

Ports `code/packages/rust/caesar-cipher`. The API mirrors the crate closely —
`brute_force` returns all 25 candidates and `frequency_analysis` returns the
best `(shift, plaintext)` — and the shift normalisation, chi-squared scoring,
and English frequency table match it exactly. See also the buffer-based
[C port](../../c/caesar-cipher/README.md).
