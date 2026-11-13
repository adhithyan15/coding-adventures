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

## Windows

`BUILD_windows` runs `code/scripts/check-perl-extension-windows.ps1`, which
picks the native Strawberry Perl (never Git for Windows' MSYS Perl), builds the
Rust cdylib with `cargo build --release`, stages `target\release\GF256Native.dll`
as `lib\auto\CodingAdventures\GF256Native\GF256Native.<dlext>` (Strawberry's
`$Config{dlext}` is `xs.dll`), and runs `prove -v t/` against it. `perl-bridge`
compiles its C shim with that Perl's own compiler configuration and links the
interpreter's CORE import library.
