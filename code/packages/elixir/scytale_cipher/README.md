# Scytale Cipher (Elixir)

Ancient Spartan transposition cipher implementation in Elixir.

This package follows [CR02](../../../specs/CR02-scytale-cipher.md). Grid cells are Unicode scalar values (not grapheme clusters), uneven ciphertext columns are reconstructed explicitly, and only trailing U+0020 padding is removed. `brute_force/1` rejects inputs above 4096 scalars before allocating quadratic candidate output. Production code is deterministic pure computation with no OS capabilities.

The native test suite includes a generated, dependency-free consumer for all 18 normative Scytale cases in `classical-ciphers-v1`; `generate_scytale_fixture_consumers.py --check` prevents fixture or language-roster drift.

## Usage

```elixir
CodingAdventures.ScytaleCipher.encrypt("HELLO WORLD", 3)
# => "HLWLEOODL R "

CodingAdventures.ScytaleCipher.decrypt("HLWLEOODL R ", 3)
# => "HELLO WORLD"

CodingAdventures.ScytaleCipher.brute_force("HLWLEOODL R ")
# => [%{key: 2, text: "..."}, %{key: 3, text: "HELLO WORLD"}, ...]
```

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.
