# csharp/scytale-cipher

Pure C# Scytale transposition cipher helpers with encrypt, decrypt, and brute-force key enumeration.

The implementation follows [CR02](../../../specs/CR02-scytale-cipher.md): each grid cell is one Unicode scalar value, uneven ciphertext columns are reconstructed explicitly, and only trailing U+0020 padding is removed. Brute force rejects inputs above 4096 scalars before allocating its quadratic candidate output. The package is deterministic pure computation with no OS capabilities.
