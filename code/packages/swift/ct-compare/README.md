# CtCompare (Swift)

Constant-time byte comparison and selection: the Swift lane of
[CT01](../../../specs/CT01-constant-time-compare.md).

A secret comparison that stops at the first differing byte tells an attacker how
long the matching prefix was. This package reads every byte regardless, folds
the differences into one accumulator and inspects it once at the end.

## Usage

```swift
import CtCompare

let same = ctEq(expectedTag, receivedTag)

// The selection bit is a UInt8 produced without converting a Bool.
let picked = try ctSelectBytes(accept, reject, choice: ctEqBit(expectedTag, receivedTag))
```

## API

| Function | Result |
| --- | --- |
| `ctEq(_:_:) -> Bool`, `ctEqFixed(_:_:) -> Bool` | byte equality over `[UInt8]` (`ctEqFixed` is the same operation) |
| `ctSelectBytes(_:_:choice:) throws -> [UInt8]` | `left` when `choice == 1`, `right` when `0` |
| `ctEqU64(_:_:) -> Bool` | equality of two `UInt64`s |
| `ctEqBit(_:_:)`, `ctEqFixedBit(_:_:) -> UInt8` | `1` or `0` |
| `ctEqU64Bit(_:_:) -> UInt8` | `1` or `0` |

* `CtCompareError.selectLengthMismatch` is CT01's `CT_SELECT_LENGTH_MISMATCH`
  (checked first); `.invalidChoice` is any choice other than 0 or 1. Both are
  decided before any content is read.
* A length mismatch makes `ctEq` false (`ctEqBit` returns 0) without reading
  either input: lengths are public.
* The selection bit is an integer (`UInt8`, exactly 0 or 1): Swift's `Bool` has
  no numeric conversion, and `b ? 1 : 0` is a branch on the secret. This lane
  follows Go, Ruby and Elixir; use the `...Bit` variants to produce the bit.
* The result of `ctSelectBytes` is a new array. Arrays are value types, so
  later mutation of an input cannot change it.
* Swift traps on integer overflow, so the two's-complement steps use `&-`.

## What this does and does not guarantee

The source has no content-dependent early return, branch, index or loop bound;
work depends only on the public lengths. Selection uses
`r ^ ((l ^ r) & mask)`; `ctEqBit` is `((acc &- 1) >> 63)` on the widened
accumulator and `ctEqU64Bit` is `((d | (0 &- d)) >> 63) ^ 1`.

It does **not** guarantee wall-clock constant time:

* The Swift optimiser (LLVM) may unroll, vectorise or turn mask arithmetic back
  into a branch when it can prove a value is 0 or 1. Nothing in the language
  lets the source pin the generated code; the optimised assembly
  (`swiftc -O -emit-assembly`) is the only evidence, and this package does not
  claim it has been reviewed for every toolchain.
* Arrays are copy-on-write reference-counted buffers: secret bytes can be
  copied (for instance when a shared array is mutated) and left behind in freed
  memory. The package cannot zero them.
* Array subscripts are bounds-checked; the checks branch on the public index,
  never on data. Cache and speculation behaviour of the host are outside the
  model.

Treat it as protection against *algorithmic* timing leaks only.

## Tests

```bash
swift test --enable-code-coverage
```

`CorpusTests` replays all 21 cases of the shared CT01 corpus
(`code/specs/fixtures/ct-compare-v1/cases.json`, located from `#filePath` and
decoded with `JSONSerialization`). The other suites add every single-byte
mismatch position, every single-bit difference of every byte value, every
accumulator value 1..255, all 256x256 byte pairs through selection under both
bits, length mismatches, every invalid choice 2..255, and all 64 single-bit
flips of a `UInt64` from five bases.

**Not run where this port was written:** no Swift toolchain was available
there, so the sources and tests have not been compiled or executed. They
follow the conventions of the sibling `swift/argon2d` and `swift/rng` packages
and should be built with `swift test` on first CI run.

## Where it fits

A leaf utility with no dependencies and no capabilities (see
`required_capabilities.json`).
