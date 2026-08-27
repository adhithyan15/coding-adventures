# Scytale Cipher (Perl)

Ancient Spartan transposition cipher implementation in Perl.

This package follows [CR02](../../../specs/CR02-scytale-cipher.md). Grid cells are Unicode scalar values, uneven ciphertext columns are reconstructed explicitly, and only trailing U+0020 padding is removed. `brute_force` rejects inputs above 4096 scalars before allocating quadratic candidate output. Production code is deterministic pure computation with no OS capabilities.

## Usage

```perl
use CodingAdventures::ScytaleCipher qw(encrypt decrypt brute_force);

my $ct = encrypt("HELLO WORLD", 3);
# => "HLWLEOODL R "

my $pt = decrypt($ct, 3);
# => "HELLO WORLD"

my @results = brute_force($ct);
# => ({key => 2, text => "..."}, {key => 3, text => "HELLO WORLD"}, ...)
```

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.
