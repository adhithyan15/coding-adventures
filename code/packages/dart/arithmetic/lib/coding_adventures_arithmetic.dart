/// Adder circuits and ALU built from logic gates.
///
/// The Dart lane of `arithmetic`: [halfAdder]/[fullAdder] single-bit
/// circuits, N-bit [rippleCarryAdder]/[rippleCarryAdderWithCarry], and the
/// six-operation [alu] with status flags — all wired from the in-lane
/// logic gates, rejecting non-bits and bad shapes with [ArgumentError].
library coding_adventures_arithmetic;

export 'src/adders.dart';
export 'src/alu.dart';
