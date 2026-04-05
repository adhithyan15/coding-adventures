# Scytale Cipher (Perl)

Ancient Spartan transposition cipher implementation in Perl.

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
