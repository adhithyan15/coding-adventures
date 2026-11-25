# coding_adventures_logic_gates

Pure-Dart implementation of the logic-gates package: the first layer of the
computing stack.

## What it provides

- Primitive gates: `NOT`, `AND`, `OR`, `XOR`, `NAND`, `NOR`, and `XNOR`
- NAND-only proofs: `nandNot`, `nandAnd`, `nandOr`, `nandXor`, `nandNor`, and
  `nandXnor`
- Multi-input gates and two-input `mux` / `dmux`
- General multiplexers, demultiplexers, decoders, encoders, priority encoders,
  and tri-state outputs
- SR and D latches, D flip-flops, registers, bidirectional shift registers,
  and binary counters

Signals are represented by the `Bit` typedef and validated at every public
boundary: only integer values 0 and 1 are accepted. Sequential functions are
pure. They return their output together with immutable `FlipFlopState` or
`CounterState` values that the caller supplies to the next clock phase.

Select vectors and register/counter values use least-significant-bit-first
ordering, matching the other language implementations.

## Input validation

Every gate validates every argument itself and throws `InvalidBitError` (a
subclass of `ArgumentError`) for the first argument that is not exactly 0 or 1.
`error.name` is the gate's own parameter name (`a`, `b`, ...), never an inner
helper's, and `error.invalidValue` is the rejected integer. The NAND-built gates
(`nandNot`, `nandAnd`, `nandOr`, `nandXor`, `nandNor`, `nandXnor`) validate up
front, so `nandOr(1, 2)` reports `b`, not an inner name.

Non-`int` values are rejected by Dart's type system: statically at compile time,
and for `dynamic` callers with a `TypeError` at the call boundary (double `1.0`,
`true`, `"1"`, and `null` never produce a result). This holds on the Dart VM,
which is the tested platform. A `List<dynamic>` (for example from `jsonDecode`)
passed to a `List<int>` parameter fails with a `TypeError` at the call boundary,
on the caller's side, before any gate code runs.

`InvalidBitError` is an `Error` (via `ArgumentError`), not an `Exception`: an
`on Exception` handler will not catch it.

**Web limitation.** dart2js represents ints as JS numbers, so `1.0` and `-0.0`
are indistinguishable from `1` and `0` and are accepted as bits (they equal 0 or
1 exactly); list snapshots canonicalise them to ints. Run `dart test -p node` to
exercise the web build; VM-only tests are skipped there.

## Hostile input and caps

Every function that takes a caller `List` reads `length` once, rejects it before
any allocation or shift if it exceeds the cap, copies each element exactly once,
and computes only from the copy, so a `List` subclass that lies, changes, or
claims an enormous length cannot cause a wrong answer or runaway work. Error
messages never embed caller objects. Caps: select bits and `decoder` inputs
<= 16, N-ary inputs / register and shift-register widths <= 65536, counter width
<= 64. Violations throw `ArgumentError`; invalid bits throw `InvalidBitError`.

## Example

```dart
import 'package:coding_adventures_logic_gates/coding_adventures_logic_gates.dart';

void main() {
  print(AND(1, 1)); // 1
  print(mux4(0, 1, 0, 1, [1, 0])); // 1

  final low = dFlipFlop(1, 0);
  final high = dFlipFlop(1, 1, state: low.$3);
  print(high.$1); // 1
}
```

## Running the tests

```sh
dart pub get
dart test
```
