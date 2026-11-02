# CT01 — Constant-time comparison and selection

## Scope and assurance boundary

`ct-compare` is a pure byte-level helper: it has no filesystem, network,
clock, random-number, process, or credential-store authority. The portable
surface is byte equality, a fixed-width equality alias, equal-length byte
selection, and unsigned 64-bit equality. Lengths, fixed widths, and the
operation being called are public. Compared contents and the selection bit are
secret-bearing. This specification requires control flow and memory indexing
that do not depend on those secret-bearing values for valid equal-length
inputs. It does **not** promise a universal wall-clock constant, cache or
speculation immunity, or that every compiler/runtime preserves the source
shape. Each lane must review its optimized code or runtime limits separately.

## Operations

| Operation | Valid inputs | Result | Public-work model |
| --- | --- | --- | --- |
| `eq` | Two byte strings | Equal lengths and bytes | Compare all `n` positions when lengths match; zero positions when lengths differ |
| `eq_fixed` | Two byte strings of declared public width `n` | Byte equality | Compare all `n` positions |
| `select_bytes` | Two byte strings of equal length and a secret selection bit (a boolean or an integer bit; see below) | Fresh copy of left when the bit is set, right when it is clear | Read both inputs and write every one of `n` positions |
| `eq_u64` | Two unsigned 64-bit values | Equality | Reduce a fixed 64-bit difference, independent of values |

An `eq` length mismatch returns false without reading either content; length
is public. `eq_fixed` width mismatch is a caller/fixture type error, not a
portable runtime case: statically typed lanes may reject it before execution.
`select_bytes` length mismatch returns the semantic error
`CT_SELECT_LENGTH_MISMATCH` before reading either input; native exception text
is not standardized. The selection result must not alias a mutable input.

### How a lane receives the selection bit

`choice` is secret, so turning it into a mask must not branch on it. Some
languages can turn a boolean into the integer 0 or 1 without a branch:
- as a plain value conversion: Rust `choice as u8`, C++
  `static_cast<unsigned>(choice)`, TypeScript `Number(choice)`, Python
  `int(choice)`;
- by reading the boolean's stored byte, as .NET does with `Unsafe.As`.

Those lanes may take a boolean `choice`, but **only if their own code uses such
a conversion**. A lane that writes `choice ? 1 : 0`, `if choice ...` or any
other conditional is not compliant, whatever its language could do.

C takes `choice` as an `int` used as a boolean, normalised with the value
expression `choice != 0`, which involves no control flow.

Optimisers and JIT compilers can reintroduce a branch on a value they know is 0
or 1. CT01 makes no promise about compiled code, so each lane reviews its
compiled output separately. Rust's `subtle` crate wraps the bit in an opaque
type for this reason.

Some languages have no branch-free way to turn a boolean into a number:
- **Go** has no bool-to-integer conversion at all;
- **Java and Kotlin** have none for `boolean`;
- in **Ruby and Elixir**, every conversion is an `if`, a `?:` or a pattern match.

Those lanes take `choice` as an **integer bit**: exactly 0 (select `right`) or
1 (select `left`). The parameter may be a signed integer type, because the
validation below rejects negative values too. This is how Go's own `crypto/subtle`
selection functions work. Any other value is a caller error, raised before
either input is read.

The validation itself must not leak the bit. It must take the same path for 0
and for 1. Testing `choice & ~1` for zero, one computation whose result is the
same for both valid values, is acceptable. Comparing `choice` against each
valid value in turn is **not**: a check like `[true, false].include?(choice)`
stops at its first match, and so does a different amount of work for each
value. The conformance corpus keeps a boolean `choice`, and an integer-bit
lane's adapter maps `true` to 1 and `false` to 0. That mapping is itself a
branch, which is fine only because corpus values are public test data:
adapters are test-only code and never ship.

**Callers must produce the bit without branching too.** The usual call is
`select_bytes(a, b, eq(x, y))`. In an integer-bit lane, `eq` returns a
boolean, so building the bit from it would need exactly the `if` this rule
removes. Integer-bit lanes therefore also offer **bit-returning equality
variants**, as Go's `subtle.ConstantTimeCompare` returns an `int`:

| Variant | Result |
|---|---|
| `eq_bit(a, b)` | 1 when `eq(a, b)`, else 0 (0 for a public length mismatch) |
| `eq_fixed_bit(a, b)` | 1 when `eq_fixed(a, b)`, else 0 |
| `eq_u64_bit(x, y)` | 1 when `eq_u64(x, y)`, else 0 |

The bit must be computed from the difference accumulator arithmetically,
never by a comparison followed by a conditional. For a byte accumulator
`acc` in 0..255 and a 64-bit difference `d`:

```
eq_bit     = (acc - 1) >> k   (logical shift, k = word width - 1;
                               acc = 0 wraps to all ones, so the bit is 1)
eq_u64_bit = ((d | -d) >> 63) xor 1
                              (d | -d has its top bit set exactly when d != 0)
```

Languages with unbounded integers (Ruby, Elixir) use the same identities
with an arithmetic shift of the negative value and a final `and 1`. Their
big-integer arithmetic is not promised to be constant time (see the scope
note above). `select_bytes(a, b, eq_bit(x, y))` then needs no conditional
anywhere. Callers must never convert a boolean equality result into a
selection bit with a conditional.

For equal lengths, equality can XOR corresponding bytes into an accumulator
and test only after the full loop. Selection can expand the bit to an
all-zero or all-one byte mask (`-bit & 0xFF`) and compute
`right ^ ((left ^ right) & mask)`.
An equivalent implementation is allowed, but no content-dependent early
return, conditional selection branch, lookup index, or loop bound is allowed.
The `eq_u64` reduction must inspect all 64 bits through a fixed operation
sequence or reviewed equivalent; using ordinary short-circuit equality is
not sufficient evidence of the intended assurance. Never use a noisy timing
ratio test as proof of this contract.

The reference `expected_work` object counts logical input positions, not CPU
instructions. For byte operations, `byte_positions` is `n` for equal-length
valid calls, zero for a public length rejection; `byte_reads` is twice that
number, and `byte_writes` is `n` only for successful selection. For `eq_u64`,
`u64_bits` is exactly 64 and byte counts are zero. These values are derivable
from public input shape; lane tests should also inspect their production
source or generated code for content-dependent work.

## Neutral conformance corpus

`code/specs/fixtures/ct-compare-v1/cases.json` is the normative functional
oracle. Its closed JSON Schema and independent Python validator reject extra
fields, duplicate IDs, noncanonical hex, wrong expected results, and work
counts inconsistent with public input shape. Byte strings are lowercase,
even-length hexadecimal; empty bytes are `""`. Unsigned 64-bit values are
exactly 16 lowercase hexadecimal digits (including leading zeros). The only
semantic error ID in v1 is `CT_SELECT_LENGTH_MISMATCH`; malformed fixture
transport is rejected by the validator and never passed to a language adapter.
An adapter may map a native exception to that ID but must not invent a
successful output for an error case.

The corpus covers empty, first/middle/last/all-byte mismatch, public length
mismatch, high-bit bytes, a 256-byte input, both choices, a fresh-output
ownership case, fixed widths, zero/maximum/sign-bit unsigned integers, and
single-bit changes. Fixture outputs and logical work counts are independently
derived from the inputs; the expected fields are not generated from any
existing language package. Existing-lane adoption and missing Dart, Haskell,
Lua, Perl, and Swift ports are separate CT01 owners, not silently marked done
by this corpus.
