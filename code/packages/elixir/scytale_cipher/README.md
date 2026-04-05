# Scytale Cipher (Elixir)

Ancient Spartan transposition cipher implementation in Elixir.

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
