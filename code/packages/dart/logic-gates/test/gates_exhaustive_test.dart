import 'package:coding_adventures_logic_gates/coding_adventures_logic_gates.dart';
import 'package:test/test.dart';

// Dart ints are 64-bit on the VM; write the extremes without a literal that
// would overflow on the web.
//
// On dart2js ints are JS numbers (`identical(1, 1.0)`): shifts are 32-bit,
// `1.0 is int`, and integral doubles pass as bits. Those cases are VM-only.
final bool isWeb = identical(1, 1.0);
final int int64Min = isWeb ? -4503599627370496 : -(1 << 62) * 2;
final int int64Max = isWeb ? 4503599627370496 : (1 << 62) - 1 + (1 << 62);

final List<int> badValues = [
  for (var v = -1024; v <= 1024; v++)
    if (v != 0 && v != 1) v,
  int64Min,
  int64Max,
  2147483648,
  -2147483648,
];

void main() {
  const pairs = [(0, 0), (0, 1), (1, 0), (1, 1)];

  final binary = <String, (int Function(int, int), List<int>)>{
    'AND': (AND, [0, 0, 0, 1]),
    'OR': (OR, [0, 1, 1, 1]),
    'XOR': (XOR, [0, 1, 1, 0]),
    'NAND': (NAND, [1, 1, 1, 0]),
    'NOR': (NOR, [1, 0, 0, 0]),
    'XNOR': (XNOR, [1, 0, 0, 1]),
    'nandAnd': (nandAnd, [0, 0, 0, 1]),
    'nandOr': (nandOr, [0, 1, 1, 1]),
    'nandXor': (nandXor, [0, 1, 1, 0]),
    'nandNor': (nandNor, [1, 0, 0, 0]),
    'nandXnor': (nandXnor, [1, 0, 0, 1]),
  };
  final unary = <String, (int Function(int), List<int>)>{
    'NOT': (NOT, [1, 0]),
    'nandNot': (nandNot, [1, 0]),
  };
  final direct = <String, int Function(int, int)>{
    'nandAnd': AND,
    'nandOr': OR,
    'nandXor': XOR,
    'nandNor': NOR,
    'nandXnor': XNOR,
  };

  group('literal truth tables', () {
    binary.forEach((name, spec) {
      test(name, () {
        final got = [for (final p in pairs) spec.$1(p.$1, p.$2)];
        expect(got, spec.$2);
      });
    });
    unary.forEach((name, spec) {
      test(name, () {
        expect([spec.$1(0), spec.$1(1)], spec.$2);
      });
    });
  });

  test('NAND-built gates equal direct gates', () {
    expect(nandNot(0), NOT(0));
    expect(nandNot(1), NOT(1));
    direct.forEach((name, gate) {
      for (final p in pairs) {
        expect(binary[name]!.$1(p.$1, p.$2), gate(p.$1, p.$2), reason: name);
      }
    });
  });

  group('invalid bits', () {
    InvalidBitError capture(void Function() body) {
      try {
        body();
      } on InvalidBitError catch (e) {
        return e;
      }
      fail('expected InvalidBitError');
    }

    unary.forEach((name, spec) {
      test('$name rejects every invalid int', () {
        for (final v in badValues) {
          final e = capture(() => spec.$1(v));
          expect(e.name, 'a');
          expect(e.invalidValue, v);
          expect(e, isA<ArgumentError>());
        }
      });
    });

    binary.forEach((name, spec) {
      test('$name rejects every invalid int in either position', () {
        final gate = spec.$1;
        for (final v in badValues) {
          for (final other in [0, 1]) {
            var e = capture(() => gate(v, other));
            expect(e.name, 'a', reason: '$name($v,$other)');
            expect(e.invalidValue, v);
            e = capture(() => gate(other, v));
            expect(e.name, 'b', reason: '$name($other,$v)');
            expect(e.invalidValue, v);
          }
          // Both invalid: the FIRST invalid parameter is reported.
          final e = capture(() => gate(v, 2));
          expect(e.name, 'a');
          expect(e.invalidValue, v);
        }
      });
    });

    test('nandOr(1, 2) reports b, not an inner name', () {
      final e = capture(() => nandOr(1, 2));
      expect(e.name, 'b');
      expect(e.invalidValue, 2);
    });
  });

  test('no valid input throws and results are exactly 0 or 1', () {
    for (final p in pairs) {
      for (final spec in binary.values) {
        final r = spec.$1(p.$1, p.$2);
        expect(r == 0 || r == 1, isTrue);
      }
    }
    for (final spec in unary.values) {
      for (final b in [0, 1]) {
        final r = spec.$1(b);
        expect(r == 0 || r == 1, isTrue);
      }
    }
  });

  group('dynamic type confusion (Dart VM)',
      skip: isWeb ? 'VM-only: dart2js accepts integral doubles' : false, () {
    final confused = <Object?>[1.0, 0.0, true, false, '1', null];
    final gates = <String, Function>{
      for (final e in binary.entries) e.key: e.value.$1,
      for (final e in unary.entries) e.key: e.value.$1,
    };

    test('non-int arguments never produce a result', () {
      gates.forEach((name, gate) {
        final arity = unary.containsKey(name) ? 1 : 2;
        for (final bad in confused) {
          for (var position = 0; position < arity; position++) {
            final args = List<Object?>.filled(arity, 1);
            args[position] = bad;
            Object? result;
            var threw = false;
            try {
              result = Function.apply(gate, args);
            } on TypeError {
              threw = true;
            } on ArgumentError {
              threw = true;
            }
            expect(threw, isTrue, reason: '$name$args produced $result');
          }
        }
      });
    });
  });
}
