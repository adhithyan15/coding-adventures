/// Unit tests for `coding_adventures_x25519`: all 21 reference
/// tests mirrored case-for-case (field arithmetic, cswap, clamping,
/// RFC 7748 vectors, Alice/Bob exchange, keypair alias, 1- and
/// 1000-iteration chains, base point, codec round-trip), plus lane
/// extras (length checks, the low-order error, error values). Field
/// tests import `src/field.dart` directly, following the ed25519
/// lane.
library;

import 'dart:typed_data';

import 'package:coding_adventures_x25519/src/field.dart';
import 'package:coding_adventures_x25519/x25519.dart';
import 'package:test/test.dart';

Uint8List _unhex(String hex) {
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

void main() {
  group('field arithmetic', () {
    test('fe roundtrip', () {
      final original = Fe(BigInt.from(42));
      final bytes = original.toBytes();
      final recovered = Fe.fromBytes(bytes);
      expect(recovered.toBytes(), equals(bytes));
      expect(bytes[0], equals(42));
      for (final b in bytes.sublist(1)) {
        expect(b, equals(0));
      }
    });

    test('fe add', () {
      final c = Fe(BigInt.from(3)) + Fe(BigInt.from(5));
      expect(c.toBytes()[0], equals(8));
    });

    test('fe mul', () {
      final c = Fe(BigInt.from(7)) * Fe(BigInt.from(6));
      expect(c.toBytes()[0], equals(42));
    });

    test('fe square matches mul', () {
      final a = Fe(BigInt.from(7));
      expect(a.square().toBytes(), equals((a * a).toBytes()));
    });

    test('fe invert', () {
      final a = Fe(BigInt.from(42));
      expect((a * a.invert()).toBytes(), equals(Fe.one.toBytes()));
    });

    test('fe invert various', () {
      for (final val in [1, 2, 3, 7, 42, 121666]) {
        final a = Fe(BigInt.from(val));
        expect((a * a.invert()).toBytes(), equals(Fe.one.toBytes()),
            reason: 'invert failed for $val');
      }
    });

    test('fe sub', () {
      final c = Fe(BigInt.from(10)) - Fe(BigInt.from(3));
      expect(c.toBytes()[0], equals(7));
    });

    test('fe from-bytes to-bytes roundtrip', () {
      final bytes = Uint8List(32);
      bytes[0] = 0xff;
      bytes[15] = 0xab;
      bytes[31] = 0x7f;
      expect(Fe.fromBytes(bytes).toBytes(), equals(bytes));
    });
  });

  group('cswap', () {
    test('no swap', () {
      final (a, b) = cswap(0, Fe(BigInt.from(10)), Fe(BigInt.from(20)));
      expect(a.toBytes()[0], equals(10));
      expect(b.toBytes()[0], equals(20));
    });

    test('swap', () {
      final (a, b) = cswap(1, Fe(BigInt.from(10)), Fe(BigInt.from(20)));
      expect(a.toBytes()[0], equals(20));
      expect(b.toBytes()[0], equals(10));
    });
  });

  group('clamping', () {
    test('clamp scalar', () {
      final clamped = clampScalar(Uint8List.fromList(List.filled(32, 0xff)));
      expect(clamped[0] & 0x07, equals(0));
      expect(clamped[31] >> 7, equals(0));
      expect((clamped[31] >> 6) & 1, equals(1));
    });

    test('clamp sets bit 254', () {
      final clamped = clampScalar(Uint8List(32));
      expect((clamped[31] >> 6) & 1, equals(1));
    });
  });

  group('RFC 7748 vectors', () {
    test('vector 1', () {
      final scalar = _unhex(
          'a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4');
      final u = _unhex(
          'e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c');
      expect(
        _hex(x25519(scalar, u)),
        equals(
            'c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552'),
      );
    });

    test('vector 2', () {
      final scalar = _unhex(
          '4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d');
      final u = _unhex(
          'e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493');
      expect(
        _hex(x25519(scalar, u)),
        equals(
            '95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957'),
      );
    });

    test('alice public key', () {
      final alicePrivate = _unhex(
          '77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a');
      expect(
        _hex(x25519Base(alicePrivate)),
        equals(
            '8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a'),
      );
    });

    test('bob public key', () {
      final bobPrivate = _unhex(
          '5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb');
      expect(
        _hex(x25519Base(bobPrivate)),
        equals(
            'de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f'),
      );
    });

    test('shared secret', () {
      final alicePrivate = _unhex(
          '77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a');
      final bobPrivate = _unhex(
          '5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb');
      final alicePublic = x25519Base(alicePrivate);
      final bobPublic = x25519Base(bobPrivate);
      const expected =
          '4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742';
      expect(_hex(x25519(alicePrivate, bobPublic)), equals(expected));
      expect(_hex(x25519(bobPrivate, alicePublic)), equals(expected));
    });

    test('generate keypair is x25519 base', () {
      final private = _unhex(
          '77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a');
      expect(generateKeypair(private), equals(x25519Base(private)));
    });
  });

  group('iteration chains', () {
    test('1 iteration', () {
      final k = Uint8List(32)..[0] = 9;
      final u = Uint8List(32)..[0] = 9;
      expect(
        _hex(x25519(k, u)),
        equals(
            '422c8e7a6227d7bca1350b3e2bb7279f7897b87bb6854b783c60e80311ae3079'),
      );
    });

    test('1000 iterations', () {
      var k = Uint8List(32)..[0] = 9;
      var u = Uint8List(32)..[0] = 9;
      for (var i = 0; i < 1000; i++) {
        final next = x25519(k, u);
        u = k;
        k = next;
      }
      expect(
        _hex(k),
        equals(
            '684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51'),
      );
    });
  });

  group('base point', () {
    test('base point is nine', () {
      final expected = Uint8List(32)..[0] = 9;
      expect(basePoint, equals(expected));
    });
  });

  group('field values', () {
    test('equality and hashes', () {
      expect(Fe(BigInt.from(7)), equals(Fe(BigInt.from(7))));
      expect(Fe(BigInt.from(7)), isNot(equals(Fe(BigInt.from(8)))));
      expect(Fe(BigInt.from(7)), isNot(equals(7)));
      expect(
        Fe(BigInt.from(7)).hashCode,
        equals(Fe(BigInt.from(7)).hashCode),
      );
      expect(
        const X25519LowOrderPoint(),
        equals(const X25519LowOrderPoint()),
      );
      expect(
        const X25519LowOrderPoint().hashCode,
        equals(const X25519LowOrderPoint().hashCode),
      );
    });
  });

  group('lane extras', () {
    test('lengths are checked', () {
      final good = Uint8List(32);
      expect(() => x25519(Uint8List(31), good), throwsArgumentError);
      expect(() => x25519(good, Uint8List(33)), throwsArgumentError);
      expect(() => x25519Base(Uint8List(0)), throwsArgumentError);
      expect(() => generateKeypair(Uint8List(31)), throwsArgumentError);
      expect(() => clampScalar(Uint8List(31)), throwsArgumentError);
    });

    test('low-order input throws with the reference message', () {
      expect(
        () => x25519(Uint8List(32), Uint8List(32)),
        throwsA(equals(const X25519LowOrderPoint())),
      );
      expect(
        const X25519LowOrderPoint().toString(),
        equals('X25519 produced the all-zeros output (low-order point)'),
      );
    });
  });
}
