# CodingAdventures::PolynomialNative (Perl)

Perl XS extension providing polynomial arithmetic, backed by the Rust
`polynomial` crate via the zero-dependency `perl-bridge`.

## Where it fits

```
CodingAdventures::PolynomialNative  (this package — Perl XS)
         │
         └── perl-bridge (Rust)   ──── Perl 5 C API declarations
         └── polynomial (Rust)    ──── core arithmetic
```

## Usage

```perl
use CodingAdventures::PolynomialNative;
use alias CPN = 'CodingAdventures::PolynomialNative';

# Polynomials are array references, index = degree
my $a = [1.0, 2.0];       # 1 + 2x
my $b = [3.0, 4.0];       # 3 + 4x

my $sum  = CPN::add($a, $b);          # [4.0, 6.0]
my $prod = CPN::multiply($a, $b);     # [3.0, 10.0, 8.0]
my $val  = CPN::evaluate($a, 2.0);    # 5.0  (1 + 2*2 = 5)
my $deg  = CPN::degree([3.0, 0, 2.0]); # 2
```

## Building

```bash
cargo build --release
# Copy PolynomialNative.so to the DynaLoader path:
mkdir -p blib/arch/auto/CodingAdventures/PolynomialNative
cp target/release/libPolynomialNative.so \
   blib/arch/auto/CodingAdventures/PolynomialNative/PolynomialNative.so
```

## Notes

- The XS calling convention (`dXSARGS`, `ST(n)`, `XSRETURN`) is routed
  through the shared `perl-bridge` shim so the extension works with threaded
  Perl builds too.
- `xs_init!` from perl-bridge needs `concat_idents` (unstable Rust), so
  `boot_CodingAdventures__PolynomialNative` is written by hand.
- `divmod` is omitted from Perl XS due to Perl's single-return convention;
  use `divide` and `modulo` separately (pure-Perl wrappers can add this).
