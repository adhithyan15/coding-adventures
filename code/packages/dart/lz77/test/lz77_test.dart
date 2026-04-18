import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_lz77/lz77.dart';
import 'package:test/test.dart';

void main() {
  group('spec vectors', () {
    test('empty input produces no tokens', () {
      expect(encode(Uint8List(0)), isEmpty);
      expect(decode(const <Token>[]), isEmpty);
    });

    test('no repetition emits literal tokens', () {
      final tokens = encode(_enc('ABCDE'));
      expect(tokens, hasLength(5));
      for (final current in tokens) {
        expect(current.offset, 0);
        expect(current.length, 0);
      }
    });

    test('all identical bytes exploit overlapping matches', () {
      final tokens = encode(_enc('AAAAAAA'));
      expect(tokens, hasLength(2));
      expect(tokens[0], token(0, 0, 65));
      expect(tokens[1].offset, 1);
      expect(tokens[1].length, 5);
      expect(tokens[1].nextChar, 65);

      expect(_dec(decode(tokens)), 'AAAAAAA');
    });

    test('repeated pairs become a backreference', () {
      final tokens = encode(_enc('ABABABAB'));
      expect(tokens, hasLength(3));
      expect(tokens[0], token(0, 0, 65));
      expect(tokens[1], token(0, 0, 66));
      expect(tokens[2].offset, 2);
      expect(tokens[2].length, 5);
      expect(tokens[2].nextChar, 66);

      expect(_dec(decode(tokens)), 'ABABABAB');
    });

    test('AABCBBABC with minMatch 3 stays literal-only', () {
      final tokens = encode(_enc('AABCBBABC'));
      expect(tokens, hasLength(9));
      for (final current in tokens) {
        expect(current.offset, 0);
        expect(current.length, 0);
      }

      expect(_dec(decode(tokens)), 'AABCBBABC');
    });

    test('AABCBBABC with minMatch 2 still round-trips', () {
      final tokens = encode(_enc('AABCBBABC'), 4096, 255, 2);
      expect(_dec(decode(tokens)), 'AABCBBABC');
    });
  });

  group('round trip', () {
    test('empty data round-trips', () {
      expect(decode(encode(Uint8List(0))), isEmpty);
    });

    test('single bytes round-trip', () {
      for (final value in <int>[65, 0, 255]) {
        expect(decode(encode(_bytes(value))), _bytes(value));
      }
    });

    for (final sample in <String>[
      'hello world',
      'the quick brown fox',
      'ababababab',
      'aaaaaaaaaa',
    ]) {
      test('string round-trips: $sample', () {
        expect(_dec(decode(encode(_enc(sample)))), sample);
      });
    }

    test('null bytes round-trip', () {
      final data = _bytes(0, 0, 0);
      expect(decode(encode(data)), data);
    });

    test('0xFF bytes round-trip', () {
      final data = _bytes(255, 255, 255);
      expect(decode(encode(data)), data);
    });

    test('all 256 byte values round-trip', () {
      final data = Uint8List.fromList(
        List<int>.generate(256, (index) => index),
      );
      expect(decode(encode(data)), data);
    });

    test('compress and decompress round-trip', () {
      for (final sample in <String>[
        '',
        'A',
        'ABCDE',
        'AAAAAAA',
        'ABABABAB',
        'hello world',
      ]) {
        final data = _enc(sample);
        expect(decompress(compress(data)), data);
      }
    });
  });

  group('parameters', () {
    test('offsets never exceed window size', () {
      final data = Uint8List(5002);
      data[0] = 88;
      for (var index = 1; index < 5001; index++) {
        data[index] = 89;
      }
      data[5001] = 88;

      final tokens = encode(data, 100);
      for (final current in tokens) {
        expect(current.offset, lessThanOrEqualTo(100));
      }
    });

    test('lengths never exceed maxMatch', () {
      final data = Uint8List.fromList(List<int>.filled(1000, 65));
      final tokens = encode(data, 4096, 50);
      for (final current in tokens) {
        expect(current.length, lessThanOrEqualTo(50));
      }
    });

    test('short matches stay as literals', () {
      final tokens = encode(_enc('AABAA'), 4096, 255, 2);
      for (final current in tokens) {
        expect(current.length == 0 || current.length >= 2, isTrue);
      }
    });
  });

  group('edge cases', () {
    test('single byte encodes as a literal token', () {
      final tokens = encode(_enc('X'));
      expect(tokens, hasLength(1));
      expect(tokens[0], token(0, 0, 88));
    });

    test('exact window boundary matches decode correctly', () {
      const window = 10;
      final data = Uint8List.fromList(List<int>.filled(window + 1, 88));
      final tokens = encode(data, window);
      expect(tokens.any((current) => current.offset > 0), isTrue);
      expect(decode(tokens), data);
    });

    test('overlapping matches are copied byte by byte', () {
      final tokens = <Token>[token(0, 0, 65), token(0, 0, 66), token(2, 5, 90)];

      expect(_dec(decode(tokens)), 'ABABABAZ');
    });

    test('binary data with nulls round-trips', () {
      final data = _bytes(0, 0, 0, 255, 255);
      expect(decode(encode(data)), data);
    });

    test('very long inputs round-trip', () {
      final repeated = _enc(List<String>.filled(100, 'Hello, World! ').join());
      final extra = Uint8List.fromList(List<int>.filled(500, 88));
      final data = Uint8List.fromList(<int>[...repeated, ...extra]);
      expect(decode(encode(data)), data);
    });

    test('long runs compress into relatively few tokens', () {
      final data = Uint8List.fromList(List<int>.filled(10000, 65));
      final tokens = encode(data);
      expect(tokens.length, lessThan(50));
      expect(decode(tokens), data);
    });

    test('initial buffer participates in decoding', () {
      final result = decode(<Token>[token(2, 3, 90)], _bytes(65, 66));
      expect(_dec(result), 'ABABAZ');
    });

    test('decode rejects zero-offset backreferences', () {
      expect(
        () => decode(<Token>[token(0, 3, 90)]),
        throwsA(isA<FormatException>()),
      );
    });

    test('decode rejects offsets beyond the decoded prefix', () {
      expect(
        () => decode(<Token>[token(2, 1, 90)], _bytes(65)),
        throwsA(isA<FormatException>()),
      );
    });

    test('decode rejects literal tokens with a non-zero offset', () {
      expect(
        () => decode(<Token>[token(1, 0, 90)]),
        throwsA(isA<FormatException>()),
      );
    });

    test('decode rejects negative match lengths', () {
      expect(
        () => decode(<Token>[token(0, -1, 90)]),
        throwsA(isA<FormatException>()),
      );
    });

    test('decode rejects nextChar values outside byte range', () {
      expect(
        () => decode(<Token>[token(0, 0, 256)]),
        throwsA(isA<FormatException>()),
      );
    });
  });

  group('serialisation', () {
    test('serialised size is four plus four bytes per token', () {
      final tokens = <Token>[token(0, 0, 65), token(2, 5, 66)];
      final serialised = serialiseTokens(tokens);
      expect(serialised.length, 4 + 2 * 4);
    });

    test('serialise and deserialise are inverses', () {
      final tokens = <Token>[token(0, 0, 65), token(1, 3, 66), token(2, 5, 67)];
      final serialised = serialiseTokens(tokens);
      expect(deserialiseTokens(serialised), tokens);
    });

    test('empty serialised data yields no tokens', () {
      expect(deserialiseTokens(Uint8List(0)), isEmpty);
    });

    test('truncated serialised data is rejected', () {
      expect(
        () => deserialiseTokens(Uint8List.fromList(<int>[0, 0, 0, 1, 0, 2, 3])),
        throwsA(isA<FormatException>()),
      );
    });

    test('deserialised backreferences must use a positive offset', () {
      expect(
        () => deserialiseTokens(
          Uint8List.fromList(<int>[0, 0, 0, 1, 0, 0, 3, 90]),
        ),
        throwsA(isA<FormatException>()),
      );
    });

    test('compress and decompress all spec vectors', () {
      for (final sample in <String>[
        '',
        'ABCDE',
        'AAAAAAA',
        'ABABABAB',
        'AABCBBABC',
      ]) {
        final data = _enc(sample);
        expect(decompress(compress(data)), data);
      }
    });
  });

  group('behaviour', () {
    test('incompressible data stays within the fixed-width overhead bound', () {
      final data = Uint8List.fromList(
        List<int>.generate(256, (index) => index),
      );
      final compressed = compress(data);
      expect(compressed.length, lessThanOrEqualTo(4 * data.length + 10));
    });

    test('repetitive data compresses to fewer bytes', () {
      final data = _enc(List<String>.filled(100, 'ABC').join());
      final compressed = compress(data);
      expect(compressed.length, lessThan(data.length));
    });

    test('compression is deterministic', () {
      final data = _enc('hello world test');
      expect(compress(data), compress(data));
    });
  });
}

Uint8List _enc(String value) => Uint8List.fromList(utf8.encode(value));

String _dec(Uint8List bytes) => utf8.decode(bytes);

Uint8List _bytes(
  int first, [
  int? second,
  int? third,
  int? fourth,
  int? fifth,
]) {
  final values = <int>[first];
  if (second != null) {
    values.add(second);
  }
  if (third != null) {
    values.add(third);
  }
  if (fourth != null) {
    values.add(fourth);
  }
  if (fifth != null) {
    values.add(fifth);
  }
  return Uint8List.fromList(values);
}
