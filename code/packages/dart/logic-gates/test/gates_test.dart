import 'package:coding_adventures_logic_gates/coding_adventures_logic_gates.dart';
import 'package:test/test.dart';

void main() {
  const pairs = [(0, 0), (0, 1), (1, 0), (1, 1)];

  group('primitive truth tables', () {
    test('NOT', () {
      expect([NOT(0), NOT(1)], [1, 0]);
    });

    test('two-input gates', () {
      expect([for (final pair in pairs) AND(pair.$1, pair.$2)], [0, 0, 0, 1]);
      expect([for (final pair in pairs) OR(pair.$1, pair.$2)], [0, 1, 1, 1]);
      expect([for (final pair in pairs) XOR(pair.$1, pair.$2)], [0, 1, 1, 0]);
      expect([for (final pair in pairs) NAND(pair.$1, pair.$2)], [1, 1, 1, 0]);
      expect([for (final pair in pairs) NOR(pair.$1, pair.$2)], [1, 0, 0, 0]);
      expect([for (final pair in pairs) XNOR(pair.$1, pair.$2)], [1, 0, 0, 1]);
    });

    test('invalid bits are rejected by every primitive', () {
      expect(() => NOT(2), throwsArgumentError);
      for (final gate in [AND, OR, XOR, NAND, NOR, XNOR]) {
        expect(() => gate(0, -1), throwsArgumentError);
      }
    });
  });

  group('derived and multi-input gates', () {
    test('NAND derivations equal their primitives', () {
      for (final bit in [0, 1]) {
        expect(nandNot(bit), NOT(bit));
      }
      for (final pair in pairs) {
        expect(nandAnd(pair.$1, pair.$2), AND(pair.$1, pair.$2));
        expect(nandOr(pair.$1, pair.$2), OR(pair.$1, pair.$2));
        expect(nandXor(pair.$1, pair.$2), XOR(pair.$1, pair.$2));
        expect(nandNor(pair.$1, pair.$2), NOR(pair.$1, pair.$2));
        expect(nandXnor(pair.$1, pair.$2), XNOR(pair.$1, pair.$2));
      }
    });

    test('multi-input gates and parity', () {
      expect(andN([1, 1, 1, 1]), 1);
      expect(andN([1, 1, 0, 1]), 0);
      expect(orN([0, 0, 0, 1]), 1);
      expect(xorN([]), 0);
      expect(xorN([1]), 1);
      expect(xorN([1, 1, 1]), 1);
      expect(xorN([1, 1, 0, 0]), 0);
    });

    test('AND and OR require at least two inputs', () {
      expect(() => andN([1]), throwsArgumentError);
      expect(() => orN([]), throwsArgumentError);
    });
  });

  test('two-way mux and dmux', () {
    expect(mux(0, 1, 0), 0);
    expect(mux(0, 1, 1), 1);
    expect(dmux(1, 0), (1, 0));
    expect(dmux(1, 1), (0, 1));
    expect(dmux(0, 1), (0, 0));
  });
}
