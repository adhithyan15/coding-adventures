/// Unit tests for `coding_adventures_arithmetic`.
///
/// Mirrors the Rust reference's inline suite plus `adders_test.rs`,
/// `alu_test.rs`, and the `bit_domain.rs` rejection contract, then covers
/// result value semantics and exact shape messages.
library;

import 'package:coding_adventures_arithmetic/coding_adventures_arithmetic.dart';
import 'package:test/test.dart';

/// Unsigned integer to LSB-first bits of [width].
List<int> toBits(int val, int width) =>
    [for (var i = 0; i < width; i++) (val >> i) & 1];

/// LSB-first bits to unsigned integer.
int fromBits(List<int> bits) {
  var acc = 0;
  for (var i = 0; i < bits.length; i++) {
    acc |= bits[i] << i;
  }
  return acc;
}

void main() {
  group('mirror: inline adders', () {
    test('half adder truth table', () {
      expect(halfAdder(0, 0), (0, 0));
      expect(halfAdder(0, 1), (1, 0));
      expect(halfAdder(1, 0), (1, 0));
      expect(halfAdder(1, 1), (0, 1));
    });

    test('full adder truth table', () {
      expect(fullAdder(0, 0, 0), (0, 0));
      expect(fullAdder(0, 0, 1), (1, 0));
      expect(fullAdder(0, 1, 0), (1, 0));
      expect(fullAdder(0, 1, 1), (0, 1));
      expect(fullAdder(1, 0, 0), (1, 0));
      expect(fullAdder(1, 0, 1), (0, 1));
      expect(fullAdder(1, 1, 0), (0, 1));
      expect(fullAdder(1, 1, 1), (1, 1));
    });

    test('ripple carry 5 plus 3', () {
      // 5 + 3 = 8 overflows signed 4-bit (positive + positive = negative).
      final r = rippleCarryAdder([1, 0, 1, 0], [1, 1, 0, 0]);
      expect(r.sum, [0, 0, 0, 1]);
      expect(r.carryOut, 0);
      expect(r.overflow, isTrue);
    });

    test('ripple carry overflow', () {
      final r =
          rippleCarryAdder([1, 1, 1, 1, 1, 1, 1, 1], [1, 0, 0, 0, 0, 0, 0, 0]);
      expect(r.sum, [0, 0, 0, 0, 0, 0, 0, 0]);
      expect(r.carryOut, 1);
    });
  });

  group('mirror: inline alu', () {
    test('alu add simple', () {
      final r = alu(toBits(3, 8), toBits(5, 8), AluOp.add);
      expect(fromBits(r.result), 8);
      expect(r.zero, isFalse);
      expect(r.carry, isFalse);
    });

    test('alu add zero flag', () {
      final r = alu(toBits(0, 8), toBits(0, 8), AluOp.add);
      expect(r.zero, isTrue);
    });

    test('alu sub simple', () {
      final r = alu(toBits(10, 8), toBits(3, 8), AluOp.sub);
      expect(fromBits(r.result), 7);
    });

    test('alu sub to zero', () {
      final r = alu(toBits(5, 8), toBits(5, 8), AluOp.sub);
      expect(r.zero, isTrue);
      expect(fromBits(r.result), 0);
    });

    test('alu and', () {
      final r = alu(toBits(12, 8), toBits(10, 8), AluOp.and);
      expect(fromBits(r.result), 8);
    });

    test('alu or', () {
      final r = alu(toBits(12, 8), toBits(10, 8), AluOp.or);
      expect(fromBits(r.result), 14);
    });

    test('alu xor', () {
      final r = alu(toBits(12, 8), toBits(10, 8), AluOp.xor);
      expect(fromBits(r.result), 6);
    });

    test('alu not', () {
      final r = alu(toBits(15, 8), toBits(0, 8), AluOp.not);
      expect(fromBits(r.result), 240);
    });

    test('alu negative flag', () {
      final r = alu(toBits(1, 8), toBits(2, 8), AluOp.sub);
      expect(r.negative, isTrue, reason: '1 - 2 should be negative');
    });
  });

  group('mirror: adders_test', () {
    test('half adder exhaustive', () {
      expect(halfAdder(0, 0), (0, 0));
      expect(halfAdder(0, 1), (1, 0));
      expect(halfAdder(1, 0), (1, 0));
      expect(halfAdder(1, 1), (0, 1));
    });

    test('full adder exhaustive', () {
      const expected = [
        (0, 0, 0, 0, 0),
        (0, 0, 1, 1, 0),
        (0, 1, 0, 1, 0),
        (0, 1, 1, 0, 1),
        (1, 0, 0, 1, 0),
        (1, 0, 1, 0, 1),
        (1, 1, 0, 0, 1),
        (1, 1, 1, 1, 1),
      ];
      for (final (a, b, cin, expSum, expCarry) in expected) {
        expect(fullAdder(a, b, cin), (expSum, expCarry),
            reason: 'fullAdder($a, $b, $cin)');
      }
    });

    test('4-bit simple cases', () {
      expect(fromBits(rippleCarryAdder(toBits(0, 4), toBits(0, 4)).sum), 0);
      expect(fromBits(rippleCarryAdder(toBits(1, 4), toBits(1, 4)).sum), 2);
      final r = rippleCarryAdder(toBits(7, 4), toBits(1, 4));
      expect(fromBits(r.sum), 8);
      expect(r.carryOut, 0);
    });

    test('4-bit overflow', () {
      var r = rippleCarryAdder(toBits(15, 4), toBits(1, 4));
      expect(fromBits(r.sum), 0);
      expect(r.carryOut, 1);
      r = rippleCarryAdder(toBits(15, 4), toBits(15, 4));
      expect(fromBits(r.sum), 14);
      expect(r.carryOut, 1);
    });

    test('8-bit various', () {
      const cases = [
        (0, 0, 0, 0),
        (1, 1, 2, 0),
        (100, 55, 155, 0),
        (200, 100, 44, 1),
        (255, 1, 0, 1),
        (128, 128, 0, 1),
      ];
      for (final (a, b, expSum, expCarry) in cases) {
        final r = rippleCarryAdder(toBits(a, 8), toBits(b, 8));
        expect(fromBits(r.sum), expSum, reason: '$a + $b');
        expect(r.carryOut, expCarry, reason: '$a + $b carry');
      }
    });

    test('8-bit commutative', () {
      for (var a = 0; a < 256; a += 17) {
        for (var b = 0; b < 256; b += 23) {
          final r1 = rippleCarryAdder(toBits(a, 8), toBits(b, 8));
          final r2 = rippleCarryAdder(toBits(b, 8), toBits(a, 8));
          expect(r1.sum, r2.sum, reason: '$a + $b');
          expect(r1.carryOut, r2.carryOut);
        }
      }
    });

    test('identity', () {
      for (var a = 0; a <= 255; a++) {
        final r = rippleCarryAdder(toBits(a, 8), toBits(0, 8));
        expect(fromBits(r.sum), a, reason: '$a + 0');
        expect(r.carryOut, 0);
      }
    });

    test('16-bit', () {
      final r = rippleCarryAdder(toBits(1000, 16), toBits(2000, 16));
      expect(fromBits(r.sum), 3000);
      expect(r.carryOut, 0);
    });

    test('signed overflow', () {
      final r = rippleCarryAdder(toBits(127, 8), toBits(1, 8));
      expect(r.overflow, isTrue, reason: '127 + 1 overflows signed 8-bit');
      expect(fromBits(r.sum), 128);
    });

    test('no signed overflow', () {
      final r = rippleCarryAdder(toBits(64, 8), toBits(63, 8));
      expect(r.overflow, isFalse);
    });

    test('mismatched lengths', () {
      expect(() => rippleCarryAdder([0, 1], [0, 1, 0]), throwsArgumentError);
    });

    test('empty', () {
      expect(() => rippleCarryAdder([], []), throwsArgumentError);
    });

    test('1-bit', () {
      final r = rippleCarryAdder([1], [1]);
      expect(r.sum, [0]);
      expect(r.carryOut, 1);
    });
  });

  group('mirror: alu_test', () {
    test('add basic', () {
      const cases = [
        (0, 0, 0),
        (1, 1, 2),
        (3, 5, 8),
        (100, 55, 155),
        (10, 20, 30),
      ];
      for (final (a, b, expected) in cases) {
        final r = alu(toBits(a, 8), toBits(b, 8), AluOp.add);
        expect(fromBits(r.result), expected, reason: '$a + $b');
      }
    });

    test('add zero flag', () {
      final r = alu(toBits(0, 8), toBits(0, 8), AluOp.add);
      expect(r.zero, isTrue);
      expect(r.negative, isFalse);
      expect(r.carry, isFalse);
    });

    test('add carry flag', () {
      final r = alu(toBits(200, 8), toBits(100, 8), AluOp.add);
      expect(r.carry, isTrue, reason: '200 + 100 should carry');
      expect(fromBits(r.result), 44);
    });

    test('add signed overflow', () {
      final r = alu(toBits(127, 8), toBits(1, 8), AluOp.add);
      expect(r.overflow, isTrue);
      expect(r.negative, isTrue);
    });

    test('sub basic', () {
      const cases = [
        (5, 3, 2),
        (10, 1, 9),
        (100, 50, 50),
        (255, 254, 1),
      ];
      for (final (a, b, expected) in cases) {
        final r = alu(toBits(a, 8), toBits(b, 8), AluOp.sub);
        expect(fromBits(r.result), expected, reason: '$a - $b');
      }
    });

    test('sub to zero', () {
      final r = alu(toBits(42, 8), toBits(42, 8), AluOp.sub);
      expect(r.zero, isTrue);
      expect(fromBits(r.result), 0);
    });

    test('sub negative result', () {
      final r = alu(toBits(1, 8), toBits(2, 8), AluOp.sub);
      expect(r.negative, isTrue);
      expect(fromBits(r.result), 255);
    });

    test('sub signed overflow', () {
      final r = alu(toBits(128, 8), toBits(1, 8), AluOp.sub);
      expect(r.overflow, isTrue, reason: '-128 - 1 overflows');
    });

    test('and', () {
      final r = alu(toBits(204, 8), toBits(170, 8), AluOp.and);
      expect(fromBits(r.result), 136);
      expect(r.carry, isFalse);
      expect(r.overflow, isFalse);
    });

    test('and zero', () {
      final r = alu(toBits(255, 8), toBits(0, 8), AluOp.and);
      expect(r.zero, isTrue);
    });

    test('and identity', () {
      final r = alu(toBits(42, 8), toBits(255, 8), AluOp.and);
      expect(fromBits(r.result), 42);
    });

    test('or', () {
      final r = alu(toBits(204, 8), toBits(170, 8), AluOp.or);
      expect(fromBits(r.result), 238);
    });

    test('or identity', () {
      final r = alu(toBits(42, 8), toBits(0, 8), AluOp.or);
      expect(fromBits(r.result), 42);
    });

    test('xor', () {
      final r = alu(toBits(204, 8), toBits(170, 8), AluOp.xor);
      expect(fromBits(r.result), 102);
    });

    test('xor self is zero', () {
      final r = alu(toBits(171, 8), toBits(171, 8), AluOp.xor);
      expect(r.zero, isTrue);
    });

    test('not', () {
      final r = alu(toBits(15, 8), toBits(0, 8), AluOp.not);
      expect(fromBits(r.result), 240);
    });

    test('not all ones', () {
      final r = alu(toBits(255, 8), toBits(0, 8), AluOp.not);
      expect(r.zero, isTrue);
    });

    test('not zero', () {
      final r = alu(toBits(0, 8), toBits(0, 8), AluOp.not);
      expect(fromBits(r.result), 255);
      expect(r.negative, isTrue);
    });

    test('add all flags clear', () {
      final r = alu(toBits(1, 8), toBits(1, 8), AluOp.add);
      expect(r.zero, isFalse);
      expect(r.carry, isFalse);
      expect(r.negative, isFalse);
      expect(r.overflow, isFalse);
      expect(fromBits(r.result), 2);
    });

    test('4-bit add', () {
      final r = alu(toBits(7, 4), toBits(5, 4), AluOp.add);
      expect(fromBits(r.result), 12);
      expect(r.carry, isFalse);
    });

    test('4-bit sub', () {
      final r = alu(toBits(9, 4), toBits(3, 4), AluOp.sub);
      expect(fromBits(r.result), 6);
    });

    test('empty input', () {
      expect(() => alu([], [], AluOp.add), throwsArgumentError);
    });

    test('mismatched lengths', () {
      expect(() => alu([0, 1], [0, 1, 0], AluOp.add), throwsArgumentError);
    });

    test('1-bit add', () {
      final r = alu([1], [1], AluOp.add);
      expect(r.result, [0]);
      expect(r.carry, isTrue);
    });
  });

  group('mirror: bit_domain', () {
    const nonBits = [2, 3, 128, 255];
    const allOps = AluOp.values;
    const binaryOps = [AluOp.add, AluOp.sub, AluOp.and, AluOp.or, AluOp.xor];

    List<int> withJunk(int fill, int junk, int position) {
      final v = List<int>.filled(8, fill);
      v[position] = junk;
      return v;
    }

    test('half adder rejects non-bits', () {
      for (final junk in nonBits) {
        for (var bit = 0; bit <= 1; bit++) {
          expect(() => halfAdder(junk, bit), throwsArgumentError,
              reason: 'halfAdder($junk, $bit)');
          expect(() => halfAdder(bit, junk), throwsArgumentError,
              reason: 'halfAdder($bit, $junk)');
        }
      }
    });

    test('full adder rejects non-bits in any input', () {
      for (final junk in nonBits) {
        for (var x = 0; x <= 1; x++) {
          for (var y = 0; y <= 1; y++) {
            expect(() => fullAdder(junk, x, y), throwsArgumentError);
            expect(() => fullAdder(x, junk, y), throwsArgumentError);
            expect(() => fullAdder(x, y, junk), throwsArgumentError);
          }
        }
      }
    });

    test('ripple rejects non-bits at every position', () {
      final zeros = List<int>.filled(8, 0);
      for (final junk in nonBits) {
        for (var position = 0; position < 8; position++) {
          final bad = withJunk(0, junk, position);
          expect(() => rippleCarryAdder(bad, zeros), throwsArgumentError,
              reason: 'a[$position]');
          expect(() => rippleCarryAdder(zeros, bad), throwsArgumentError,
              reason: 'b[$position]');
          expect(() => rippleCarryAdderWithCarry(bad, zeros, 0),
              throwsArgumentError);
          expect(() => rippleCarryAdderWithCarry(zeros, bad, 1),
              throwsArgumentError);
        }
      }
    });

    test('with-carry rejects non-bit carry-in', () {
      for (final junk in nonBits) {
        expect(() => rippleCarryAdderWithCarry([0, 1], [1, 0], junk),
            throwsArgumentError);
        expect(() => rippleCarryAdderWithCarry([0], [0], junk),
            throwsArgumentError);
      }
    });

    test('ripple rejects bad shapes', () {
      expect(() => rippleCarryAdder([1, 0], [1]), throwsArgumentError);
      expect(() => rippleCarryAdder([1], [1, 0]), throwsArgumentError);
      expect(() => rippleCarryAdder([], []), throwsArgumentError);
      expect(() => rippleCarryAdder([], [1]), throwsArgumentError);
      expect(
          () => rippleCarryAdderWithCarry([1, 0], [1], 0), throwsArgumentError);
      expect(() => rippleCarryAdderWithCarry([], [], 0), throwsArgumentError);
    });

    test('alu rejects non-bits in a for every op', () {
      final zeros = List<int>.filled(8, 0);
      for (final junk in nonBits) {
        for (var position = 0; position < 8; position++) {
          final bad = withJunk(0, junk, position);
          for (final op in allOps) {
            expect(() => alu(bad, zeros, op), throwsArgumentError,
                reason: 'a[$position]=$junk $op');
          }
        }
      }
    });

    test('alu rejects non-bits in b for binary ops', () {
      final zeros = List<int>.filled(8, 0);
      for (final junk in nonBits) {
        for (var position = 0; position < 8; position++) {
          final bad = withJunk(0, junk, position);
          for (final op in binaryOps) {
            expect(() => alu(zeros, bad, op), throwsArgumentError,
                reason: 'b[$position]=$junk $op');
          }
        }
      }
    });

    test('alu rejects unequal lengths for binary ops', () {
      for (final op in binaryOps) {
        expect(() => alu([1, 0, 1], [1, 0], op), throwsArgumentError,
            reason: '$op b shorter');
        expect(() => alu([1, 0], [1, 0, 1], op), throwsArgumentError,
            reason: '$op b longer');
        expect(() => alu([1], [], op), throwsArgumentError,
            reason: '$op b empty');
      }
    });

    test('alu rejects empty a for every op', () {
      for (final op in allOps) {
        expect(() => alu([], [], op), throwsArgumentError,
            reason: '$op both empty');
        expect(() => alu([], [1, 0], op), throwsArgumentError,
            reason: '$op a empty');
      }
    });

    test('not ignores b whatever it holds', () {
      final a = [1, 0, 1, 1];
      final expected = alu(a, [0, 0, 0, 0], AluOp.not);
      for (final b in [
        <int>[],
        [255],
        [2, 3, 128, 255],
        List<int>.filled(64, 1),
      ]) {
        expect(alu(a, b, AluOp.not), equals(expected), reason: '$b');
      }
    });

    test('long operand costs linear time and own size', () {
      final a = List<int>.filled(100000, 1);
      final b = List<int>.filled(100000, 0);
      final r = rippleCarryAdderWithCarry(a, b, 1);
      expect(r.sum.length, 100000);
      expect(r.carryOut, 1);
      expect(r.sum.every((bit) => bit == 0), isTrue);
    });
  });

  group('EXTRA: values and messages', () {
    test('shape messages are exact', () {
      expect(
          () => rippleCarryAdder([1, 0], [1]),
          throwsA(isA<ArgumentError>().having((e) => e.message, 'message',
              'a and b must have the same length, got 2 and 1')));
      expect(
          () => rippleCarryAdder([], []),
          throwsA(isA<ArgumentError>().having(
              (e) => e.message, 'message', 'bit lists must not be empty')));
      expect(
          () => alu([], [], AluOp.add),
          throwsA(isA<ArgumentError>()
              .having((e) => e.message, 'message', 'a must not be empty')));
    });

    test('results compare by value', () {
      final r1 = rippleCarryAdder([1, 0], [0, 1]);
      final r2 = rippleCarryAdder([1, 0], [0, 1]);
      final r3 = rippleCarryAdder([1, 0], [1, 1]);
      expect(r1, equals(r2));
      expect(r1.hashCode, r2.hashCode);
      expect(r1 == r3, isFalse);
      expect(r1 == 'x', isFalse);
      expect(r1.toString(), contains('carryOut'));

      final a1 = alu([1, 0], [0, 1], AluOp.add);
      final a2 = alu([1, 0], [0, 1], AluOp.add);
      final a3 = alu([1, 0], [0, 1], AluOp.sub);
      expect(a1, equals(a2));
      expect(a1.hashCode, a2.hashCode);
      expect(a1 == a3, isFalse);
      expect(a1 == 'x', isFalse);
      expect(a1.toString(), contains('zero'));
    });

    test('result equality rejects shape and content mismatches', () {
      final two = rippleCarryAdder([1, 0], [0, 1]);
      final one = rippleCarryAdder([1], [1]);
      expect(two == one, isFalse);
      final add = alu([1, 0], [0, 1], AluOp.add);
      final narrow = alu([1], [1], AluOp.add);
      expect(add == narrow, isFalse);
      final other = alu([0, 0], [0, 1], AluOp.add);
      expect(add == other, isFalse);
    });

    test('with-carry direct vectors', () {
      // 3 + 5 + 1 = 9.
      var r = rippleCarryAdderWithCarry(toBits(3, 8), toBits(5, 8), 1);
      expect(fromBits(r.sum), 9);
      expect(r.carryOut, 0);
      // 255 + 0 + 1 wraps with carry.
      r = rippleCarryAdderWithCarry(toBits(255, 8), toBits(0, 8), 1);
      expect(fromBits(r.sum), 0);
      expect(r.carryOut, 1);
    });
  });
}
