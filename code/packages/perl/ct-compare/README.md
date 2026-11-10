# CodingAdventures::CtCompare (Perl)

Constant-time byte comparison and selection: the Perl lane of
[CT01](../../../specs/CT01-constant-time-compare.md).

A secret comparison that stops at the first differing byte tells an attacker how
long the matching prefix was. This module compares every byte regardless, folds
the differences into one accumulator and inspects it once at the end.

## API

```perl
use CodingAdventures::CtCompare qw(
    ct_eq ct_eq_fixed ct_select_bytes ct_eq_u64
    ct_eq_bit ct_eq_fixed_bit ct_eq_u64_bit
);

ct_eq($left, $right);                 # boolean
ct_eq_fixed($left, $right);           # boolean (same as ct_eq in Perl)
ct_select_bytes($left, $right, $bit); # byte string; $left when $bit == 1, $right when 0
ct_eq_u64($left, $right);             # boolean

ct_eq_bit($left, $right);             # 1 or 0
ct_eq_fixed_bit($left, $right);       # 1 or 0
ct_eq_u64_bit($left, $right);         # 1 or 0
```

* Byte inputs are byte strings or unblessed array references of integers
  0..255. `undef`, other references, strings with characters above 0xFF, and
  array elements that are not plain integers 0..255 (floats, `"9e1"`, negatives)
  raise an error rather than being coerced.
* A length mismatch makes `ct_eq` false (`ct_eq_bit` returns 0) without reading
  either input: lengths are public.
* `ct_select_bytes` dies with a message starting `CT_SELECT_LENGTH_MISMATCH` for
  unequal lengths, before reading either input.
* The selection bit is an **integer** that must be exactly 0 or 1. Perl has no
  branch-free boolean-to-number conversion (`$b ? 1 : 0` is a branch), so this
  lane follows Go, Ruby and Elixir. Produce the bit with the `_bit` variants:

  ```perl
  my $picked = ct_select_bytes($a, $b, ct_eq_bit($x, $y));
  ```
* Unsigned 64-bit values are non-negative decimal integers up to
  18446744073709551615 (numbers or digit strings). It needs a Perl with 64-bit
  integers. The 64-bit difference is OR-folded with a fixed shift sequence
  (`d |= d >> 32; ... d |= d >> 1`) instead of `d | -d`, because unary minus on
  a Perl unsigned value above 2**63 yields a floating point number.

`use feature 'bitwise'` (Perl 5.28+) is used so `^`, `|`, `&` are always
numeric; in classic Perl they are *string* operators when both operands are
strings.

## What this does and does not guarantee

The source has no content-dependent early return, branch, subscript or loop
bound; work depends only on the public lengths.

It does **not** guarantee wall-clock constant time, and Perl cannot:

* Perl is interpreted: each op is dispatched through the runloop, and the
  interpreter's own branching, caching and allocation are not under this
  module's control.
* Scalars are reference counted, reallocated as they grow, and may share string
  buffers copy-on-write; arrays of bytes are built per call. Timing noise from
  allocation and the lack of any way to zero secrets in freed memory are
  outside the module's reach.
* There is no way to inspect or pin the machine code that executes the loop.
* The validation regexes inspect only the *type and shape* of inputs
  (digit strings, ranges), never the comparison result. A secret `choice` is
  validated by the single expression `($bit & ~1) != 0`, which does the same
  work for 0 and 1; the preceding shape check uses one character class and also
  takes the same path for both.

Treat it as protection against *algorithmic* timing leaks, not against an
attacker with cache or microarchitectural access.

## Tests

```bash
prove -l -v t/
```

`t/01-corpus.t` replays all 21 cases of the shared CT01 corpus
(`code/specs/fixtures/ct-compare-v1/cases.json`, read through core `JSON::PP`).
`t/02-exhaustive.t` adds every single-byte mismatch position, every single-bit
difference of every byte value, all 256 byte values through selection (all
256x256 pairs under both choices), length mismatches, all 64 single-bit flips of
a u64 from five bases, and input validation. Measured with Devel::Cover:
statement, branch and condition coverage of `CtCompare.pm` are 100%.

## Where it fits

A leaf utility with no dependencies and no capabilities (see
`required_capabilities.json`). MAC verifiers, token checkers and password-hash
verifiers should compare secrets through it.
