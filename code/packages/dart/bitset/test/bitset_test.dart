import 'package:coding_adventures_bitset/coding_adventures_bitset.dart';
import 'package:test/test.dart';

void main() {
  group('construction', () {
    test('empty and sized bitsets have rounded capacity', () {
      expect(Bitset().length, 0);
      expect(Bitset().capacity, 0);
      expect(Bitset().all, isTrue);
      expect(Bitset(1).capacity, 64);
      expect(Bitset(64).capacity, 64);
      expect(Bitset(65).capacity, 128);
      expect(Bitset(200).capacity, 256);
    });

    test('negative size is rejected', () {
      expect(() => Bitset(-1), throwsRangeError);
    });

    test('fromInteger accepts int and arbitrarily large BigInt', () {
      final small = Bitset.fromInteger(42);
      expect(small.length, 6);
      expect(small.toInteger(), BigInt.from(42));
      final value = (BigInt.one << 200) | (BigInt.one << 100) | BigInt.one;
      final large = Bitset.fromInteger(value);
      expect(large.length, 201);
      expect(large.setBits, [0, 100, 200]);
      expect(large.toInteger(), value);
    });

    test('fromInteger rejects negative and unsupported values', () {
      expect(() => Bitset.fromInteger(-1), throwsA(isA<BitsetError>()));
      expect(() => Bitset.fromInteger('1'), throwsA(isA<BitsetError>()));
    });

    test('fromBinaryString preserves leading zeros', () {
      final bits = Bitset.fromBinaryString('0001');
      expect(bits.length, 4);
      expect(bits.toInteger(), BigInt.one);
      expect(bits.toBinaryString(), '0001');
    });

    test('fromBinaryString validates input', () {
      expect(Bitset.fromBinaryString('').length, 0);
      for (final invalid in ['102', 'abc', '10 01', '1.0']) {
        expect(
          () => Bitset.fromBinaryString(invalid),
          throwsA(isA<BitsetError>()),
        );
      }
    });
  });

  group('single-bit operations', () {
    test('set, test, clear, and toggle', () {
      final bits = Bitset(100);
      bits.set(50);
      expect(bits.test(50), isTrue);
      expect(bits.popcount, 1);
      bits.set(50);
      expect(bits.popcount, 1);
      bits.clear(50);
      expect(bits.test(50), isFalse);
      bits.toggle(5);
      expect(bits.test(5), isTrue);
      bits.toggle(5);
      expect(bits.test(5), isFalse);
    });

    test('set and toggle auto-grow with doubling capacity', () {
      final bits = Bitset();
      bits.set(0);
      expect(bits.capacity, 64);
      bits.set(64);
      expect(bits.capacity, 128);
      bits.toggle(200);
      expect(bits.length, 201);
      expect(bits.capacity, 256);
      expect(bits.test(200), isTrue);
    });

    test('clear and test beyond length do not grow', () {
      final bits = Bitset(10);
      bits.clear(999);
      expect(bits.test(999), isFalse);
      expect(bits.length, 10);
    });

    test('negative indices are rejected consistently', () {
      final bits = Bitset();
      expect(() => bits.set(-1), throwsRangeError);
      expect(() => bits.clear(-1), throwsRangeError);
      expect(() => bits.test(-1), throwsRangeError);
      expect(() => bits.toggle(-1), throwsRangeError);
    });

    test('word boundaries remain independent', () {
      final bits = Bitset(200);
      for (final index in [0, 63, 64, 127, 128, 199]) {
        bits.set(index);
      }
      expect(bits.setBits, [0, 63, 64, 127, 128, 199]);
    });
  });

  group('bulk operations', () {
    final a = Bitset.fromInteger(0x0c);
    final b = Bitset.fromInteger(0x0a);

    test('AND, OR, XOR, NOT, and AND-NOT truth tables', () {
      expect(a.bitwiseAnd(b).toInteger(), BigInt.from(0x08));
      expect(a.bitwiseOr(b).toInteger(), BigInt.from(0x0e));
      expect(a.bitwiseXor(b).toInteger(), BigInt.from(0x06));
      expect(b.bitwiseNot().toInteger(), BigInt.from(0x05));
      expect(a.andNot(b).toInteger(), BigInt.from(0x04));
    });

    test('operators delegate to bulk methods', () {
      expect((a & b).toInteger(), BigInt.from(0x08));
      expect((a | b).toInteger(), BigInt.from(0x0e));
      expect((a ^ b).toInteger(), BigInt.from(0x06));
      expect((~b).toInteger(), BigInt.from(0x05));
    });

    test('different lengths are zero-extended to the maximum length', () {
      final short = Bitset.fromInteger(0x0a);
      final long = Bitset.fromInteger(0xcc);
      final union = short | long;
      expect(union.length, 8);
      expect(union.toInteger(), BigInt.from(0xce));
      expect((short & Bitset()).length, short.length);
      expect((short & Bitset()).none, isTrue);
    });

    test('bulk operations do not mutate operands', () {
      final left = a.toInteger();
      final right = b.toInteger();
      a & b;
      a | b;
      a ^ b;
      ~a;
      a.andNot(b);
      expect(a.toInteger(), left);
      expect(b.toInteger(), right);
    });

    test('NOT cleans trailing capacity bits', () {
      final result = ~Bitset.fromBinaryString('10101');
      expect(result.toBinaryString(), '01010');
      expect(result.popcount, 2);
    });

    test('NOT clears whole spare words left by capacity doubling', () {
      final bits = Bitset()..set(128);
      expect(bits.length, 129);
      expect(bits.capacity, 256);
      final result = ~bits;
      expect(result.length, 129);
      expect(result.capacity, 256);
      expect(result.popcount, 128);
      expect(result.toInteger().bitLength, lessThanOrEqualTo(129));
      expect(result.setBits.last, 127);
    });
  });

  group('queries and iteration', () {
    test('popcount, any, none, and all', () {
      final bits = Bitset(70);
      expect(bits.popcount, 0);
      expect(bits.hasAny, isFalse);
      expect(bits.none, isTrue);
      expect(bits.all, isFalse);
      for (var index = 0; index < bits.length; index++) {
        bits.set(index);
      }
      expect(bits.popcount, 70);
      expect(bits.all, isTrue);
      bits.clear(69);
      expect(bits.all, isFalse);
    });

    test('iteration yields ascending set-bit indices', () {
      final bits = Bitset.fromInteger(0xa5);
      expect(bits.setBits, [0, 2, 5, 7]);
      expect(bits.iterSetBits(), [0, 2, 5, 7]);
      expect(bits.toList(), [0, 2, 5, 7]);
      final collected = <int>[];
      for (final index in bits) {
        collected.add(index);
      }
      expect(collected, [0, 2, 5, 7]);
      expect(bits.contains(5), isTrue);
      expect(bits.contains(6), isFalse);
      expect(bits.contains('5'), isFalse);
    });

    test('large sparse iteration matches membership', () {
      final bits = Bitset(10000);
      for (var index = 0; index < bits.length; index += 3) {
        bits.set(index);
      }
      expect(bits.popcount, (10000 + 2) ~/ 3);
      expect(bits.setBits, [
        for (var index = 0; index < 10000; index += 3) index,
      ]);
    });
  });

  group('identity and algebra', () {
    test('length participates in equality and hash code', () {
      final a = Bitset.fromBinaryString('101');
      final b = Bitset.fromInteger(5);
      final leadingZero = Bitset.fromBinaryString('0101');
      expect(a == b, isTrue);
      expect(a.hashCode, b.hashCode);
      expect(a == leadingZero, isFalse);
      expect(a == 5, isFalse);
    });

    test('boolean algebra laws hold across multiple words', () {
      final a = Bitset(200);
      final b = Bitset(200);
      for (var index = 0; index < 200; index += 3) {
        a.set(index);
      }
      for (var index = 0; index < 200; index += 5) {
        b.set(index);
      }
      expect(~(a & b), (~a) | (~b));
      expect(~(a | b), (~a) & (~b));
      expect((a ^ b) ^ b, a);
      expect(a & a, a);
      expect(a | a, a);
      expect((a ^ a).none, isTrue);
      expect(a.andNot(b), a & (~b));
    });

    test('word operations match BigInt at boundary-heavy widths', () {
      for (final width in [0, 1, 5, 63, 64, 65, 127, 128, 129, 200]) {
        final a = Bitset(width);
        final b = Bitset(width);
        for (var index = 0; index < width; index++) {
          if (index % 3 == 0) a.set(index);
          if (index % 5 == 0) b.set(index);
        }
        final av = a.toInteger();
        final bv = b.toInteger();
        final mask =
            width == 0 ? BigInt.zero : (BigInt.one << width) - BigInt.one;
        expect((a & b).toInteger(), av & bv, reason: 'AND width=$width');
        expect((a | b).toInteger(), av | bv, reason: 'OR width=$width');
        expect((a ^ b).toInteger(), av ^ bv, reason: 'XOR width=$width');
        expect((~a).toInteger(), mask ^ av, reason: 'NOT width=$width');
        expect(
          a.andNot(b).toInteger(),
          av & (mask ^ bv),
          reason: 'AND-NOT width=$width',
        );
      }
    });

    test('string and integer conversions round-trip', () {
      for (final value in [0, 1, 5, 42, 255, 65535]) {
        final first = Bitset.fromInteger(value);
        final second = Bitset.fromBinaryString(first.toBinaryString());
        expect(second, first);
        expect(second.toInteger(), BigInt.from(value));
      }
      expect(Bitset.fromInteger(5).toString(), "Bitset('101')");
      expect(Bitset().toString(), "Bitset('')");
      expect(Bitset.fromBinaryStr('101').toBinaryStr(), '101');
    });

    test('copy is independent', () {
      final original = Bitset.fromInteger(42);
      final copied = original.copy();
      original.set(100);
      expect(copied.toInteger(), BigInt.from(42));
      expect(copied, isNot(original));
    });
  });
}
