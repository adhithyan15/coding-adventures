# Vigenere Cipher (Ruby)

## CR03 conformance

This implementation follows [CR03](../../../specs/CR03-vigenere-cipher.md): keys and cipher transforms use ASCII letters only, analysis ignores non-ASCII letters, and non-ASCII Unicode scalars pass through without advancing the key. Analysis accepts at most 8,192 Unicode scalars and key-length bounds up to 40, uses the smallest candidate within 90% of the best index-of-coincidence score, preserves the exact requested recovered-key length, and resolves score ties toward the smallest shift.

The native suite executes all 26 language-neutral `classical-ciphers-v1` Vigenere cases through a generated dependency-free consumer; `generate_vigenere_fixture_consumers.py --check` prevents corpus, expected-object, and established-lane drift.

A polyalphabetic substitution cipher with full cryptanalysis tools. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

## What is the Vigenere Cipher?

The Vigenere cipher shifts each letter by a different amount determined by a repeating keyword. It was considered unbreakable for 300 years until Kasiski's attack in 1863.

## Installation

```ruby
gem "coding_adventures_vigenere_cipher", path: "."
```

## Usage

```ruby
require "coding_adventures_vigenere_cipher"

VC = CodingAdventures::VigenereCipher

# Encryption and decryption
VC.encrypt("ATTACKATDAWN", "LEMON")  # => "LXFOPVEFRNHR"
VC.decrypt("LXFOPVEFRNHR", "LEMON")  # => "ATTACKATDAWN"

# Preserves case and punctuation
VC.encrypt("Hello, World!", "key")  # => "Rijvs, Uyvjn!"

# Automatic cipher breaking (needs ~200+ chars)
key, plaintext = VC.break_cipher(long_ciphertext)
```

## Running Tests

```bash
bundle install
bundle exec rake test
```
