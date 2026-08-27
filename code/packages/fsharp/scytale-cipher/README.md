# fsharp/scytale-cipher

Pure F# Scytale transposition cipher helpers with encrypt, decrypt, and brute-force key enumeration.

The implementation follows [CR02](../../../specs/CR02-scytale-cipher.md): each grid cell is one Unicode scalar value, uneven ciphertext columns are reconstructed explicitly, and only trailing U+0020 padding is removed. Brute force rejects inputs above 4096 scalars before allocating its quadratic candidate output. The package is deterministic pure computation with no OS capabilities.

The native test suite includes a generated, dependency-free consumer for all 18 normative Scytale cases in `classical-ciphers-v1`; `generate_scytale_fixture_consumers.py --check` prevents fixture or language-roster drift.
