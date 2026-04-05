# Vigenere Cipher (Ruby)

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
