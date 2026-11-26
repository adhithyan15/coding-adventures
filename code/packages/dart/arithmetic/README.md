# coding_adventures_arithmetic (Dart)

Adder circuits and ALU built from logic gates. The Dart lane of the Rust
`arithmetic` crate: half/full adders, N-bit ripple-carry addition, and the
six-operation ALU with status flags — all wired from the in-lane logic
gates, exactly as hardware wires them.

```dart
import 'package:coding_adventures_arithmetic/coding_adventures_arithmetic.dart';

// 5 + 3 = 8 in 4-bit (LSB first).
final sum = rippleCarryAdder([1, 0, 1, 0], [1, 1, 0, 0]);
print(sum.sum); // [0, 0, 0, 1]
print(sum.overflow); // true: signed 4-bit cannot hold 8

final r = alu([1, 1, 0, 0, 0, 0, 0, 0], [1, 0, 1, 0, 0, 0, 0, 0], AluOp.add);
print(r.result); // 3 + 5 = 8
```

## API

- `halfAdder(a, b)` / `fullAdder(a, b, carryIn)` → `(sum, carry)`
  records.
- `rippleCarryAdder(a, b)` / `rippleCarryAdderWithCarry(a, b, carryIn)` —
  N-bit LSB-first addition → [RippleCarryResult] (`sum`, `carryOut`,
  signed `overflow`).
- `alu(a, b, op)` — [AluOp] (`add`, `sub`, `and`, `or`, `xor`, `not`) →
  [AluResult] (`result`, `zero`, `carry`, `negative`, `overflow`).
  Subtraction runs `A + NOT(B) + 1`; `not` inverts `a` and never reads
  `b` (any content, any length — pinned by contract tests).

## Contract

- Bits are `int`s holding 0 or 1. Non-bit rejection is inherited from the
  gates (always-on [ArgumentError]), as in the reference.
- Shape rejection is this package's own [ArgumentError] with identical
  messages: equal non-empty operand lengths (`not` only requires a
  non-empty `a`).
- No width-driven amplification: every output is exactly as long as its
  input, pinned by a 100,000-bit test.
- The reference's `assert!` panics become [ArgumentError]s (same texts);
  the sandbox/strace `runtime_capability` harness and the generated
  Warrant-evidence vectors are Rust-harness artifacts and do not port —
  the Dart lane proves the same properties with seeded properties and
  fuzzing instead.

## Validation

- **Unit** (`test/arithmetic_test.dart`): mirrors the inline suite,
  `adders_test.rs`, `alu_test.rs`, and the `bit_domain.rs` rejection
  contract (non-bits at every position of every input, shapes, `not`'s
  blindness, the 100k-bit case), plus value semantics and exact messages.
  65 tests.
- **Property** (`test/properties_test.dart`): adder/integer models,
  ripple and ALU against a [BigInt] oracle (values, carries, signed
  ranges), commutativity, identities, `not` blindness — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration campaign over hostile
  operands (junk bits, empty/mismatched shapes) asserting oracle
  agreement, exact rejection, and output-length discipline. Ad-hoc
  60,000-iteration runs over three more seeds: zero crashes.
- **Coverage**: 100% lines and branches, 94.1% functions (one implicit
  enum constructor, uncoverable by construction).
