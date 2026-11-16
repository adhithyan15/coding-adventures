# PHY00a: Trig Convergence (TRIG-CONV): One Reference Algorithm for Every Port

**Status:** Specification only. No port is changed by this document. It is the
normative text for backlog item TRIG-CONV (owner decision 16, 2026-11-16, VER30
track C16; VER04p FL-15). The implementation work is the slice list in section 8:
one fixture slice, then one pull request per port.

**Parent:** [PHY00](PHY00-trig.md) (what `trig` is for and what it exports),
[VER04p](VER04p-warrant-floats.md) (the floats feature whose pilot waits for this
document: 3.3 NaN, 3.4 signed zero, 5.4 tolerance claims, 6 why there is no
built-in `sin`, 9 the environment probe, 10 bit-pattern encoding, 11.1 the edge
list, 12.2 the lane table, 15 the pilot),
[VER30](VER30-warrant-expansion-roadmap.md) (decision 16, track C16),
[VER-BACKLOG](VER-BACKLOG.md) (TRIG-CONV), and the existing shared corpus
[`fixtures/phy00-phy01-v1`](fixtures/phy00-phy01-v1/README.md), which this
document extends and does not replace.

**Numbering.** Package specifications in this directory are numbered by layer
(`PHY00` is `trig`), and amendments take a letter suffix (`VER04p`, `LM00a`). This
is therefore `PHY00a`, not a new number: it is an amendment to the `trig`
contract that fixes its arithmetic to the bit.

## 0. Why this exists

PHY00 promises that every `trig` port "uses the same first-principles algorithm".
That is true of the idea (a Maclaurin series with range reduction) and false of
the arithmetic. The seventeen ports differ in how many terms they add, how they
fold an angle into a small interval, how they group the multiplications inside a
term, how they multiply by a degree constant, and what they do at a NaN, an
infinity, or an argument of 1e20. Section 2 measures this: on twelve inputs, nine
ports run side by side return **up to four different bit patterns for one
`sin`**, and `sin(1e20)` is `0.0` in three ports, `NaN` in one, and a plausible
number in the rest.

Today that costs nothing, because every test compares with a tolerance of 1e-10.
It costs a great deal the moment someone wants to *prove* something. The floats
feature (VER04p) lets a Warrant claim "this function returns exactly these bits".
A claim that holds in the Rust lane and fails in the Lua lane is not a claim about
`trig`; the polyglot rule (VER04i) refuses it. The owner therefore decided (VER30
decision 16, 2026-11-16) to **converge the ports on one reference algorithm, spec
first, every port changed in its own pull request, before the floats pilot
starts.** This document is the spec. It does three jobs:

1. It **surveys** what the ports do (section 2), so that the change to each port is
   known before it is made.
2. It **defines one algorithm** (section 3) that every language can reproduce bit
   for bit using only IEEE 754 binary64 addition, subtraction, multiplication and
   comparison, and says honestly how accurate it is (section 4).
3. It defines how a port **proves** it is converged (section 6, language-neutral
   vectors) and **in what order** the ports change (section 8).

The analogy is a metre bar. Seventeen workshops each cut a "one metre" rod from
the same drawing; all rods look right to the eye and differ by a hair. Nobody
could build a bridge from parts cut in two workshops. The reference algorithm is
the one master bar, the conformance vectors are the calibration marks, and each
port PR is one workshop being re-cut and checked against the bar.

### 0.1 Decisions of record and the choices this spec adds

| # | Decision | Source |
|---|---|---|
| a | The ports converge on one reference algorithm; spec first; one PR per port; the floats pilot waits | owner, VER30 decision 16, 2026-11-16 |
| b | `sin(-0)` is `-0` (a "sin(-0) must be -0" claim is a real contract, VER04p 3.4) | VER04p 3.4, adopted here |
| c | A float in a corpus is a bit pattern, never a decimal | VER04p 10 |
| d | Results are compared to a correctly rounded value measured in ulps, not to a host libm | VER04p 5.4, 6 |
| e | **Added here, recommended, open for the owner (section 9):** range reduction is **nearest-integer Cody-Waite**, not truncation; arguments beyond 2^29 are **folded** (finite, bounded, no accuracy claim) rather than NaN; `radians` and `degrees` multiply by a **pre-divided constant**; the series is **Horner on stored coefficients**, ten terms each | this document |

Choice (e) departs from one phrase of the backlog row, "range reduction through
truncation". Truncation (`x - 2*pi*int(x/(2*pi))`) leaves a remainder in (-2*pi,
2*pi) that needs a second correction, rounds in every step, and has no defined
result once the quotient leaves the integer range. The same operation with a
*rounded* quotient reduces to [-pi/4, pi/4] directly. It is the same idea; the
difference is the rounding of the quotient, and section 3.4 explains why it is the
better choice.

## 1. Scope

| Operation | In this document | Treatment |
|---|---|---|
| `sin`, `cos` | normative, section 3.4 to 3.6 | new algorithm, **result bits change in every port** |
| `radians`, `degrees` | normative, section 3.7 | one multiply by a bit-pinned constant; changes 8 of 17 ports |
| `tan` | normative, section 3.8 | `sin / cos` with the existing pole sentinel; changes with `sin`/`cos`; Lua and Perl gain the sentinel |
| `sqrt`, `atan`, `atan2` | normative, section 3.9 | **the existing majority algorithm, unchanged**, pinned to the bit and checked by vectors; one port (Dart `atan`) differs today |
| `PI`, `TWO_PI`, `HALF_PI` constants | pinned, section 3.2 | bit patterns |
| Perl `sin_deg`, `cos_deg`, `tan_deg`; Haskell `sqrt :: Either`; error signalling of `sqrt` | unchanged | language-idiomatic, section 3.9.2 |

Out of scope: any change to port code (this is a specification), a correctly
rounded or table-driven `sin` (VER04p 6 explains why that is a project of its own),
a `Payne-Hanek` reduction for huge arguments (open question OQ-1), and any
`float32` variant.

**Port count.** `origin/main` (046c2d8b84b) holds **seventeen** `trig` packages: the
fifteen established lanes (C#, Dart, Elixir, F#, Go, Haskell, Java, Kotlin, Lua,
Perl, Python, Ruby, Rust, Swift, TypeScript) plus the emerging C and C++ lanes
(PHY00 section 6). VER04p 15 and the backlog row say "sixteen". The plan below
covers all seventeen; the wording fix is OQ-7.

## 2. Survey: what the ports do today

Read at `origin/main` 046c2d8b84b on 2026-11-16: every port's `sin`, `cos`,
`radians`, `degrees`, its reduction helper, `tan`, `sqrt`, `atan`, `atan2`, and its
tests' tolerance. Nine ports (Python, Ruby, Lua, Perl, TypeScript, C, Rust, Java,
Go) were also **run** on a probe set (2.2); the others are read, not run.

### 2.1 The comparison table

Two groupings of one multiplication matter and are named here. **G1** is
`term * ((-x*x) / denom)` (the quotient is formed first; `term *= -x*x/denom` in
C-family syntax). **G2** is `(term * (-x*x)) / denom` (left to right). They round
differently. **Radians A** is `deg * PI / 180` (two roundings); **radians B** is
`deg * (PI / 180)` (one rounding, a pre-divided constant).

| Port | Type | Terms (sin/cos) | Reduction to the series | Term update | `radians`/`degrees` |
|---|---|---|---|---|---|
| c | f64 | 20 / 20 | `x - 2pi*trunc(x/2pi)` with a `long long` cast (the quotient is passed through unchanged if `\|q\| >= 2^53` or NaN); then clamp to [-pi, pi] both sides | G1 | B |
| cpp | f64 | 20 / 20 | identical to C (header only) | G1 | B |
| csharp | f64 | 20 / 20 | `x % 2pi` (exact truncated remainder); clamp both sides | G1 | B |
| dart | f64 | 20 / 20 | `x % 2pi` (Euclidean: result in [0, 2pi)); only `> pi` corrected, so the range is [0, pi] and (-pi, 0) | G1 | A |
| elixir | f64 (accepts ints) | 20 / 20 | `x - Float.round(x/2pi) * 2pi` (nearest, ties away from zero); clamp both sides | G2 | A |
| fsharp | f64 | 20 / 20 | `x % 2pi`; clamp both sides | G2 | B |
| go | f64 | **21 / 21** (`n = 1..20`) | `x - 2pi*float64(int(x/2pi))`; `int()` out of range is implementation-defined; clamp both sides | G2 | A |
| haskell | f64 | 20 / 20 | `mod'` (floored: `x - fromInteger(floor(x/y)) * y`, where the product rounds for large `x`); only `> pi` corrected | G2 | A |
| java | f64 | 20 / 20 | `x % 2pi`; clamp both sides | G1 | B |
| kotlin | f64 | 20 / 20 | `x % 2pi`; clamp both sides | G1 | B |
| lua | f64 (5.3+ also has an integer subtype) | **21 / 21** | `x - 2pi*math.modf(x/2pi)`; clamp both sides | G2 | A |
| perl | f64 (NV) | **21 / 21** | `x - 2pi*int(x/2pi)`; clamp both sides | G2 | A (`degrees_to_radians`, `radians_to_degrees`) |
| python | f64 | 20 / 20 | `x % 2pi` (exact floored remainder, [0, 2pi)); only `> pi` corrected | G2 | A |
| ruby | f64 | 20 / 20 | `x % 2pi` (floored); only `> pi` corrected | G1 | A |
| rust | f64 | 20 / 20 | `x % 2pi` (exact truncated); clamp both sides | G1 | B |
| swift | f64 | **21 / 21** (`1...20`) | `truncatingRemainder` (exact); clamp both sides | G1 | B |
| typescript | f64 (`number`) | 20 / 20, but the sum starts at `0.0` and adds the term before updating it | `x % 2pi` (exact); clamp both sides | G1 | B |

What every port shares: `PI = 3.141592653589793`, `TWO_PI = 2*PI`, a Maclaurin
series whose term is built from the previous one, `sum` accumulated ascending, and
no use of the host `sin` or `cos`. Everything else in the table varies.

Other behaviours found on the way:

| Port | Finding |
|---|---|
| lua, perl | `tan` has **no pole sentinel** (every other port returns +-1e308 when `\|cos\| < 1e-15`; PHY00 3.6 documents it) |
| lua | on 5.3+, `-x*x` is **integer** arithmetic for an integer argument and wraps silently for large ones |
| perl | `degrees(-0.0)` is `+0.0` (measured), because `* 180.0` takes an integer path; `radians(-0.0)` is `-0.0` |
| go | `int(x / TwoPI)` is implementation-defined out of range: `sin(1e20)` is `NaN` on amd64 (measured), and arm64 differs |
| c, cpp, lua, perl | the truncating cast or `modf` of a huge quotient leaves `x - m*q` as rounding residue: `sin(1e20) = 0` and `cos(1e20) = 1` (measured) |
| dart, haskell, python, ruby | the floored remainder plus a one-sided correction means the reduced angle lies in [0, pi] or (-pi, 0), a different interval from the other twelve |
| perl | extra exports `sin_deg`, `cos_deg`, `tan_deg`, `degrees_to_radians`, `radians_to_degrees` |
| haskell | `sqrt :: Double -> Either String Double` (the error channel differs by design) |
| dart | `atan` groups its term `term *= -squared * (2i-1) / (2i+1)` (a different association from the other sixteen) |
| elixir | BEAM floats cannot be NaN or infinite: arithmetic that would produce one raises `ArithmeticError` (VER04p 12.2) |

### 2.2 Measured divergence

The harness (kept out of the repository; it is throwaway) called each port's own
source on twelve inputs and printed the result bits. Nine ports were run, on
x86-64: Python, Ruby, Lua, Perl, TypeScript (via `node --experimental-strip-types`),
C (`gcc -O2`), Rust (`rustc -O`), Java 17+, Go. "Distinct" counts different bit
patterns among the nine; every NaN counts as one.

| Input | `sin` distinct | `cos` distinct | `radians` distinct | `degrees` distinct |
|---|---|---|---|---|
| -0 | 1 (all `+0`) | 1 | 1 | **2** (Perl `+0`, others `-0`) |
| 2.718281828459045 | **2** | 1 | 1 | 1 |
| 1e5 | **2** | **4** | **2** | 1 |
| 1e6 | **2** | **2** | 1 | 1 |
| 1e20 | **4** (`0`, NaN, two numbers) | **3** | 1 | 1 |
| 0x400921fb54442d18 (`PI`) | **3** | **2** | **2** | 1 |
| 0x3fe921fb54442d18 (`PI/4`) | 1 | 1 | **2** | 1 |

Four consequences:

1. **`sin(-0)` is `+0` in all nine ports run** (and, by reading, in all seventeen).
   The series starts at `-0`, the next term is `+0`, and `-0 + +0 = +0`. VER04p
   15.1 suspected this; it is true.
2. `radians` splits the ports into two camps of 8 (A) and 9 (B); they differ in the
   last bit on about one input in four (28% of 100,000 random inputs in
   [-720, 720] for `radians`, 24% for `degrees`).
3. At `sin(PI)` the nine ports give three different tiny values (the true value,
   1.2246467991473532e-16, is `0x3ca1a62633145c07`; none of the nine returns it).
4. A NaN's sign differs by language (`0xfff8...` from x86 hardware, `0x7ff8...`
   from Rust and Java). Corpora must compare NaN as a class (VER04p 3.3).

### 2.3 The findings the algorithm must answer

| # | Finding | Answered in |
|---|---|---|
| F1 | Term count differs (20 vs 21) | 3.5: fixed, ten terms each, stored coefficients |
| F2 | Four reduction families (exact truncated, cast-truncated, floored, rounded) | 3.4 |
| F3 | Two groupings (G1, G2) of the term update | 3.5: no recurrence, no per-term division |
| F4 | `radians`/`degrees` grouping (A vs B) | 3.7, evidence in 4.3 |
| F5 | `sin(-0) = +0` | 3.3: explicit early return |
| F6 | No stated result for NaN, infinities, and quotients beyond the integer range | 3.3, 3.4 |
| F7 | A fusable shape `x - 2pi*k` in the reduction (VER04p 15.1 "fusion") and every series step `acc*z + c` | 5 |
| F8 | Lua/Perl `tan` without the sentinel; Dart `atan` grouping | 3.8, 3.9 |
| F9 | Division inside the series (Python, Perl, BEAM raise on a zero divisor; division is the slowest and least-portable op) | 3.5: no division anywhere in `sin`/`cos`/`radians`/`degrees` |

## 3. The reference algorithm (TRIG-REF-1)

### 3.1 The arithmetic model

Every value is an IEEE 754 binary64 number. Every operation is performed in
binary64 with **round-to-nearest, ties-to-even**, **one rounding per operation**,
with **no fused multiply-add**, **no extended precision**, **no flushing of
subnormals**, no reassociation, no reciprocal substitution. Permitted operations:

| Operation | Notation | Exactness |
|---|---|---|
| add, subtract, multiply | `a + b`, `a - b`, `a * b` | correctly rounded, one at a time |
| negate, absolute value | `-a`, `\|a\|` | exact (sign-bit operations) |
| compare | `<`, `>`, `==` | exact; any comparison with a NaN is false |
| integer part of an integer-valued double | `int(k)` | exact; used only on `\|k\| < 2^30` |
| bitwise and on that integer | `n & 3` | two's complement (so `-1 & 3 = 3`) |
| power-of-two scaling | `x * 0.5` | exact (below the underflow range, never reached) |

`sin`, `cos` and the two conversions use **no division, no square root, no
library function**. (`tan` and Part II use division and are labelled where they
do.) An implementation that cannot meet this model in a language or on a target is
`not-live` for floats under VER04p 9.2; this spec adds no lane-specific exemption.

A *product that feeds an addition or subtraction* is a **fusable site**. Every
fusable site in the algorithm is written as two separate statements in the
pseudo-code below, and section 5 says how each language keeps them separate.

### 3.2 Constants

All constants are binary64 and are specified **by bit pattern**; the decimal is
the shortest round-trip decimal and is informative. Where a language has
hexadecimal float literals the bit pattern is authoritative; where it does not,
the decimal literal is used **and** the port's test asserts the bits (section 6.5),
and a language whose decimal parser is not guaranteed correctly rounded builds the
constant from its bits (section 5).

**Reduction constants.** `INV_PIO2` is the double nearest `2/pi`. `PIO2_1` to
`PIO2_4` are the first 96 bits of `pi/2`, in four chunks of **24 significant bits
each**, and `PIO2_5` is the remainder rounded to 53 bits. The chunking is the
Cody-Waite device (William J. Cody and William Waite, *Software Manual for the
Elementary Functions*, 1980): because each of the first four has 24 significant
bits, `k * PIO2_i` is **exact** whenever `|k| < 2^29`, and the long subtraction
below loses nothing until the last term. The five parts sum to `pi/2` with a
residual of `2^-150`.

| Name | Bits | Hex float | Shortest decimal |
|---|---|---|---|
| `INV_PIO2` | `0x3FE45F306DC9C883` | `0x1.45f306dc9c883p-1` | `0.6366197723675814` |
| `PIO2_1` | `0x3FF921FB40000000` | `0x1.921fb4p+0` | `1.570796251296997` |
| `PIO2_2` | `0x3E74442D00000000` | `0x1.4442dp-24` | `7.549789415861596e-08` |
| `PIO2_3` | `0x3CF8469880000000` | `0x1.84698p-48` | `5.390302529957765e-15` |
| `PIO2_4` | `0x3B78CC5160000000` | `0x1.8cc516p-72` | `3.282003415807913e-22` |
| `PIO2_5` | `0x39F01B839A25204A` | `0x1.01b839a25204ap-96` | `1.2706558760139879e-29` |
| `MAGIC` | `0x4338000000000000` | `0x1.8p+52` | `6755399441055744.0` |
| `LIMIT` | `0x41C0000000000000` | `0x1p+29` | `536870912.0` |
| `TINY` | `0x3E40000000000000` | `0x1p-27` | `7.450580596923828e-09` |

`MAGIC` is `1.5 * 2^52`. Adding it to a value `t` with `|t| < 2^51` pushes the
fractional bits off the end of the significand, so `(t + MAGIC) - MAGIC` is `t`
**rounded to the nearest integer, ties to even**, using only an add and a subtract
(section 3.4 explains). `INV_PIO2` is the same double as fdlibm's `invpio2`; the
four 24-bit chunks are a different split from fdlibm's 33-bit one, which covers `k`
only below 2^20 (OQ-13). All five reduction constants were derived here from `pi`
computed to 400 bits (section 6.4), not copied.

**Series coefficients.** `S[n] = (-1)^n / (2n+1)!` for `n = 1..9`, and
`C[n] = (-1)^n / (2n)!` for `n = 2..9`, each rounded **once** to the nearest double
from the exact rational. (The `n = 1` cosine coefficient, `-1/2`, is applied as
`0.5 * z`, which is exact.)

| Name | Value | Bits | Shortest decimal |
|---|---|---|---|
| `S[1]` | -1/3! | `0xBFC5555555555555` | `-0.16666666666666666` |
| `S[2]` | +1/5! | `0x3F81111111111111` | `0.008333333333333333` |
| `S[3]` | -1/7! | `0xBF2A01A01A01A01A` | `-0.0001984126984126984` |
| `S[4]` | +1/9! | `0x3EC71DE3A556C734` | `2.7557319223985893e-06` |
| `S[5]` | -1/11! | `0xBE5AE64567F544E4` | `-2.505210838544172e-08` |
| `S[6]` | +1/13! | `0x3DE6124613A86D09` | `1.6059043836821613e-10` |
| `S[7]` | -1/15! | `0xBD6AE7F3E733B81F` | `-7.647163731819816e-13` |
| `S[8]` | +1/17! | `0x3CE952C77030AD4A` | `2.8114572543455206e-15` |
| `S[9]` | -1/19! | `0xBC62F49B46814157` | `-8.22063524662433e-18` |
| `C[2]` | +1/4! | `0x3FA5555555555555` | `0.041666666666666664` |
| `C[3]` | -1/6! | `0xBF56C16C16C16C17` | `-0.001388888888888889` |
| `C[4]` | +1/8! | `0x3EFA01A01A01A01A` | `2.48015873015873e-05` |
| `C[5]` | -1/10! | `0xBE927E4FB7789F5C` | `-2.755731922398589e-07` |
| `C[6]` | +1/12! | `0x3E21EED8EFF8D898` | `2.08767569878681e-09` |
| `C[7]` | -1/14! | `0xBDA93974A8C07C9D` | `-1.1470745597729725e-11` |
| `C[8]` | +1/16! | `0x3D2AE7F3E733B81F` | `4.779477332387385e-14` |
| `C[9]` | -1/18! | `0xBCA6827863B97D97` | `-1.5619206968586225e-16` |

A quick transcription check: wherever `2n+2` is a power of two the factorials
differ by that power, so the two coefficients share a significand and differ only in
exponent and sign. `S[3]` and `C[4]` (`1/7!` and `1/8!`, ratio 8) both end
`...A01A01A01A01A`; `S[7]` and `C[8]` (`1/15!` and `1/16!`, ratio 16) both end
`...AE7F3E733B81F`. If a port has one of a pair wrong, the pair will not line up.

**Degree constants.**

| Name | Bits | Hex float | Shortest decimal |
|---|---|---|---|
| `RAD_PER_DEG` | `0x3F91DF46A2529D39` | `0x1.1df46a2529d39p-6` | `0.017453292519943295` |
| `DEG_PER_RAD` | `0x404CA5DC1A63C1F8` | `0x1.ca5dc1a63c1f8p+5` | `57.29577951308232` |

Both equal the double nearest the real `pi/180` and `180/pi`, and (checked) equal
the double computed as `PI / 180.0` and `180.0 / PI` from the pinned `PI`. A port
may therefore keep writing `PI / 180.0` and the compiler's constant folding gives
the same bits, but the spec pins the bits so no one has to reason about it.

**Existing constants, pinned.** `PI = 0x400921FB54442D18` (`3.141592653589793`),
`TWO_PI = 0x401921FB54442D18`, `HALF_PI = 0x3FF921FB54442D18`. Tan and Part II use
`1e-15 = 0x3CD203AF9EE75616`, `1e308 = 0x7FE1CCF385EBC8A0`, `1e-17 = 0x3C670EF54646D497`,
`1e-300 = 0x01A56E1FC2F8F359`. (`TINY` equals the existing `atan` threshold
`7.450580596923828e-9`, which is `2^-27` exactly.)

### 3.3 Special values

These are the defined results; every port returns exactly these, and the vectors
(section 6) test each.

| Input `x` | `sin(x)` | `cos(x)` |
|---|---|---|
| NaN (any sign, quiet or signalling, any payload) | a quiet NaN | a quiet NaN |
| `+inf`, `-inf` | a quiet NaN (invalid operation) | a quiet NaN |
| `+0` | `+0` | `1` |
| `-0` | **`-0`** | `1` |
| `0 < \|x\| < TINY` (`2^-27`), including every subnormal | `x` (exactly) | `1` |
| `TINY <= \|x\| <= LIMIT` (`2^29`) | section 3.6 (the accurate domain) | section 3.6 |
| `LIMIT < \|x\| <= max finite` | section 3.6 with `x` first **folded** (3.4.1): finite, in [-1, 1], **no accuracy claim** | same |

A **quiet NaN** result is compared as a class (`isNaN`), never by bits, in every
port's tests (VER04p 3.3: payload and sign are not portable); the Warrant model
returns `0x7FF8000000000000`.

Why `sin(x) = x` below `TINY`: the next term of the series is `x^3/6`, which for
`|x| < 2^-27` is below `|x| * 2^-55`, under half an ulp of `x`, so the correctly
rounded result is `x`. Returning it early also (a) keeps `-0` and subnormals exact
without depending on how a CPU treats underflow, and (b) is the only thing that
makes `sin(-0) = -0` true: the general path computes `r + (r*z)*p`, and
`-0 + +0 = +0`. The same threshold makes `cos` return `1` (its first correction,
`x^2/2 < 2^-55`, rounds away at the double below 1).

### 3.4 Range reduction

A sine is a repeating wave: knowing `sin` on [-pi/4, pi/4] and which quarter-turn
you are in is enough to know it everywhere. Reducing means answering two
questions: *how many quarter-turns fit in `x`?* (an integer `k`) and *what is left
over?* (`r = x - k*pi/2`, with `|r| <= pi/4`).

**Step 1: the quotient.**

```text
t = x * INV_PIO2                 -- one rounding; x / (pi/2), approximately
k = (t + MAGIC) - MAGIC          -- nearest integer to t, ties to even; |k| <= 3.5e8
```

Why the add-and-subtract: `MAGIC = 1.5 * 2^52` has its units digit in the last
place of the significand. `t + MAGIC` therefore rounds `t` to a whole number, in
hardware, by the rule IEEE already defines (ties to even). The following `- MAGIC`
is exact and hands back the integer as a double. It needs only `+` and `-`: no
`floor`, no `round`, no `trunc`, no float-to-integer cast, and so no
language-dependent behaviour at NaN, infinity or out-of-range. It is valid for
`|t| < 2^51`; after the fold below `|t| <= 2^29 * 0.6366 < 3.5e8`.

This is the one place the backlog row said "through truncation" and the spec says
"to nearest". Truncating `t` leaves `r` anywhere in (-pi/2, pi/2) (twice the
interval, and the series then needs more terms, or the argument must be reduced a
second time); rounding to nearest leaves `|r| <= pi/4` at no extra cost.

**Step 2: the remainder, by Cody-Waite subtraction.**

```text
r = x - k * PIO2_1               -- exact: k*PIO2_1 is exact, and x and k*PIO2_1 are
                                 -- within a factor of 2 (Sterbenz) for |k| >= 1
r = r - k * PIO2_2
r = r - k * PIO2_3
r = r - k * PIO2_4
r = r - k * PIO2_5               -- the only product that rounds
```

`pi/2` is irrational, so `k * (pi/2)` cannot be formed exactly in 53 bits. The
trick is to store `pi/2` as a *sum* of pieces, each so short (24 bits) that
multiplying by an integer `k` below 2^29 needs at most 53 bits and is therefore
**exact**. Each subtraction removes one more slice of `k*pi/2` from `x`; because
`r` shrinks by about 24 bits per line, the intermediate differences have few
significant bits and (the standard Cody-Waite argument, here backed by the
near-multiple search of 4.2) lose nothing. The one product that rounds is the last
(`PIO2_5` is a full 53-bit double). The arithmetic
carries `pi/2` to 149 bits, which is more than the 29 bits of `k` plus the roughly
60 bits by which the worst double can sit next to a multiple of `pi/2`.

```text
   x --------------------------------------------------------- the angle
   |-- k*PIO2_1 --|                                             24 bits of k*pi/2
                  |-- k*PIO2_2 --|                              next 24
                                 |-- k*PIO2_3 --|               next 24
                                                |- k*PIO2_4 -|  next 24
                                                             |k*PIO2_5| last, rounds
   r = what is left, |r| <= pi/4
```

**Step 3: the quadrant.** `q = int(k) & 3`. `k` is an integer-valued double with
`|k| < 2^29`, so `int(k)` is exact in every language (a 32-bit integer suffices);
`& 3` on a two's-complement integer is the floor-modulus 4 (so `k = -1` gives
`q = 3`, `k = -5` gives `q = 3`). This is the only integer operation in the
algorithm.

#### 3.4.1 Folding huge arguments

If `|x| > LIMIT`, replace `x` by `x * 0.5` and test again; repeat until
`|x| <= LIMIT`.

```text
fold(x):  while |x| > LIMIT:  x = x * 0.5
          return x
```

Each step is an exact exponent decrement, so the loop is exact and terminates in
at most 995 iterations (`max finite = 2^1024 * (1 - 2^-53)`). An implementation may
instead scale once by `2^-j` for the smallest `j >= 0` with `|x| * 2^-j <= 2^29`
(`ldexp`, `scalbn`, exponent bit surgery); the result is the same double.

**What this is and is not.** The folded value is **not** the sine of the original
angle. Nothing a 53-bit multiply can do recovers `x mod 2*pi` for `x = 1e22`; the
real remedy is a reduction carrying `pi` to over a thousand bits (Payne-Hanek;
OQ-1). The fold is the simplest rule that is (a) **total**: finite and in [-1, 1]
for every finite input, which the shared PHY01 corpus requires of `sin` (its
`extreme-finite-bounded` and `subnormal-period-overflow` cases feed `sin` angles
near 1.8e308 and demand a finite, amplitude-bounded result); (b) **deterministic
to the bit** in all languages; (c) **honest**: the spec claims no accuracy above
`LIMIT` (4.2), and the conformance vectors mark those rows `cr = -`.

The alternatives (return NaN; implement Payne-Hanek) and why the recommended
default is the fold are in OQ-1.

### 3.5 The series kernels

On `|r| <= pi/4` (about 0.7854), `sin` and `cos` are polynomials in `z = r*r`:

```text
sin(r) = r + r*z*(S1 + S2*z + S3*z^2 + ... + S9*z^8)       -- x^1 .. x^19, ten terms
cos(r) = 1 - z/2 + z^2*(C2 + C3*z + ... + C9*z^7)          -- x^0 .. x^18, ten terms
```

The first omitted terms are `r^21/21! < 1e-19 * r` for `sin` and `r^20/20! < 4e-21`
for `cos`, each far below half an ulp, so ten terms is already more than rounding
can resolve; the count is fixed so every port adds the same terms.

Each polynomial is evaluated by **Horner's rule**, innermost coefficient first:

```text
sinK(r):                                 cosK(r):
    z = r * r                                z = r * r
    p = S[9]                                 q = C[9]
    for n = 8 down to 1:                     for n = 8 down to 2:
        p = p * z                                q = q * z
        p = p + S[n]                             q = q + C[n]
    w = r * z                                u = 0.5 * z
    w = w * p                                u = 1.0 - u
    return r + w                             v = z * z
                                             v = v * q
                                             return u + v
```

**Why Horner on stored coefficients.** (a) It adds the smallest terms first, which
is the accurate order, and the recurrence "build each term from the previous one"
that the ports use today accumulates a rounding error per term and needs a division
per term. (b) There is no division at all, so no zero-divisor trap in Python, Perl
or the BEAM and no "reciprocal then multiply" compiler substitution (VER04p P7).
(c) It removes finding F3: one multiply and one add per step cannot be grouped two
ways. (d) The coefficient table is the Maclaurin series, `1/3!`, `1/5!`, ..., so
the educational shape of the package is preserved: a reader can see the factorials
in the table.

**Why `r + r*z*p` and `(1 - z/2) + z^2*q`.** The leading term is added *last* and
unrounded in the polynomial, so the dominant part of the result (`r`, or
`1 - z/2`) carries one rounding, and the correction, which is smaller than 0.25 of
the result, carries its own much smaller rounding. `0.5 * z` is exact.

### 3.6 `sin` and `cos`

```text
sin(x):
    if x is NaN:                   return NaN
    if |x| = +inf:                 return NaN
    if |x| < TINY:                 return x                 -- includes +0, -0, subnormals
    x = fold(x)                                             -- 3.4.1, a no-op unless |x| > LIMIT
    (k, r) = reduce(x)                                      -- 3.4 steps 1 and 2
    q = int(k) & 3
    q = 0:  return  sinK(r)
    q = 1:  return  cosK(r)
    q = 2:  return -sinK(r)
    q = 3:  return -cosK(r)

cos(x):
    if x is NaN:                   return NaN
    if |x| = +inf:                 return NaN
    if |x| < TINY:                 return 1.0
    x = fold(x)
    (k, r) = reduce(x)
    q = int(k) & 3
    q = 0:  return  cosK(r)
    q = 1:  return -sinK(r)
    q = 2:  return -cosK(r)
    q = 3:  return  sinK(r)
```

The quadrant table is the identity set `sin(r + k*pi/2) = {sin r, cos r, -sin r,
-cos r}[k mod 4]` and `cos(r + k*pi/2) = {cos r, -sin r, -cos r, sin r}[k mod 4]`.
The negations are exact. `|x| = +inf` is tested as `|x| > max_finite` or with the
language's `isinf`; either is acceptable.

**Order of tests matters at one point:** NaN first (a NaN fails every
comparison, so it would fall through `|x| < TINY` and the fold test into the
reduction and reach `int(k)`, which raises in Python and is undefined in C), then
infinity (the fold loop on an infinity would never terminate), then `TINY`, then the
fold.

### 3.7 `radians` and `degrees`

```text
radians(d) = d * RAD_PER_DEG          -- one multiply
degrees(r) = r * DEG_PER_RAD          -- one multiply
```

NaN propagates, `+-inf` maps to `+-inf`, `+-0` keeps its sign, a result below the
smallest subnormal rounds to a signed zero, and a product above `max finite`
overflows to `+-inf`. (`degrees(1.7e308)` is `+inf`; in Elixir the multiply raises
`ArithmeticError`, section 5.)

Why one multiply by a stored constant (B), not `d * PI / 180` (A): (a) one
rounding, not two; (b) no intermediate `d * PI` that overflows for `d > 5.7e307`
though `radians(d)` is finite; (c) it is *more accurate*: measured over about 21,000
inputs, B's worst error is **0.65 ulp** for `radians` and **0.81 ulp** for
`degrees`, against **1.52** and **1.46** for A (section 4.3); (d) it reproduces
`radians(180) = PI` and `degrees(PI) = 180` exactly (both camps do) *and* round
trips `degrees(radians(n)) = n` for 1275 of the 1441 integers in [-720, 720]
against 1105 for A. (B is also what nine of the seventeen ports already write, so
eight ports change and nine do not.)

### 3.8 `tan`

```text
tan(x):
    s = sin(x)
    c = cos(x)
    if |c| < 1e-15:                         -- false when c is NaN
        if s > 0:  return  1e308
        else:      return -1e308
    return s / c                            -- the only division in Part I
```

This is the existing contract (PHY00 3.6: a documented sentinel at the poles, not
`+-inf`) with the new `sin` and `cos`. `tan(+-0) = +-0`, `tan(NaN) = NaN`,
`tan(+-inf) = NaN`. The guard makes `c = 0` unreachable, so Python's, Perl's and
Lua's zero-division behaviour never matters. Lua and Perl gain the guard.

### 3.9 Part II: `sqrt`, `atan`, `atan2` (unchanged, pinned)

The survey (read, not run) finds these already written the same way: sixteen of
seventeen ports for `atan` and all seventeen for `sqrt` and `atan2` (the vectors
will confirm it; Part II was not run in the prototypes of 3.11). The reference adopts
the existing majority text **verbatim**, so no port's `sqrt`, `atan` or `atan2`
changes except Dart's `atan` grouping; their bits are pinned by vectors so they
cannot drift again. The operations, in the arithmetic model of 3.1 (division and
comparison included):

```text
sqrt(x):                      -- precondition x >= 0 (else a language-idiomatic error, 3.9.2)
    if x = 0:        return x                          -- keeps -0
    if x = +inf:     return x
    scaled = x;  scale = 1.0
    while scaled < 0.25:   scaled = scaled * 4.0;   scale = scale * 0.5
    while scaled >= 4.0:   scaled = scaled * 0.25;  scale = scale * 2.0
    guess = scaled if scaled >= 1.0 else 1.0
    repeat up to 60 times:
        next = (guess + scaled / guess) / 2.0
        if |next - guess| < 1e-15 * guess + 1e-300:  return next * scale
        guess = next
    return guess * scale

atanCore(x):                  -- |x| <= 1
    reduced = x / (1.0 + sqrt(1.0 + x * x))
    t = reduced;  tsq = t * t;  term = t;  result = t
    for n = 1 to 30:
        term = term * (-tsq) * (2n-1) / (2n+1)         -- left to right: ((term*(-tsq))*(2n-1))/(2n+1)
        result = result + term
        if |term| < 1e-17:  break
    return 2.0 * result

atan(x):
    if |x| <= TINY:   return x                         -- keeps -0 and subnormals
    if x > 1:         return HALF_PI - atanCore(1.0 / x)
    if x < -1:        return -HALF_PI - atanCore(1.0 / x)
    return atanCore(x)

atan2(y, x):
    x > 0:                    atan(y/x)
    x < 0 and y >= 0:         atan(y/x) + PI
    x < 0 and y < 0:          atan(y/x) - PI
    x = 0 and y > 0:          HALF_PI
    x = 0 and y < 0:          -HALF_PI
    otherwise (x = 0, y = 0): 0.0
```

Each `(2n-1)` and `(2n+1)` is the exact small integer converted to double. Dart's
`atan` changes to the left-to-right form above.

#### 3.9.1 Honest caveat

These are **not** shown accurate by this document; PHY00 3.7 to 3.9 describes them
and their tests use 1e-10. The convergence claim is only that all seventeen compute
the same bits. Improving their accuracy is out of scope (a later amendment may
reuse the fold and quadrant machinery).

#### 3.9.2 What stays language-idiomatic

How `sqrt` of a negative number signals (Rust panics, Python raises `ValueError`,
Haskell returns `Left`, C returns a status code) is unchanged; the vectors list the
case with `expect = error` and the port's test asserts "an error of the port's own
kind". Names (`sin_approx` in Perl, `Trig.sin` in Swift) are unchanged. Perl's
`sin_deg(d)` is defined as `sin(radians(d))` and likewise `cos_deg`, `tan_deg`.

### 3.10 A worked example

`sin(1.0)`. Bits of `x` are `0x3FF0000000000000`.

```text
t = x * INV_PIO2             = 0x3FE45F306DC9C883   (0.6366...)
k = (t + MAGIC) - MAGIC      = 1.0                  (0.6366 rounds up to 1)
q = 1 & 3                    = 1                    -- so sin(x) = cosK(r)
r = x - k*PIO2_1             = 0xBFE243F680000000   exact
r = r - k*PIO2_2             = 0xBFE243F6A8885A00
r = r - k*PIO2_3             = 0xBFE243F6A8885A31
r = r - k*PIO2_4             = 0xBFE243F6A8885A31   (nothing left to remove at this scale)
r = r - k*PIO2_5             = 0xBFE243F6A8885A31   = -0.5707963267948967 = 1 - pi/2
z = r * r                    = 0x3FD4DA0BABD0C632
cosK(r)                      = 0x3FEAED548F090CEE   = 0.8414709848078965
sin(1.0) = cosK(r)           = 0x3FEAED548F090CEE   equals the correctly rounded value
```

`sin(1.0)` is `cos(1 - pi/2)`: one radian is a bit more than one quarter-turn, so
the sine is the cosine of the small leftover angle. Three more, as a check on a
reader's implementation (all verified; `CR` is the correctly rounded value):

| `x` (bits) | `k`, `q` | `r` | result bits | `CR` |
|---|---|---|---|---|
| `3.0` (`0x4008000000000000`) | 2, 2 | `0xBFC21FB54442D184` | `sin = 0x3FC210386DB6D55B`, `cos = 0xBFEFAE04BE85E5D2` | same, same |
| `1e6` (`0x412E848000000000`) | 636620, 0 | `0xBFD6E254D0F6B399` | `sin = 0xBFD6664B2568D868`, `cos = 0x3FEDF9DF9906D32C` | sin `...867` (1 ulp), cos same |
| `PI` (`0x400921FB54442D18`) | 2, 2 | `0xBCA1A62633145C07` | `sin = 0x3CA1A62633145C07`, `cos = 0xBFF0000000000000` | same, same |
| `1e20` (folded by 38 halvings to 363797880.7091713) | | | `sin = 0x3F7F67A58C59426F`, `cos = 0x3FEFFFC25B83B6D3` | none (no claim) |

`sin(PI)` returns `1.2246467991473532e-16`, the true sine of the *double* `PI`
(which is slightly less than the real pi): nothing in the algorithm hides that, and
it is the value a numerical analyst wants.

### 3.11 What was verified while writing this

The algorithm was prototyped in Python (the oracle) and re-implemented from the
pseudo-code of 3.3 to 3.8 in **nine other languages**: C (`gcc -O2
-ffp-contract=off`, and again with default flags), JavaScript (Node 22), Lua 5.4,
Perl 5, Ruby 3, Rust (`rustc -O`), Java 21, Go 1.24 (with the explicit `float64()`
conversions of section 5), and Elixir 1.18 on OTP 27. They were run on **30,025 inputs** (the
edges of 6.3, 30,000 pseudo-random values in four classes including raw random bit
patterns) for `sin`, `cos`, `radians` and `degrees`: **every result agrees bit for
bit** (NaN compared as a class; Elixir on the 30,014 finite inputs, with its 21
raised `ArithmeticError`s all falling where the true result is non-finite).
Compiled with `-mfma -ffp-contract=fast`, the same C changes **225 of 30,025 `sin`
and 208 `cos` results**, confirming the hazard of section 5 and that the vectors
see it. Haskell, Swift, C#, F#, Kotlin, Dart and C++ were not run; the first
conformance PR of each is where they are (section 8). The throwaway harnesses are
not committed; the committed generator (6.4) supersedes them.

## 4. Accuracy

### 4.1 The statement

Over the accurate domain `TINY <= |x| <= 2^29`, the results of `sin` and `cos` are
within **3 ulp of the exact value**, and within **3 representable steps of the
correctly rounded value** (VER04p 5.4's `ulp_dist`), on every vector of section 6
that carries a reference. The measured maxima over 600,000 inputs are **2.21 ulp**
(`sin`) and **2.16 ulp** (`cos`) of the exact value, and **2 steps** from the
correctly rounded value (150,000 inputs). The median error is 0.28 ulp; about 83%
(`sin`) and 76% (`cos`) of results are correctly rounded, and 99% are within
1.04 ulp. `radians` and `degrees` are within **1 ulp** (measured 0.65 and 0.81).
`TINY` and below: exact. Above `2^29`: no claim (3.4.1).

This is weaker than a libm (usually under 1 ulp), and far stronger than the ports
today. Measured against the exact value, today's ports are off by **2.7e-12 to
3.9e-12 at 1e5** (387,000 to 561,000 steps) and by **3.7e-11 to 6.8e-11 at 1e6**
(658,000 to 1,221,000 steps), growing with `x`, because they subtract multiples of
a 53-bit `2*pi`; above 1e19 their behaviour is arbitrary. The new algorithm keeps
every existing PHY00 tolerance (1e-10 on [-1e6, 1e6]) satisfied, and sits inside the
pilot's planned `sin_accurate` tolerance of `approx(<=4 ulp)` (VER04p 15) with room.

An **ulp** of the exact value `v` is the spacing of doubles in the binade of `v`:
`|got - v| / 2^(e-52)` for `v in [2^e, 2^(e+1))`. VER04p 5.4's `within_ulps` counts
representable steps, which differs from this across a power of two. Both are
measured and gated in 6.4.

### 4.2 Where the error comes from

| Source | Size |
|---|---|
| final addition in the kernel (`r + w`, `u + v`) | up to 0.5 ulp |
| rounding of the polynomial term `w`, `v` | up to ~0.25 ulp |
| rounding of `r` itself (the last Cody-Waite line, `PIO2_5`, and the final subtract) | up to ~0.5 ulp relative to `r`, passed through the function's own condition |
| truncation (omitted series terms) | under 1e-3 ulp |
| representing `pi/2` | 149 bits: negligible (below 2^-90 absolute at `k = 2^29`) |

Near a multiple of `pi/2`, `r` is tiny and so is the result; the relative error
stays small because the *absolute* error of `r` is about `2^-90`. This was checked
by searching, for every binade of doubles from 2^-3 to 2^28, the doubles nearest to
multiples of `pi/2` (the best rational approximations of `pi/2` by continued
fractions): the 56 hardest found were all within **0.48 ulp**. That is a search,
not a proof; the exhaustive worst case over all 2^29-range doubles is not claimed.

### 4.3 How it was measured (and how the gate re-measures it)

For each of `sin` and `cos`, 600,000 inputs drawn from five classes with a fixed
seed (uniform [-0.8, 0.8]; uniform [-4, 4]; uniform [-2^29, 2^29]; log-uniform from
2^-27 to 2^29; and the neighbours (+-3 ulp) of `fl(k*pi/2)` for random `k` up to
3.4e8), compared with the correctly rounded value from exact rational arithmetic
(section 6.4: `pi` to 400 bits, a Taylor series with a proved remainder, a second
engine for cross-check: GNU `bc -l` at 80 digits agreed to 1e-64 on every spot
check). The `radians` and `degrees` figures are over 20,000 random values in
[-720, 720] and the 1441 integers in that range.

The committed generator (6.4) **re-measures** on every regeneration and fails if
any vector exceeds 3 ulp (`sin`, `cos`) or 1 ulp (`radians`, `degrees`), so the
numbers in this section are checked, not quoted.

## 5. Language binding notes (bit-exactness)

The algorithm uses only IEEE `+ - *`, comparisons and one integer mask. It passes
the environment probe of VER04p 9.1 where the probe passes; **the probe, not this
text, is the arbiter**. Every lane runs the thirteen probes in the same build and
process as `trig` (9.1 rules 1 and 2) and is `not-live` for float evidence
otherwise (9.2). This section names, per family, the hazards the *algorithm* meets
and the form each port must take. The conformance vectors (section 6) are the
second line of defence: they distinguish fused from unfused arithmetic (3.11).

**The fusable sites** (VER04p P2 and the `fuse-fma` fault of its section 13) are
**24**: in `sinK`, the 8 Horner steps `p*z + S[n]` and the final `r + (r*z)*p`;
in `cosK`, the 7 Horner steps `q*z + C[n]`, the `1.0 - 0.5*z`, and the final
`u + (z*z)*q`; in the reduction, the 5 steps `r - k*PIO2_i` and the `t + MAGIC`
where `t = x*INV_PIO2`. The pseudo-code writes each product on its own line. A
conforming port makes sure the language keeps them separate; the table says how.
(The generator's fusion-sensitivity gate, 6.4, checks that each of the 24 matters
to at least one vector.)

| Family | Rules for the port |
|---|---|
| **C, C++** | `-ffp-contract=off` **always**: GCC's default in GNU C mode is `fast`, clang's is `on`, and MSVC needs `#pragma fp_contract(off)` with `/fp:precise` or `/fp:strict`, never `/fp:fast`. No `-ffast-math`, `-Ofast`, `-funsafe-math-optimizations`, `-fassociative-math`, `-ffinite-math-only`. `FLT_EVAL_METHOD == 0` asserted at compile time (`static_assert` in C++, `_Static_assert` in C11); x87 targets are refused (VER04p 12.2). Do not use `long double`, `fma`, `fmod`, `floor`, `round`, `trunc` from `<math.h>` (the algorithm needs none; the existing C port deliberately has no `<math.h>`). Constants as hex float literals (C99, C++17) or decimal with the bit assert. `int(k)` is `(int)k` after the fold has bounded `k`: the cast is defined for `\|k\| < 2^29`. The existing `d_trunc` helper disappears. |
| **Rust** | no contraction by default; nothing to do but avoid `f64::mul_add`. `(k as i64) & 3`. Constants as `f64::from_bits(0x...)` in a `const` or as decimal. i586 x87 targets refused by the probe. |
| **Go** | `x*y + z` **may be fused** on arm64, ppc64, s390x, riscv64 and others, and there is no flag. **An intermediate variable does not stop it:** measured on `GOARCH=arm64`, `t := x*INV_PIO2; k := (t + MAGIC) - MAGIC` compiled to a fused `FMADDD`; wrapping the product, `t := float64(x * INV_PIO2)`, removes it, and wrapping all fusable sites as `float64(p*z) + S[n]`, `r + float64(w*p)`, `y - float64(k*PIO2_1)` leaves **zero** `FMADD`/`FMSUB` in the arm64 assembly. (Go spec: an explicit conversion rounds.) Each port PR for Go adds the check `GOARCH=arm64 go build -gcflags=-S . 2>&1 \| grep -c 'FMADD\|FMSUB\|FNMSUB'` equal to 0 as a CI step, and VER04p FL-16 stands: Go float *evidence* is amd64-only. Do not write constant expressions that combine the pinned constants (Go evaluates constant expressions exactly, with arbitrary precision, at compile time; 6.5's bit assert catches a mistake). `int64(k) & 3`; `math.NaN()` for NaN. |
| **Java, Kotlin (JVM)** | Java 17+ is strict by default (JEP 306); older JVMs are refused or the class is `strictfp` and probed. The JIT never fuses `a*b + c`; do not call `Math.fma`. Java has hex float literals (`0x1.45f306dc9c883p-1`); Kotlin does not: use `java.lang.Double.longBitsToDouble(0x...L)` or the decimal with the bit assert. `((int) k) & 3`, Kotlin `k.toInt() and 3`. `Double.NaN`. |
| **C#, F#** | .NET Core 3.0+ and .NET 5+ (RyuJIT, SSE2). The JIT does not contract; do not call `Math.FusedMultiplyAdd`. C# and F# have no hex float literal: decimal literals are parsed correctly rounded since .NET Core 3.0, and the bit assert (`BitConverter.DoubleToInt64Bits`) guards them. `(long)k & 3L`, F# `int64 k &&& 3L`. |
| **JavaScript / TypeScript** | only binary64 exists; no flags matter; V8, SpiderMonkey and JavaScriptCore do not contract. `k & 3` (ToInt32) is exact for `\|k\| < 2^30`, and floor-mod 4 for negatives. Literals are parsed correctly rounded (ECMAScript). `NaN`. No `Math.fround`, no `Math.sin`/`Math.floor`/`Math.round`. The loop counter for Horner is a small integer, not part of the numeric result. |
| **Python** | CPython `float` is binary64; no contraction. `int(k) & 3` (exact, floor-mod for negatives). `abs(x) == float('inf')` rather than `math.isinf` (either is fine), `x != x` for NaN. **No division occurs, so Python's raise-on-zero-divide never triggers** (`tan`'s guard keeps `c != 0`). Never call `int()` on a non-finite value (it raises): the guards in 3.6 run first. `repr(float)` constants round-trip. |
| **Haskell** | GHC with the default native code generator on x86-64 (SSE2) or LLVM without fast-math; no `-fexcess-precision`; 32-bit x86 refused. GHC fuses only an explicit `fmaddDouble`. `truncate k :: Int` then `.&. 3` (`Data.Bits`): exact on an integer-valued double. Constants by `castWord64ToDouble 0x...` (`GHC.Float`) or decimal (Haskell's literal is correctly rounded via `fromRational`) with the bit assert. Do **not** use `mod'` or `floor` for reduction (the current port does; `mod'` multiplies `fromInteger (floor (x/y))` by `y`, a product that rounds for large `x`, and its behaviour at infinity is not the NaN of 3.3). |
| **Elixir / Erlang (BEAM)** | floats cannot be NaN or infinite. A NaN or infinity **input** is unrepresentable as a float (it crosses as bits only); the vectors mark those rows `nonfinite = unrepresentable` and the port's test skips them. A result that would be non-finite (`degrees(1.7e308)`, which is `+inf`) **raises `ArithmeticError`**; the vector expects `+inf` and the harness maps "raises" to that expectation (`nonfinite = rejects`, VER04p 12.2). `sin`/`cos`/`tan` of a finite value are always finite (the fold guarantees it), so no raise is expected there. Integer arguments are converted first with `x * 1.0` (never `x + 0.0`: that turns `-0.0` into `+0.0`). `trunc(k) &&& 3` (`Bitwise.band`; two's complement, floor-mod for negatives). Constants as `<<0x...::float-64>>` patterns are exact on every OTP; OTP 27 distinguishes `0.0` from `-0.0` in `===` and pattern matches, so adapters compare bits. |
| **Lua** | build with `LUA_FLOAT_TYPE = double`. **On 5.3+ an integer argument is an integer subtype**: `-x*x` is integer arithmetic and wraps. Convert once at entry with `if math.type(x) == "integer" then x = x * 1.0 end` (not `x + 0.0`; see Elixir). `math.tointeger(k) & 3` (5.3+); on 5.1/LuaJIT, `k - 4*math.floor(k/4)` (`k` integer-valued, so exact). Lua has hex floats since 5.2. `0.0/0.0` makes a NaN (with the sign bit set on x86: compared as a class). No `math.fmod`, `math.modf`, `math.floor` in the algorithm. |
| **Perl** | refuse a build whose `NV` is not 8 bytes (`$Config{nvsize} == 8`, VER04p 12.2). Perl dies on division by zero (no division occurs). `int($k) & 3`: for a negative integer, `&` works on the two's-complement 64-bit pattern, so `& 3` is floor-mod 4 (checked). **Perl's own decimal-to-NV parser has historically not been correctly rounded on every platform**, so build the constants from bits: `unpack('d<', pack('Q<', 0x...))`, once at module load, and assert them. NaN: `my $inf = 9**9**9; my $nan = $inf - $inf`. Integer values stay integer in Perl (`180.0` multiplies as an NV, but a literal `180` takes an integer path): write every constant as a float, which is why `degrees(-0.0)` is `+0` today. |
| **Ruby** | `Float` is binary64; no contraction. `k.to_i & 3`. `x.nan?`, `x.abs.infinite?`. Decimal float literals are parsed correctly rounded. Convert an `Integer` argument with `x.to_f` (preserves `-0.0` for floats). |
| **Swift** | `Double`; never `-Ounchecked`; Swift does not contract (`addingProduct` is explicit and is not used). `Int(k) & 3` (traps only on non-finite `k`, which the guards exclude). Hex float literals (`0x1.8p52`) are supported. `Double.nan`. |
| **Dart** | VM: native; web (`dart2js`): numbers are JavaScript `number`s and an `int` is a double that bitwise operators truncate to 32 bits, which is why `LIMIT = 2^29` (`k < 2^31`): `k.toInt() & 3` is exact on both. `double.nan`. Dart has no hex float literal: decimal with the bit assert (`ByteData.setFloat64` / `getUint64`). Never `%` on doubles in this algorithm (Euclidean). |

**Cross-cutting.** (1) *Literals*: a decimal constant is accepted in a language only
if its bits are asserted in the port's tests (6.5), because "the literal parses to
the nearest double" is a property of the compiler, not of the code. (2) *Subnormals*:
`TINY` and the early return keep `sin` away from underflow, but `radians`/`degrees`
of a subnormal produce subnormals or zeros; a lane with flush-to-zero set fails
probes P3/P4 and is `not-live`, which is a correct result. (3) *Constant folding*:
compilers fold `0.5 * z`-style expressions with the same rounding as run time on
IEEE targets; probe P11 detects the exceptions. (4) *Loop unrolling and
vectorisation of the Horner loop* may not reassociate (P6): the dependence chain is
the algorithm. (5) *The fold loop*: at most 995 iterations; a language may use its
exact `scalbn`/`ldexp` instead, and the result is the same double.

**The Warrant model** (VER04p 8, 15) is expressed with `fmul`, `fadd`, `fsub`,
`fneg`, `fabs`, ordered comparison and `f_to_int` on an integer-valued `k` with
`|k| < 2^29` (exact), plus an integer `mod 4`. It needs no `sqrt` and no division
for `sin`, `cos`, `radians`, `degrees`. The fold is a bounded `fold(range(995), ...)`
that leaves `x` unchanged once `|x| <= LIMIT`. The decision FL-15 text "reduction
(truncation through `f_to_int`)" is therefore satisfied differently: `k` is
integer-valued by construction (`MAGIC`), and `f_to_int` is applied only to it.

## 6. Conformance vectors

### 6.1 What exists, what this adds

`fixtures/phy00-phy01-v1` is a shared corpus of decimal-tagged cases with
**tolerances** (absolute 1e-10), and PHY00 5.1 says it "may not be weakened or
reinterpreted". It stays exactly as it is: the converged ports still pass it. This
spec adds a **second, bit-exact corpus**, `fixtures/trig-reference-v1`, that every
port must match to the bit. The two answer different questions: "is it a sine?" and
"is it *this* sine?".

### 6.2 Layout and encoding

```text
code/specs/fixtures/trig-reference-v1/
  README.md              the rules below, the regeneration command
  manifest.json          closed schema: version, algorithm id "TRIG-REF-1", the constant
                         table (name -> 16 lowercase hex digits), the groups (name, file,
                         row count, sha256 of the file), generator version, the domain
                         hashes of 6.3
  schema.json            JSON Schema 2020-12 for manifest.json
  edge.tsv               the edge group (6.3)
  sweep-binade.tsv       ┐
  sweep-cw.tsv           ├ the sweep groups (6.3)
  sweep-prng.tsv         ┘
```

Every vector is one line of tab-separated fields (no quoting, no escapes, a final
newline, `\n` line endings, UTF-8 without BOM):

```text
op <TAB> x <TAB> y <TAB> cr <TAB> flags
```

| Field | Content |
|---|---|
| `op` | `sin`, `cos`, `tan`, `radians`, `degrees`, `sqrt`, `atan`, `atan2` |
| `x` | exactly 16 **lowercase** hex digits: the input's IEEE bits. `atan2` has two inputs, `x` then `y`, as `x` field `"<y>,<x>"`: 33 characters |
| `y` | the **reference result bits** (the TRIG-REF-1 result), 16 lowercase hex digits; a NaN result is written `7ff8000000000000` and **compared as a class**; the word `error` for a `sqrt` of a negative |
| `cr` | the correctly rounded value of the *mathematical* function at the input, as 16 hex digits, or `-` when none is claimed (a NaN or infinite result; `\|x\| > LIMIT`; `tan` poles; Part II ops) |
| `flags` | comma-separated, may be empty: `nan-class`, `nonfinite=unrepresentable` (an input that cannot exist as a BEAM float), `nonfinite=rejects` (a result that raises in BEAM), `tier2` (`\|x\| > LIMIT`), `signed-zero` |

Decimal numbers, exponent letters, `0x` prefixes, uppercase hex and whitespace
inside a field are a **corpus error** (VER04p 10: one value has one spelling, so
hashes are stable). The validator rejects them mechanically.

### 6.3 Contents

**Edge group (`edge.tsv`, about 3,000 lines).** For each of `sin`, `cos`, `tan`,
`radians`, `degrees`: the whole of VER04p 11.1's `f64_edge` list (about 214 values:
the zeros, subnormals and their neighbours, normals, the largest finite, the
infinities and every NaN shape, the integer boundaries 2^53 and 2^63 and 2^64, and
the halfway cases) **plus** the trig-specific points:

| Point set | Values |
|---|---|
| multiples of `pi/2` | `fl(k*pi/2)` for `k` in +-{1..8, 16, 100, 1000, 10^5, 2^20, 2^28} and the neighbours at +-1, +-2 ulp, which are the cases where `r` is smallest |
| thresholds | `+-TINY`, `+-LIMIT` and their neighbours at +-1, +-2 ulp; 2^30; 2^53 +- 1; 2^63; 2^64; 1e10; 1e20; 1e22; 1e300; `max finite` and its predecessor |
| quadrant edges | `pi/4`, `3pi/4`, `5pi/4`, `7pi/4` and neighbours (the points where `k` changes and `\|r\|` is largest) |
| common values | 0.5, 1, 2, 3, 0.1, `e`, `355/113`, 1e5, 1e6 and their negatives |
| poles for `tan` | `fl(pi/2)` and neighbours, `fl(3pi/2)`, `fl(-pi/2)` |
| conversions | 180, 90, 45, 360, `PI`, `PI/2`, `PI/4`, `2*PI` and negatives; `max finite` (overflow to `inf` in `degrees`); `min subnormal` (underflow in `radians`) |

plus a Part II set for `sqrt` (0, -0, 1, 2, 4, 0.25, 1e-300, `max finite`, `+inf`,
NaN, and negatives as `error`), `atan` (the boundaries 7.45e-9 and its neighbours,
+-1, +-2, +-1e300, +-inf, NaN) and `atan2` (the nine quadrant and axis cases with
both signs of zero).

**Sweep groups** (bounded sets, VER04p 11.1 `f64_set`; each has a name, a count and
a generator-recorded SHA-256 in the manifest):

| Group | Inputs | Count per op |
|---|---|---|
| `sweep-binade` | for every binade `e` in [-30, 30]: `2^e * (1 + j/32)` for `j = 0..31`, both signs | 3,904 |
| `sweep-cw` | for `k = 1..256`, and for 64 values `k_i` spread to 3.4e8 by a fixed rule: `fl(k*pi/2)` and its +-1, +-2 ulp neighbours, both signs; plus the continued-fraction hard cases of 4.2 | about 3,300 |
| `sweep-prng` | 2,048 raw 64-bit patterns and 2,048 uniform doubles in [-2^29, 2^29], from the specified generator (below) | 4,096 |

The PRNG is **xoshiro256\*\*** seeded with SplitMix64 of the 64-bit integer given by
the ASCII bytes `TRIG-REF` (big-endian), a fully specified, language-neutral
integer generator; the generator does not use any language's `random`. About 11,000
inputs per operation (`sin`, `cos`, `tan`, `radians`, `degrees`) and about 55,000
lines in all (about 3.5 MB); the sweep is sized so that a port's whole conformance
test runs in a few seconds even in the slowest interpreters.

**Why bounded sets, not "exhaustive"**: 2^64 doubles cannot be enumerated, and
VER04p 11.2 forbids the label. These sets are `sampled`/`bounded-set` and are
described that way everywhere.

### 6.4 How the vectors are generated, independently

The generator is `code/scripts/generate_trig_reference_fixture.py` (to be written in
slice TC-1, section 8); its validator is
`code/scripts/tests/test_trig_reference_fixture.py`. It is **independent of every
port and of every host libm**:

1. **Inputs** come from the lists above and the PRNG, as integers. No `float()`, no
   `math.sin`, no `random` is called on a corpus value (VER04p 10, "the generator
   refuses to call `float()`").
2. **`y` (reference bits)** is computed by an **integer-only softfloat model** of
   the algorithm: binary64 values as `(sign, exponent, significand)` integers, add,
   subtract and multiply defined by exact integer arithmetic and one
   round-to-nearest-even; the algorithm of 3.3 to 3.8 transcribed over those
   operations. This is the same model VER04p 8 requires per lane; until that exists
   the generator carries its own. **Cross-check:** every `y` is also computed by two
   unrelated host implementations built with fusion off (C `-ffp-contract=off`
   and Rust), and the generator **fails** on any disagreement, so a transcription
   error in any one of the three cannot reach the fixture.
3. **`cr` (correctly rounded)** is computed by **exact rational arithmetic**:
   `pi` to 400 bits (Machin's formula over big integers), the argument reduced
   modulo `2*pi` exactly (`Fraction`), a Taylor series summed in fixed point at
   220 bits with the remainder bound checked, and the result rounded to nearest-even
   by an exact comparison against the midpoints; if the interval of uncertainty
   straddles a midpoint (the table-maker's dilemma) precision is doubled and the
   row recomputed. **Cross-check:** GNU `bc -l` (`s()`, `c()` at `scale=100`) is
   run on every edge row and on a pinned 1-in-8 of each sweep, and the generator
   fails if a rounded value differs; `mpmath` or MPFR is accepted instead when
   present. For `sin(x)` this covers `|x| <= 2^29`; above that the generator would
   need `pi` to over a thousand bits and, because no accuracy is claimed there,
   `cr` is `-`.
4. **Gates, at generation time:** (a) on every row with a `cr`, the error of `y`
   against the **exact** value (the rational the generator already holds) is at
   most 3 ulp for `sin`/`cos` and 1 ulp for `radians`/`degrees`, **and** VER04p
   5.4's `ulp_dist(y, cr)` is at most 3 and 1 steps respectively (the two
   definitions differ at binade edges; both are gated); (b) **mutation sensitivity**: flipping the last bit of each constant
   in 3.2 (one at a time) must change at least one `y` in the fixture, else the
   fixture could not detect a typo in that constant and the generator fails; (c)
   **fusion sensitivity**: the model re-run with each fusable site fused
   (computed exactly then rounded once) must change at least one `y`, for each of
   the 24 sites; (d) the manifest's SHA-256s match.
5. **Determinism:** the generator's output is byte-identical on every run and
   platform; a CI job regenerates it and fails on a diff.

The statement "the vectors are independent" is checkable: the three `y` sources
share no code; the `cr` source shares no code with them; and none uses a
platform's `sin`.

### 6.5 What a port must do

Each port's test suite (a) **decodes** `x` from hex to a double by a bit cast (never
by parsing a decimal), (b) **calls** the port's public function (not an internal
one), (c) **encodes** the result back to bits and compares it to `y`
**bit-for-bit**, except that `y = 7ff8000000000000` compares as `isNaN`, and
`y = error` compares as "the port's idiomatic error"; (d) skips rows whose flags say
`nonfinite=unrepresentable` and maps "raises" to the expectation on
`nonfinite=rejects` rows (Elixir); (e) **asserts the bits of every constant it
embeds** whose language lacks hex float literals (3.2), by decoding the constant's
own bits (`Double.doubleToRawLongBits`, `struct.pack`, `string.pack`); (f) keeps its
existing example tests, except those that pin old behaviour (a 21-term sum, a
particular `sin(1e22)`), which are deleted in the PR that makes them false.

The port's tests need the vectors. A package whose `required_capabilities.json`
declares **no filesystem capability** (Dart today) must not read the TSV at test
time: the generator emits a generated source file per such lane, the same pattern as
`fixtures/phy00-phy01-v1/dart/generated_cases.dart` (a canonical base64 string
decoded by a validated loader), under the same rules (a size, nesting and range
check in the loader). Every other lane reads the TSV from the repository with the
test-only read capability its other fixtures already use. Which lanes embed is OQ-4.

### 6.6 The vectors and the pilot

The Stage C claim of VER04p 15.1 becomes `forall x in f64_edge`-shaped, over
exactly these groups: `sin_bit_exact` is `sampled f64_edge(n)` for the edge group
and `bounded-set(trig_sweep_v1, n)` for the sweeps, never `full`; `sin_accurate` is
`approx(<=3 ulp)` against the `cr` column on `|x| <= LIMIT` (the pilot's planned
4 also holds; 3 is what the generator's gate proves). The `cr` column is the
"pinned WX-E table" that VER04p 15's sketch (`sin_table`) needs; its generator is
pinned by the manifest hash.

## 7. Dependents and risk

Changing `sin` and `cos` changes the last bits of every result and, for `|x| > 1e6`,
changes the values substantially in the C family, Lua, Perl and Go (today wrong
there). Consumers include `wave`, `oscillator`, the audio and music packages
(`pcm-audio`, `note-audio`, `audio-device-sink`, `music-machine`, `percussion-kit`),
`affine2d`, `arc2d`, `bezier2d`, `cas-ode-numeric`, `macsyma-runtime` and others (a
`git grep` of build files at 046c2d8b84b lists them). Each port PR (section 8)
must:

1. list the dependents of that lane (`git grep -l 'trig' -- 'code/**/BUILD'`) and
   run their tests in the PR;
2. fix any dependent test that compares a `trig` result **exactly** (as opposed to
   within a tolerance): none is expected, and the PR says so either way;
3. note that PHY01 `wave` feeds `sin` an angle of up to about 1.8e308 in two corpus
   cases and requires a finite, amplitude-bounded result: the fold (3.4.1) keeps it
   so (the corpus's `finite-absolute-not-greater-than-amplitude` predicate passes by
   construction: `|sin| <= 1` holds for every folded argument, which the vectors
   check on the prng sweep).

## 8. Migration plan

One slice for the fixture, one pull request per port, in this order. The order is
chosen so each PR's new risk is a *new hazard class*, learned before the next lane
repeats it.

| Slice | Content | Hazard it settles | Gate |
|---|---|---|---|
| **TC-0** | this specification (merged) and the backlog row | none | review |
| **TC-1** | the fixture of section 6 (generator, validator, `edge.tsv`, sweeps, manifest, README, generated Dart source), **and** the Python port (3.3 to 3.9), whose tests become the vectors | the oracle is also a lane; the generator's cross-check against C and Rust must be green | validator and the Python tests pass; `TC-0` accepted |
| TC-2 | **rust** | compiled, strict; the model lane for VER04p's softfloat self-check | TC-1 |
| TC-3 | **typescript** | f64-only, JS `%` and `0.0 + x` quirks go away; the web side of Dart shares its semantics | TC-1 |
| TC-4 | **c** | flag hazards (`-ffp-contract=off`, `FLT_EVAL_METHOD`); the build file gains the flag and a second `-mfma` build that must **fail** the vectors (the fusion canary) | TC-1 |
| TC-5 | **cpp** | same hazards as C, header-only | TC-4 |
| TC-6 | **go** | FMA on arm64; the `float64()` rule; the arm64 assembly check as a CI step | TC-1 |
| TC-7 | **java** | strict by default; replaces `%` | TC-1 |
| TC-8 | **kotlin** | JVM again, no hex float literal | TC-7 |
| TC-9 | **csharp** | .NET, decimal literals plus bit assert | TC-1 |
| TC-10 | **fsharp** | .NET, G2 grouping disappears | TC-9 |
| TC-11 | **swift** | 21 terms to 10, `Int(k) & 3` | TC-1 |
| TC-12 | **haskell** | removes `mod'`/`floor`; constants by `castWord64ToDouble` | TC-1 |
| TC-13 | **ruby** | floored `%` disappears | TC-1 |
| TC-14 | **lua** | integer subtype, `math.modf` removed, `tan` sentinel added | TC-1 |
| TC-15 | **perl** | `int()` pitfalls, constants from bits, `tan` sentinel, `degrees(-0)` | TC-1 |
| TC-16 | **dart** | zero-filesystem embedding of the vectors; `%` Euclidean disappears; `atan` grouping | TC-1 |
| TC-17 | **elixir** | BEAM non-finite handling; `Float.round` disappears; `nonfinite` flags exercised | TC-1 |
| TC-18 | close-out: amend PHY00 sections 3.2 to 3.4, 5 and 6 (they say 20 terms and `round(x/2pi)`), the backlog row to done, VER04p 15.1 to cite this document | none | all 17 merged |

Each port PR has the same shape (`feat(trig-<lang>): converge on TRIG-REF-1`):

1. read `lessons.d` for the language (CLAUDE.md rule 7); branch from `origin/main`;
2. implement 3.3 to 3.9 **verbatim**, literate: the reduction diagram, the
   factorial table and the quadrant table go in the source comments;
3. the constants of 3.2, by bits where the language needs it, each asserted in a
   test (6.5);
4. switch the tests to the vectors; delete tests that pin superseded behaviour;
5. update `README.md` (the algorithm, the domain, the accuracy statement of 4.1)
   and `CHANGELOG` (numeric change: last-bit differences everywhere; large `|x|`
   changes materially; Lua and Perl `tan` gain the sentinel; `sin(-0) = -0`; the
   package minor version is bumped, OQ-12);
6. run the package's build, its dependents' tests (7), the lane's lint
   (`ruff`, `standardrb`, `go vet`, ...), and `/security-review`;
7. open the PR, then `/babysit-pr` until green; clean the worktree when it merges.

**Acceptance of the whole item (TRIG-CONV done):** all seventeen port PRs merged;
each port's vectors green in CI; the Go arm64 and C `-mfma` canaries green; the
generator's regeneration job green; PHY00 amended; backlog row marked done; then
(and only then) the floats pilot of VER04p, Stage C, may start.

**Parallelism.** TC-2 to TC-17 are independent once TC-1 is merged; they are listed
in an order, not a chain, and may proceed in parallel except for the stated
dependencies (TC-5 after TC-4, TC-8 after TC-7, TC-10 after TC-9). The sequence is the
*recommended* order for learning, not a constraint.

**Rollback.** Each port PR is a single commit's worth of numeric change and is
reverted by reverting that PR; the vectors do not need to change (a port that
reverts simply fails its own conformance test, loudly), and the dependents'
tests are the early warning.

## 9. Open questions

Each has a recommended default; the defaults are what this document specifies.

| # | Question | Recommended default |
|---|---|---|
| **OQ-1** | What should `sin`/`cos` return above `2^29`? (a) NaN; (b) the fold of 3.4.1; (c) a Payne-Hanek reduction carrying `pi` to ~1,200 bits, exact for every double | **(b) the fold**. NaN is the honest answer but breaks PHY01, whose corpus requires a finite, bounded result for angles near 1.8e308, and every dependent that accumulates phase (`oscillator`, `wave`, audio: a 440 Hz phase reaches 2^29 radians after about 2.2 days, and 2^20 radians after 6 minutes). Payne-Hanek is the real answer and a project of its own (a 1,200-bit table and multi-word arithmetic in seventeen languages, several of which have no big integers); the fold leaves it open as a later amendment that changes only the `tier2` rows. A later owner decision for (a) would require changing `wave` to reduce its phase first |
| **OQ-2** | Is 2.2 ulp worst case acceptable, or should the reduction carry a high and a low part to reach about 1 ulp? | **Accept 2.2 ulp.** A (hi, lo) pair adds a second polynomial and a correction term per function, and doubles the number of fusable sites, for a package whose stated precision target is 1e-10 and whose pilot tolerance is 4 ulp. The `tier` boundary of 3 ulp leaves a margin |
| **OQ-3** | Are `tan`, `sqrt`, `atan`, `atan2` in this convergence? | **Yes, Part II**, pinned unchanged (3.9); they are nearly free (one port differs) and `tan` changes anyway. A later amendment may improve their accuracy |
| **OQ-4** | Which lanes embed the vectors as generated source instead of reading the TSV? | **Only lanes with no filesystem capability** (Dart today), by the existing `phy00-phy01-v1/dart` pattern; everything else reads the TSV. Revisit if a second lane is capability-sealed |
| **OQ-5** | Should NaN results be compared by bits? | **No: as a class** (VER04p 3.3); the Warrant model returns `0x7FF8000000000000`. Ports may return any NaN |
| **OQ-6** | Is `sin(-0) = -0` the contract? | **Yes** (VER04p 3.4: "a real contract"; IEEE 754-2019 9.2.1). It is the one place the early return is needed for correctness rather than speed |
| **OQ-7** | The backlog and VER04p say sixteen ports; `origin/main` has seventeen | **Say seventeen** and cover all (the plan does). Fix the wording in TC-18 |
| **OQ-8** | Perl's extra exports and Haskell's `Either` `sqrt` | **Keep them**; they are defined in terms of the reference (3.9.2) |
| **OQ-9** | Amend PHY00 now or at the end? | **At TC-1** for the single statement that changes the algorithm (sections 3.2 to 3.4, 5 and 6 of PHY00), and again at TC-18 for the count; amending only at the end leaves a spec that contradicts every merged port in between |
| **OQ-10** | Elixir/BEAM overflow: raise, or return an atom? | **Raise** (`ArithmeticError`) as the host does; the harness's `nonfinite = rejects`. An atom would change the function's type (PHY01 already does that for one case: `:positive_infinity`, a separate decision) |
| **OQ-11** | Flush-to-zero hosts | **Not supported:** such a lane is `not-live` for floats (VER04p P3/P4); `radians(min subnormal)` is `+0` by IEEE and the vector says so |
| **OQ-12** | Is the numeric change a major version? | **A minor bump** for the 0.x packages with a loud CHANGELOG entry; the contract (a sine to 1e-10 on [-1e6, 1e6]) is unchanged and tightened |
| **OQ-13** | `LIMIT = 2^29` or `2^20`? | **`2^29`**: it keeps `k < 2^29`, which the 24-bit chunks exactly cover and a 32-bit (or JavaScript/dart2js) integer holds; `2^20` (fdlibm's three 33-bit chunks) would cover phases only to 1.05e6 radians, about 6 minutes of a 440 Hz tone. The five-constant reduction costs two more lines |
| **OQ-14** | Hex float literals in the spec vs in sources | **Spec is bit-pattern first** (this document); sources use hex floats where the language has them, else decimal plus a bit assert, else from-bits construction (Perl, Haskell, Elixir) |

## 10. Specification change record

| Date | Change |
|---|---|
| 2026-11-16 | First draft. Survey of seventeen ports at `origin/main` 046c2d8b84b (nine run, eight read). Reference algorithm TRIG-REF-1 (nearest-integer Cody-Waite with five 24-bit chunks to 2^29, folded beyond, ten-term Horner kernels on stored coefficients, quadrant selection, pre-divided degree constants, early returns for `\|x\| < 2^-27` and NaN and infinity). Prototyped in Python and nine other languages; bit-identical on 30,025 inputs. Accuracy measured at 2.21 ulp (`sin`), 2.16 (`cos`), 0.81 (`degrees`). Conformance corpus format and generator; migration plan TC-0 to TC-18; open questions OQ-1 to OQ-14 |

## Appendix A. Reference implementation (Python, normative as a transcription)

This is the listing that produced the vectors of 3.10 and was checked against the
nine other languages. It is **a transcription of the pseudo-code, not a second
source of truth**: where it and sections 3.1 to 3.8 differ, the sections win.

```python
import math, struct

def _f(bits): return struct.unpack('<d', struct.pack('<Q', bits))[0]

INV_PIO2 = _f(0x3FE45F306DC9C883)
PIO2 = [_f(b) for b in (0x3FF921FB40000000, 0x3E74442D00000000, 0x3CF8469880000000,
                        0x3B78CC5160000000, 0x39F01B839A25204A)]
MAGIC, LIMIT, TINY = _f(0x4338000000000000), _f(0x41C0000000000000), _f(0x3E40000000000000)
S = [None] + [_f(b) for b in (0xBFC5555555555555, 0x3F81111111111111, 0xBF2A01A01A01A01A,
        0x3EC71DE3A556C734, 0xBE5AE64567F544E4, 0x3DE6124613A86D09,
        0xBD6AE7F3E733B81F, 0x3CE952C77030AD4A, 0xBC62F49B46814157)]
C = [None, None] + [_f(b) for b in (0x3FA5555555555555, 0xBF56C16C16C16C17,
        0x3EFA01A01A01A01A, 0xBE927E4FB7789F5C, 0x3E21EED8EFF8D898,
        0xBDA93974A8C07C9D, 0x3D2AE7F3E733B81F, 0xBCA6827863B97D97)]
RAD_PER_DEG, DEG_PER_RAD = _f(0x3F91DF46A2529D39), _f(0x404CA5DC1A63C1F8)
NAN = _f(0x7FF8000000000000)

def _fold(x):
    while abs(x) > LIMIT:
        x = x * 0.5
    return x

def _reduce(x):
    t = x * INV_PIO2
    k = (t + MAGIC) - MAGIC
    r = x
    for p in PIO2:              # five separate multiply, then subtract
        m = k * p
        r = r - m
    return k, r

def _sin_k(r):
    z = r * r
    p = S[9]
    for n in range(8, 0, -1):
        p = p * z
        p = p + S[n]
    w = r * z
    w = w * p
    return r + w

def _cos_k(r):
    z = r * r
    q = C[9]
    for n in range(8, 1, -1):
        q = q * z
        q = q + C[n]
    u = 0.5 * z
    u = 1.0 - u
    v = z * z
    v = v * q
    return u + v

def sin(x):
    if x != x or abs(x) == math.inf: return NAN
    if abs(x) < TINY: return x
    k, r = _reduce(_fold(x))
    return (_sin_k(r), _cos_k(r), -_sin_k(r), -_cos_k(r))[int(k) & 3]

def cos(x):
    if x != x or abs(x) == math.inf: return NAN
    if abs(x) < TINY: return 1.0
    k, r = _reduce(_fold(x))
    return (_cos_k(r), -_sin_k(r), -_cos_k(r), _sin_k(r))[int(k) & 3]

def radians(d): return d * RAD_PER_DEG
def degrees(r): return r * DEG_PER_RAD
```

One subtlety the listing makes visible: `_reduce` writes the product and the
subtraction as two statements, which is the form every fusable site takes in a
port.
