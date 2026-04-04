# CodingAdventures::GF256Native (Perl)

Perl XS extension providing GF(256) Galois Field arithmetic, backed by the
Rust `gf256` crate via the zero-dependency `perl-bridge`.

## Usage

```perl
use CodingAdventures::GF256Native;

CodingAdventures::GF256Native::add(83, 202)        # 153 (XOR)
CodingAdventures::GF256Native::multiply(2, 16)     # 32
CodingAdventures::GF256Native::divide(4, 2)        # 2
CodingAdventures::GF256Native::power(2, 8)         # 29
CodingAdventures::GF256Native::inverse(83)         # multiplicative inverse
```

## Building

```bash
cargo build --release
mkdir -p blib/arch/auto/CodingAdventures/GF256Native
cp target/release/libGF256Native.so \
   blib/arch/auto/CodingAdventures/GF256Native/GF256Native.so
```
