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
| `select_bytes` | Two byte strings of equal length and a boolean choice | Fresh copy of left for true, right for false | Read both inputs and write every one of `n` positions |
| `eq_u64` | Two unsigned 64-bit values | Equality | Reduce a fixed 64-bit difference, independent of values |

An `eq` length mismatch returns false without reading either content; length
is public. `eq_fixed` width mismatch is a caller/fixture type error, not a
portable runtime case: statically typed lanes may reject it before execution.
`select_bytes` length mismatch returns the semantic error
`CT_SELECT_LENGTH_MISMATCH` before reading either input; native exception text
is not standardized. The selection result must not alias a mutable input.

For equal lengths, equality can XOR corresponding bytes into an accumulator
and test only after the full loop. Selection can expand the boolean to an
all-zero or all-one byte mask and compute `right ^ ((left ^ right) & mask)`.
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
