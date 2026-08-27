# Scytale Cipher (Ruby)

Ancient Spartan transposition cipher implementation in Ruby.

This package follows [CR02](../../../specs/CR02-scytale-cipher.md). Grid cells are Unicode scalar values, uneven ciphertext columns are reconstructed explicitly, and only trailing U+0020 padding is removed. `brute_force` rejects inputs above 4096 scalars before allocating quadratic candidate output. Production code is deterministic pure computation with no OS capabilities.

## Usage

```ruby
require "coding_adventures_scytale_cipher"

ct = CodingAdventures::ScytaleCipher.encrypt("HELLO WORLD", 3)
# => "HLWLEOODL R "

pt = CodingAdventures::ScytaleCipher.decrypt(ct, 3)
# => "HELLO WORLD"

results = CodingAdventures::ScytaleCipher.brute_force(ct)
# => [{key: 2, text: "..."}, {key: 3, text: "HELLO WORLD"}, ...]
```

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.
