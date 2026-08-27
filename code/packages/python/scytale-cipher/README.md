# Scytale Cipher (Python)

Ancient Spartan transposition cipher implementation in Python.

This package follows [CR02](../../../specs/CR02-scytale-cipher.md). Grid cells are Unicode scalar values, uneven ciphertext columns are reconstructed explicitly, and only trailing U+0020 padding is removed. `brute_force` rejects inputs above 4096 scalars before allocating quadratic candidate output. Production code is deterministic pure computation with no OS capabilities.

## What is the Scytale Cipher?

The Scytale (pronounced "SKIT-ah-lee") is one of the earliest known transposition ciphers, used by the Spartans around 700 BCE. Unlike substitution ciphers which replace characters, the Scytale rearranges their positions using a columnar transposition.

## Usage

```python
from scytale_cipher import encrypt, decrypt, brute_force

# Encrypt with a key (number of columns)
ciphertext = encrypt("HELLO WORLD", 3)
# => "HLWLEOODL R "

# Decrypt with the same key
plaintext = decrypt(ciphertext, 3)
# => "HELLO WORLD"

# Brute-force all possible keys
results = brute_force(ciphertext)
# => [{"key": 2, "text": "..."}, {"key": 3, "text": "HELLO WORLD"}, ...]
```

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.
