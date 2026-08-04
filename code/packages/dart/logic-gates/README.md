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
