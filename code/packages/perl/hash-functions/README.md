# CodingAdventures::HashFunctions

Pure-Perl implementations of five educational, non-cryptographic hash
functions:

- FNV-1a in 32-bit and 64-bit forms
- DJB2 with 64-bit wrapping
- polynomial rolling hash with configurable base and modulus
- MurmurHash3 x86_32 with a configurable seed

The package also includes deterministic avalanche and bucket-distribution
helpers for comparing hash behavior.

```perl
use CodingAdventures::HashFunctions qw(
    fnv1a_32 fnv1a_64 murmur3_32 uint64_hex
);

my $short = fnv1a_32('hello');
my $exact = fnv1a_64('hello');
my $bits  = uint64_hex($exact);       # a430d84680aabd0b
my $mixed = murmur3_32('hello', 42);
```

Inputs are binary-safe byte strings. UTF-8 flagged strings are encoded to
UTF-8 first, which makes text behavior explicit and repeatable. The 64-bit
functions return core `Math::BigInt` values so results remain exact on every
supported Perl build.

These algorithms are for learning, hash tables, and checksums. They are not
cryptographic hashes and must not be used for passwords, signatures, or other
security-sensitive purposes.
