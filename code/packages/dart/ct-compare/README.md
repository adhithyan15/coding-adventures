# coding_adventures_ct_compare (Dart)

Constant-time byte comparison and selection: the Dart lane of
[CT01](../../../specs/CT01-constant-time-compare.md).

A secret comparison that stops at the first differing byte tells an attacker how
long the matching prefix was. This package reads every byte regardless, folds
the differences into one accumulator and inspects it once at the end.

## API

```dart
import 'package:coding_adventures_ct_compare/coding_adventures_ct_compare.dart';

bool ctEq(Uint8List left, Uint8List right);
bool ctEqFixed(Uint8List left, Uint8List right);        // same as ctEq in Dart
Uint8List ctSelectBytes(Uint8List left, Uint8List right, int choice);
bool ctEqU64(BigInt left, BigInt right);

int ctEqBit(Uint8List left, Uint8List right);           // 1 or 0
int ctEqFixedBit(Uint8List left, Uint8List right);      // 1 or 0
int ctEqU64Bit(BigInt left, BigInt right);              // 1 or 0
```

* Inputs are `Uint8List`s (views included), so every element is already a byte.
* A length mismatch makes `ctEq` false (`ctEqBit` returns 0) without reading
  either input: lengths are public.
* `ctSelectBytes` throws `CtCompareError` with id `CT_SELECT_LENGTH_MISMATCH`
  for unequal lengths (checked first) and `CT_INVALID_CHOICE` for a choice other
  than 0 or 1; both are decided before any content is read. It returns a fresh
  `Uint8List`.
* The selection bit is an **int** that must be exactly 0 or 1. Dart has no
  branch-free bool-to-number conversion (`b ? 1 : 0` is a branch), so this lane
  follows Go, Ruby and Elixir. Produce the bit with the `...Bit` variants:

  ```dart
  final picked = ctSelectBytes(a, b, ctEqBit(x, y));
  ```
* Unsigned 64-bit values are `BigInt`s in `0..2^64-1` (`CT_INVALID_U64`
  otherwise), because a Dart `int` is a 64-bit value on the VM but a 53-bit
  double when compiled to JavaScript. Each value is split into 32-bit halves
  and the reduction uses only 32-bit arithmetic, so the VM and JavaScript agree.

## What this does and does not guarantee

The source has no content-dependent early return, branch, index or loop bound;
work depends only on the public lengths. Selection uses
`r ^ ((l ^ r) & mask)`; `ctEqBit` is `((acc - 1) >>> 8) & 1`.

It does **not** guarantee wall-clock constant time:

* The Dart VM's JIT can recompile the loops, the AOT compiler can rewrite them,
  and compiled to JavaScript the browser or Node JIT sits underneath too.
  Nothing lets the source pin or inspect machine code.
* The garbage collector moves objects and can run at any allocation, so secret
  bytes may be copied and left behind in freed memory. The package cannot zero
  them.
* `BigInt` arithmetic (used only to accept and split a u64 argument) is not
  constant time; only the 32-bit reduction after the split is fixed-work. A
  caller whose u64 is secret should compare as two 32-bit halves through
  `ctEqBit` on bytes instead.
* Typed-data bounds checks branch on the public index, never on data. Cache and
  speculation behaviour of the host are outside the model.

Treat it as protection against *algorithmic* timing leaks only.

## Tests

```bash
dart pub get
dart test
```

The suite replays all 21 cases of the shared CT01 corpus
(`code/specs/fixtures/ct-compare-v1/cases.json`, read relative to the package
directory with `dart:io`, so the tests run on the VM) and adds every
single-byte mismatch position, every single-bit difference of every byte value,
every accumulator value 1..255, all 256x256 byte pairs through selection under
both bits, length mismatches, invalid choices (including 2^32 and 2^53-1),
fresh-output ownership, and all 64 single-bit flips of a u64 from five bases.
`dart test --coverage` with `package:coverage` reports 38 of 38 library lines
hit. The arithmetic was also compiled with `dart compile js` and its main
properties re-checked under Node (that harness is not part of the package).

## Where it fits

A leaf utility with no dependencies and no capabilities (see
`required_capabilities.json`).
