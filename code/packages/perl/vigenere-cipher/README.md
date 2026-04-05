# Vigenere Cipher (Perl)

A Perl implementation of the Vigenere polyalphabetic substitution cipher with full cryptanalysis support (key length detection via Index of Coincidence and key recovery via chi-squared analysis).

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

## Installation

```bash
cpanm --installdeps .
```

## Usage

```perl
use CodingAdventures::VigenereCipher qw(encrypt decrypt find_key_length find_key break_cipher);

# Encrypt
my $ct = encrypt("ATTACKATDAWN", "LEMON");  # "LXFOPVEFRNHR"

# Decrypt
my $pt = decrypt("LXFOPVEFRNHR", "LEMON");  # "ATTACKATDAWN"

# Cryptanalysis (requires 200+ character ciphertext)
my $key_len = find_key_length($long_ciphertext);
my $key = find_key($long_ciphertext, $key_len);
my ($recovered_key, $plaintext) = break_cipher($long_ciphertext);
```

## API

| Function | Description |
|----------|-------------|
| `encrypt($plaintext, $key)` | Encrypt using Vigenere cipher |
| `decrypt($ciphertext, $key)` | Decrypt using Vigenere cipher |
| `find_key_length($ciphertext, $max_length?)` | Estimate key length via IC analysis |
| `find_key($ciphertext, $key_length)` | Find key via chi-squared analysis |
| `break_cipher($ciphertext)` | Full automatic break (returns key, plaintext) |

## Testing

```bash
prove -l -v t/
```
