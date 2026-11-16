# trig

Pure-Perl trigonometry built from first principles: no `POSIX`, no libm.
`sin_approx`, `cos_approx`, `tan_approx`, `degrees_to_radians`,
`radians_to_degrees`, `sqrt_approx`, `atan_approx`, `atan2_approx`, plus the
`sin_deg` / `cos_deg` / `tan_deg` wrappers.

## How it fits in the stack

`trig` is the base of the geometry and signal packages: `affine2d`, `arc2d`,
`point2d` and `wave` call it. Since 0.3.0 it follows **TRIG-REF-1**, the one
reference algorithm of
[PHY00a](../../../specs/PHY00a-trig-convergence.md), so the same input gives the
same 64 bits in every language port.

## The algorithm (TRIG-REF-1)

```
sin(x), cos(x):
  1. not (|x| <= 2^29)  ->  NaN        (NaN, +-inf and huge angles: outside the domain)
  2. |x| < 2^-27        ->  sin = x (so sin(-0.0) = -0.0), cos = 1
  3. k = nearest whole number of quarter-turns:  (x * 2/pi + MAGIC) - MAGIC
     r = x - k*pi/2     by Cody-Waite: pi/2 is stored as five slices, the first four
                        24 bits wide, so every k*slice is exact; |r| <= pi/4
  4. Horner polynomials of ten terms on stored coefficients 1/3!, 1/5!, ...
  5. quadrant q = k & 3 picks sin r, cos r, -sin r or -cos r
```

`radians` and `degrees` are one multiply by a bit-pinned constant. `tan` is
`sin / cos` with the documented pole sentinel (`+-1e308` when `|cos| < 1e-15`).
`sqrt_approx`, `atan_approx` and `atan2_approx` keep their algorithms; `atan2`
of a NaN is now NaN.

Every constant is built from its IEEE bit pattern at load time
(`unpack "d>", pack "H16", ...`), because Perl's decimal parser is not
guaranteed to be correctly rounded on every platform. The module refuses to
load on a Perl whose NV is not an 8-byte double.

## The domain, and accuracy

`sin_approx`, `cos_approx` and `tan_approx` are defined for `|x| <= 2**29`
radians (about 5.4e8). Outside it, and for NaN or an infinity, they return NaN
rather than a plausible-looking wrong number. A caller that accumulates a phase
must reduce it first (see `CodingAdventures::Wave`, which keeps its phase in
cycles). Accuracy, measured against correctly rounded values over the 62,154
shared vectors: `sin` and `cos` within 3 ulp (observed worst 2.38 ulp),
`radians` and `degrees` within 1 ulp. `tan` carries no accuracy claim.

## Perl details worth knowing

* `sin_approx(-0.0)` is `-0.0`, but Perl has no negative-zero *integer*: the
  literals `0` and `-0` are the integer 0, and `sin_approx(0)` is `+0`. Write
  `-0.0` (or decode the bits) to get a negative zero. The module never rewrites
  an argument with `* 1.0` or `+ 0.0`, which would erase the sign.
* Perl prints an NV with 15 digits. Compare results by their bits
  (`unpack "H16", pack "d>", $v`), as `t/02-conformance.t` does, never by
  stringifying them.

## Usage

```perl
use CodingAdventures::Trig qw(sin_approx cos_approx tan_approx degrees_to_radians);

my $s = sin_approx(3.141592653589793 / 6);   # 0.5 (to within 3 ulp)
my $r = degrees_to_radians(180);             # exactly PI
my $n = sin_approx(1e20);                    # NaN: outside the domain
```

## Tests

`t/02-conformance.t` checks every row of
`code/specs/fixtures/trig-reference-v1` (62,154 vectors) **bit for bit**,
asserts every embedded constant against the manifest, and checks accuracy
against the correctly rounded column (`tan` is a named no-claim op; an op with
a correct value but no limit fails; so does an unknown op).

```bash
bash BUILD
```

## Exact small arctangent

`atan` returns `$x` unchanged when `|x| <= 2^-27`, before any half-angle
reduction. This exact binary64 identity preserves negative zero and both signs
of the minimum subnormal; the shared PHY00 fixture locks those boundaries.

## Full-range square root

The square-root API follows PHY00 across the full binary64 range. It preserves
negative zero, returns positive infinity and NaN unchanged, rejects negative
inputs with the lane-native error, normalizes by powers of four into `[0.25, 4)`,
and then applies at most 60 Newton steps without calling the host square-root
routine. Boundary behavior is checked against
[`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).
