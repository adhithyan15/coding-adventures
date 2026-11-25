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
which is the tested platform; on the web `1.0 is int` is true.

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
