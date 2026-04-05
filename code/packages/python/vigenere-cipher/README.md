# Vigenere Cipher (Python)

A polyalphabetic substitution cipher with full cryptanalysis tools. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

## What is the Vigenere Cipher?

The Vigenere cipher shifts each letter by a different amount determined by a repeating keyword. Unlike Caesar (single shift) or Atbash (fixed mapping), each position uses a different shift, making simple frequency analysis ineffective.

It was considered unbreakable for 300 years until Kasiski's attack in 1863.

## Installation

```bash
pip install -e .
```

## Usage

### Encryption and Decryption

```python
from vigenere_cipher import encrypt, decrypt

# Basic encryption
ciphertext = encrypt("ATTACKATDAWN", "LEMON")  # "LXFOPVEFRNHR"

# Decryption
plaintext = decrypt("LXFOPVEFRNHR", "LEMON")   # "ATTACKATDAWN"

# Preserves case and punctuation
encrypt("Hello, World!", "key")  # "Rijvs, Uyvjn!"
```

### Cryptanalysis (Breaking the Cipher)

```python
from vigenere_cipher import find_key_length, find_key, break_cipher

# Automatic break (needs ~200+ chars of English text)
key, plaintext = break_cipher(long_ciphertext)

# Or step by step:
key_length = find_key_length(ciphertext)       # e.g. 6
recovered_key = find_key(ciphertext, key_length)  # e.g. "SECRET"
```

## How It Fits in the Stack

This is package CR03 in the cryptography layer, building on CR00 (Caesar) and CR01 (Atbash). The cryptanalysis techniques (IC, chi-squared) connect to the statistics packages (ST00, ST01).

## Running Tests

```bash
uv pip install -e .[dev]
pytest tests/ -v
```
