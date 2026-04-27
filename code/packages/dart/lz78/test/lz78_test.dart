import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_lz78/lz78.dart';
import 'package:test/test.dart';

void main() {
  group('spec vectors', () {
    test('empty input', () {
      expect(encode(Uint8List(0)), isEmpty);
      expect(decode(const <Token>[], 0), isEmpty);
    });

    test('single byte', () {
      final tokens = encode(_enc('A'));
      expect(tokens, <Token>[token(0, 65)]);
      expect(decode(tokens, 1), _enc('A'));
    });

    test('no repetition emits literal-only tokens', () {
      final tokens = encode(_enc('ABCDE'));
      expect(tokens, hasLength(5));
      for (final current in tokens) {
        expect(current.dictIndex, 0);
      }
    });

    test('AABCBBABC matches the spec trace', () {
      final want = <Token>[
        token(0, 65),
        token(1, 66),
        token(0, 67),
        token(0, 66),
        token(4, 65),
        token(4, 67),
      ];
      expect(encode(_enc('AABCBBABC')), want);
      expect(_roundTripString('AABCBBABC'), 'AABCBBABC');
    });

    test('ABABAB ends with a flush token', () {
      final want = <Token>[
        token(0, 65),
        token(0, 66),
        token(1, 66),
        token(3, 0),
      ];
      expect(encode(_enc('ABABAB')), want);
      expect(_roundTripString('ABABAB'), 'ABABAB');
    });

    test('all identical bytes produce dictionary growth', () {
      final tokens = encode(_enc('AAAAAAA'));
      expect(tokens, hasLength(4));
      expect(tokens[0], token(0, 65));
      expect(tokens[1], token(1, 65));
      expect(tokens[2], token(2, 65));
      expect(tokens[3], token(1, 0));
    });

    test('repeated pairs compress', () {
      expect(_roundTripString('ABABABAB'), 'ABABABAB');
      expect(encode(_enc('ABABABAB')).length, lessThan(8));
    });
  });

  group('round trip', () {
    for (final sample in <String>[
      '',
      'A',
      'ABCDE',
      'AAAAAAA',
      'ABABABAB',
      'AABCBBABC',
      'hello world',
      'the quick brown fox',
      'ababababab',
      'aaaaaaaaaa',
    ]) {
      test('ascii round-trip: $sample', () {
        expect(_roundTripString(sample), sample);
      });
    }

    test('binary zeros', () {
      final data = Uint8List(3);
      expect(decompress(compress(data)), data);
    });

    test('binary 255s', () {
      final data = Uint8List.fromList(<int>[255, 255, 255]);
      expect(decompress(compress(data)), data);
    });

    test('full byte range', () {
      final data = Uint8List.fromList(
        List<int>.generate(256, (index) => index),
      );
      expect(decompress(compress(data)), data);
    });

    test('binary repeat', () {
      final data = Uint8List.fromList(<int>[0, 1, 2, 0, 1, 2]);
      expect(decompress(compress(data)), data);
    });

    test('binary null and max mix', () {
      final data = Uint8List.fromList(<int>[0, 0, 0, 255, 255]);
      expect(decompress(compress(data)), data);
    });
  });

  group('parameters', () {
    test('maxDictSize is respected', () {
      final tokens = encode(_enc('ABCABCABCABCABC'), 10);
      for (final current in tokens) {
        expect(current.dictIndex, lessThan(10));
      }
    });

    test('maxDictSize 1 keeps everything literal', () {
      final tokens = encode(_enc('AAAA'), 1);
      for (final current in tokens) {
        expect(current.dictIndex, 0);
      }
    });
  });

  group('trie cursor', () {
    test('step insert and reset mirror the encoder model', () {
      final cursor = TrieCursor();
      expect(cursor.atRoot, isTrue);
      expect(cursor.dictId, 0);

      expect(cursor.step(65), isFalse);
      cursor.insert(65, 1);
      expect(cursor.step(65), isTrue);
      expect(cursor.dictId, 1);
      expect(cursor.atRoot, isFalse);

      cursor.reset();
      expect(cursor.atRoot, isTrue);
      expect(cursor.dictId, 0);
    });
  });

  group('edge cases', () {
    test('single byte stays literal', () {
      expect(encode(_enc('X')), <Token>[token(0, 88)]);
    });

    test('two literals decode cleanly', () {
      final tokens = <Token>[token(0, 65), token(0, 66)];
      expect(decode(tokens), _enc('AB'));
    });

    test('flush token round-trip', () {
      expect(_roundTripString('ABABAB'), 'ABABAB');
    });

    test('all null bytes round-trip', () {
      final data = Uint8List(100);
      expect(decompress(compress(data)), data);
    });

    test('all max bytes round-trip', () {
      final data = Uint8List.fromList(List<int>.filled(100, 255));
      expect(decompress(compress(data)), data);
    });

    test('very long inputs round-trip', () {
      final repeated = _enc(List<String>.filled(100, 'Hello, World! ').join());
      final suffix = Uint8List.fromList(
        List<int>.generate(256, (index) => index),
      );
      final data = Uint8List.fromList(<int>[...repeated, ...suffix]);
      expect(decompress(compress(data)), data);
    });

    test('decode rejects dictionary indexes beyond the built table', () {
      expect(
        () => decode(<Token>[token(1, 65)]),
        throwsA(isA<FormatException>()),
      );
    });

    test('decode rejects negative dictionary indexes', () {
      expect(
        () => decode(<Token>[token(-1, 65)]),
        throwsA(isA<FormatException>()),
      );
    });

    test('decode rejects nextChar outside byte range', () {
      expect(
        () => decode(<Token>[token(0, 256)]),
        throwsA(isA<FormatException>()),
      );
    });

    test('decode rejects declared lengths that are too large', () {
      expect(
        () => decode(<Token>[token(0, 65)], 2),
        throwsA(isA<FormatException>()),
      );
    });

    test('decode rejects declared lengths that are too small', () {
      expect(
        () => decode(<Token>[token(0, 65)], 0),
        throwsA(isA<FormatException>()),
      );
    });

    test('decode rejects overflow caused by reconstructed sequences', () {
      expect(
        () => decode(<Token>[token(0, 65), token(1, 0)], 1),
        throwsA(isA<FormatException>()),
      );
    });
  });

  group('serialisation', () {
    test('wire format is eight bytes plus four per token', () {
      final compressed = compress(_enc('AB'));
      final tokens = encode(_enc('AB'));
      expect(compressed.length, 8 + tokens.length * 4);
    });

    test('deserialise round-trip', () {
      final tokens = <Token>[token(0, 65), token(1, 66)];
      final decoded = deserialiseTokens(serialiseTokens(tokens, 3));
      expect(decoded.tokens, tokens);
      expect(decoded.originalLength, 3);
    });

    test('all spec vectors survive compress and decompress', () {
      for (final sample in <String>[
        '',
        'A',
        'ABCDE',
        'AAAAAAA',
        'ABABABAB',
        'AABCBBABC',
      ]) {
        expect(_roundTripString(sample), sample);
      }
    });

    test('compression is deterministic', () {
      final data = _enc('hello world test data repeated');
      expect(compress(data), compress(data));
    });

    test('truncated token streams are rejected', () {
      expect(
        () => deserialiseTokens(
          Uint8List.fromList(<int>[0, 0, 0, 2, 0, 0, 0, 1, 0, 1]),
        ),
        throwsA(isA<FormatException>()),
      );
    });

    test('incomplete headers are rejected', () {
      expect(
        () => deserialiseTokens(Uint8List.fromList(<int>[0, 0, 0, 2])),
        throwsA(isA<FormatException>()),
      );
    });

    test('trailing bytes beyond the declared token stream are rejected', () {
      expect(
        () => deserialiseTokens(
          Uint8List.fromList(<int>[0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 65, 0, 99]),
        ),
        throwsA(isA<FormatException>()),
      );
    });
  });

  group('behaviour', () {
    test('repetitive data compresses', () {
      final data = _enc(List<String>.filled(1000, 'ABC').join());
      expect(compress(data).length, lessThan(data.length));
    });

    test('incompressible data does not expand excessively', () {
      final data = Uint8List.fromList(
        List<int>.generate(256, (index) => index),
      );
      expect(compress(data).length, lessThanOrEqualTo(4 * data.length + 10));
    });

    test('single-byte repetitions compress and round-trip', () {
      final data = Uint8List.fromList(List<int>.filled(10000, 65));
      expect(compress(data).length, lessThan(data.length));
      expect(decompress(compress(data)), data);
    });
  });
}

Uint8List _enc(String value) => Uint8List.fromList(utf8.encode(value));

String _roundTripString(String value) =>
    utf8.decode(decompress(compress(_enc(value))));
