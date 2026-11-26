/// Jest/Vitest-style test matchers with bit-exact equality, in pure Dart.
///
/// The Dart lane of TF01 (`test-matchers`): `expect` with the full Jest
/// matcher surface ([Expectation]), async assertions via
/// [Expectation.resolves] and [Expectation.rejects] ([AsyncExpectation]),
/// and the bit-exact comparison engine underneath ([toValue],
/// [valuesEqual], [deepEqual], [encodeValue]).
///
/// The port follows the TypeScript reference matcher-for-matcher; the
/// deliberate Dart adaptations (no `undefined` matchers, generic
/// [Expectation.toBeInstanceOf], bool-only truthiness, `Type` arguments for
/// [Expectation.toThrow]) are documented on each member and in the package
/// README's porting table.
library coding_adventures_test_matchers;

export 'src/deep_equal.dart';
export 'src/matchers.dart';
export 'src/value.dart';
