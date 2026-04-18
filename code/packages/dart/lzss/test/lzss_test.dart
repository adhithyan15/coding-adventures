import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_lzss/lzss.dart';
import 'package:test/test.dart';

void main() {
  group('spec vectors', () {
    test('empty input', () {
      expect(encode(Uint8List(0)), isEmpty);
      expect(decode(const <Token>[], 0), isEmpty);
      expect(
        compress(Uint8List(0)),
        Uint8List.fromList(<int>[0, 0, 0, 0, 0, 0, 0, 0]),
      );
    });

    test('single byte', () {
      final tokens = encode(_enc('A'));
      expect(tokens, <Token>[literal(65)]);
      expect(decode(tokens, 1), _enc('A'));
    });

    test('no repetition emits literal-only tokens', () {
      final tokens = encode(_enc('ABCDE'));
      expect(tokens, <Token>[
        literal(65),
        literal(66),
        literal(67),
        literal(68),
        literal(69),
      ]);
    });

    test('ABABAB emits two literals then one match', () {
      final tokens = encode(_enc('ABABAB'));
      expect(tokens, <Token>[literal(65), literal(66), match(2, 4)]);
      expect(_roundTripString('ABABAB'), 'ABABAB');
    });

    test('AAAAAAA becomes one literal and one overlapping match', () {
      final tokens = encode(_enc('AAAAAAA'));
      expect(tokens, <Token>[literal(65), match(1, 6)]);
      expect(_roundTripString('AAAAAAA'), 'AAAAAAA');
    });

    test('AABCBBABC keeps a mixed literal and match trace', () {
      final tokens = encode(_enc('AABCBBABC'));
      expect(tokens, <Token>[
        literal(65),
        literal(65),
        literal(66),
        literal(67),
        literal(66),
        literal(66),
        match(5, 3),
      ]);
      expect(_roundTripString('AABCBBABC'), 'AABCBBABC');
    });
  });

  group('round trip', () {
    for (final sample in <String>[
      '',
      'A',
      'ABCDE',
      'AAAAAAA',
      'ABABAB',
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
      final data = Uint8List.fromList(<int>[0, 1, 2, 0, 1, 2, 0, 1, 2]);
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

    test('custom decompression limit can allow a larger trusted payload', () {
      final data = _enc('hello');
      expect(decompress(compress(data), data.length), data);
    });
  });

  group('parameters', () {
    test('windowSize is respected', () {
      final tokens = encode(_enc('ABCABCABCABC'), 3);
      for (final current in tokens.whereType<Match>()) {
        expect(current.offset, lessThanOrEqualTo(3));
      }
    });

    test('maxMatch is respected', () {
      final tokens = encode(_enc('AAAAAAAAAA'), defaultWindowSize, 4, 3);
      for (final current in tokens.whereType<Match>()) {
        expect(current.length, lessThanOrEqualTo(4));
      }
    });

    test('high minMatch keeps short repeats literal', () {
      final tokens = encode(
        _enc('ABABAB'),
        defaultWindowSize,
        defaultMaxMatch,
        10,
      );
      expect(tokens, <Token>[
        literal(65),
        literal(66),
        literal(65),
        literal(66),
        literal(65),
        literal(66),
      ]);
    });

    test('invalid encoder parameters are rejected', () {
      expect(() => encode(_enc('abc'), 0), throwsRangeError);
      expect(() => encode(_enc('abc'), 1, 0), throwsRangeError);
      expect(() => encode(_enc('abc'), 1, 1, 0), throwsRangeError);
    });
  });

  group('token value semantics', () {
    test('literals compare by value and expose readable debug strings', () {
      expect(literal(65), literal(65));
      expect(literal(65).hashCode, literal(65).hashCode);
      expect(literal(65).toString(), 'Literal(byte: 65)');
    });

    test('matches compare by value and expose readable debug strings', () {
      expect(match(2, 4), match(2, 4));
      expect(match(2, 4).hashCode, match(2, 4).hashCode);
      expect(match(2, 4).toString(), 'Match(offset: 2, length: 4)');
    });
  });

  group('decode validation', () {
    test('negative declared lengths are rejected', () {
      expect(() => decode(<Token>[literal(65)], -2), throwsRangeError);
    });

    test('overlapping match decoding is supported', () {
      final tokens = <Token>[literal(65), match(1, 6)];
      expect(decode(tokens, 7), _enc('AAAAAAA'));
    });

    test('offset beyond the produced output is rejected', () {
      expect(
        () => decode(<Token>[match(1, 3)]),
        throwsA(isA<FormatException>()),
      );
    });

    test('zero offset is rejected', () {
      expect(
        () => decode(<Token>[literal(65), match(0, 3)]),
        throwsA(isA<FormatException>()),
      );
    });

    test('zero match length is rejected', () {
      expect(
        () => decode(<Token>[literal(65), match(1, 0)]),
        throwsA(isA<FormatException>()),
      );
    });

    test('literal bytes outside the byte range are rejected', () {
      expect(
        () => decode(<Token>[literal(256)]),
        throwsA(isA<FormatException>()),
      );
    });

    test('declared lengths that are too small are rejected', () {
      expect(
        () => decode(<Token>[literal(65)], 0),
        throwsA(isA<FormatException>()),
      );
    });

    test('declared lengths that are too large are rejected', () {
      expect(
        () => decode(<Token>[literal(65)], 2),
        throwsA(isA<FormatException>()),
      );
    });

    test('matches that overflow the declared length are rejected', () {
      expect(
        () => decode(<Token>[literal(65), match(1, 2)], 2),
        throwsA(isA<FormatException>()),
      );
    });
  });

  group('serialisation', () {
    test('wire format matches the CMP02 block layout', () {
      expect(
        compress(_enc('ABABAB')),
        Uint8List.fromList(<int>[0, 0, 0, 6, 0, 0, 0, 1, 4, 65, 66, 0, 2, 4]),
      );
    });

    test('deserialise round-trip', () {
      final tokens = <Token>[literal(65), literal(66), match(2, 4)];
      final decoded = deserialiseTokens(serialiseTokens(tokens, 6));
      expect(decoded.tokens, tokens);
      expect(decoded.originalLength, 6);
    });

    test('single partial final block is accepted', () {
      final decoded = deserialiseTokens(
        Uint8List.fromList(<int>[0, 0, 0, 1, 0, 0, 0, 1, 0, 65]),
      );
      expect(decoded.tokens, <Token>[literal(65)]);
      expect(decoded.originalLength, 1);
    });

    test('truncated token streams are rejected', () {
      expect(
        () => deserialiseTokens(
          Uint8List.fromList(<int>[0, 0, 0, 6, 0, 0, 0, 1, 4, 65, 66, 0, 2]),
        ),
        throwsA(isA<FormatException>()),
      );
    });

    test('incomplete headers are rejected', () {
      expect(
        () => deserialiseTokens(Uint8List.fromList(<int>[0, 0, 0, 1])),
        throwsA(isA<FormatException>()),
      );
    });

    test('non-positive decompression limits are rejected', () {
      expect(
        () => deserialiseTokens(
          Uint8List.fromList(<int>[0, 0, 0, 0, 0, 0, 0, 0]),
          0,
        ),
        throwsRangeError,
      );
      expect(
        () => decompress(Uint8List.fromList(<int>[0, 0, 0, 0, 0, 0, 0, 0]), 0),
        throwsRangeError,
      );
    });

    test('declared empty outputs must stay empty', () {
      expect(
        () => deserialiseTokens(
          Uint8List.fromList(<int>[0, 0, 0, 0, 0, 0, 0, 1, 0]),
        ),
        throwsA(isA<FormatException>()),
      );
    });

    test('declared block counts larger than the payload are rejected', () {
      expect(
        () => deserialiseTokens(
          Uint8List.fromList(<int>[0, 0, 0, 1, 0, 0, 0, 2, 0, 65]),
        ),
        throwsA(isA<FormatException>()),
      );
    });

    test('missing flag bytes for declared blocks are rejected', () {
      expect(
        () => deserialiseTokens(
          Uint8List.fromList(<int>[0, 0, 0, 1, 0, 0, 0, 1]),
        ),
        throwsA(isA<FormatException>()),
      );
    });

    test('blocks that end before a literal byte are rejected', () {
      expect(
        () => deserialiseTokens(
          Uint8List.fromList(<int>[0, 0, 0, 2, 0, 0, 0, 1, 0, 65]),
        ),
        throwsA(isA<FormatException>()),
      );
    });

    test('decoded lengths that exceed the header are rejected', () {
      expect(
        () => deserialiseTokens(
          Uint8List.fromList(<int>[0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 1, 2]),
        ),
        throwsA(isA<FormatException>()),
      );
    });

    test('decoded lengths that fall short of the header are rejected', () {
      expect(
        () => deserialiseTokens(
          Uint8List.fromList(<int>[0, 0, 0, 2, 0, 0, 0, 1, 0, 65]),
        ),
        throwsA(isA<FormatException>()),
      );
    });

    test('declared output lengths above the configured cap are rejected', () {
      expect(
        () => deserialiseTokens(
          Uint8List.fromList(<int>[0, 0, 0, 5, 0, 0, 0, 0]),
          4,
        ),
        throwsA(isA<FormatException>()),
      );
      expect(
        () => decompress(Uint8List.fromList(<int>[0, 0, 0, 5, 0, 0, 0, 0]), 4),
        throwsA(isA<FormatException>()),
      );
    });

    test('invalid serialised matches are rejected', () {
      expect(
        () => serialiseTokens(<Token>[match(0, 3)], 3),
        throwsA(isA<FormatException>()),
      );
      expect(
        () => serialiseTokens(<Token>[match(1, 0)], 1),
        throwsA(isA<FormatException>()),
      );
      expect(() => serialiseTokens(<Token>[literal(65)], -1), throwsRangeError);
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
      expect(compress(data).length, lessThanOrEqualTo(2 * data.length + 64));
    });

    test('compression is deterministic', () {
      final data = _enc('hello world test data repeated');
      expect(compress(data), compress(data));
    });
  });
}

Uint8List _enc(String value) => Uint8List.fromList(utf8.encode(value));

String _roundTripString(String value) =>
    utf8.decode(decompress(compress(_enc(value))));
