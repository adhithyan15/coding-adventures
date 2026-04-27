import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_lzw/lzw.dart';
import 'package:test/test.dart';

void main() {
  group('constants', () {
    test('match the CMP03 specification', () {
      expect(clearCode, 256);
      expect(stopCode, 257);
      expect(initialNextCode, 258);
      expect(initialCodeSize, 9);
      expect(maxCodeSize, 16);
    });
  });

  group('BitWriter and BitReader', () {
    test('empty writer flushes to an empty byte list', () {
      final writer = BitWriter();
      writer.flush();
      expect(writer.bytes(), isEmpty);
    });

    test('round-trips a single 9-bit code', () {
      final writer = BitWriter()
        ..write(clearCode, 9)
        ..flush();
      final reader = BitReader(writer.bytes());
      expect(reader.read(9), clearCode);
    });

    test('round-trips multiple 9-bit codes', () {
      const codes = <int>[clearCode, 65, 66, 258, stopCode];
      final writer = BitWriter();
      for (final code in codes) {
        writer.write(code, 9);
      }
      writer.flush();

      final reader = BitReader(writer.bytes());
      for (final code in codes) {
        expect(reader.read(9), code);
      }
    });

    test('reader rejects exhausted streams', () {
      final reader = BitReader(Uint8List(0));
      expect(() => reader.read(9), throwsA(isA<FormatException>()));
    });

    test('writer rejects codes that do not fit in the requested width', () {
      final writer = BitWriter();
      expect(() => writer.write(512, 9), throwsRangeError);
    });

    test('writer rejects non-positive code widths', () {
      final writer = BitWriter();
      expect(() => writer.write(1, 0), throwsRangeError);
    });

    test('reader rejects non-positive code widths', () {
      final reader = BitReader(Uint8List.fromList(<int>[0]));
      expect(() => reader.read(0), throwsRangeError);
    });
  });

  group('encodeCodes', () {
    test('empty input becomes CLEAR then STOP', () {
      expect(encodeCodes(Uint8List(0)), <int>[clearCode, stopCode]);
    });

    test('single byte becomes CLEAR 65 STOP', () {
      expect(encodeCodes(_enc('A')), <int>[clearCode, 65, stopCode]);
    });

    test('two distinct bytes become CLEAR 65 66 STOP', () {
      expect(encodeCodes(_enc('AB')), <int>[clearCode, 65, 66, stopCode]);
    });

    test('ABABAB matches the spec trace', () {
      expect(encodeCodes(_enc('ABABAB')), <int>[
        clearCode,
        65,
        66,
        258,
        258,
        stopCode,
      ]);
    });

    test('AAAAAAA matches the tricky-token trace', () {
      expect(encodeCodes(_enc('AAAAAAA')), <int>[
        clearCode,
        65,
        258,
        259,
        65,
        stopCode,
      ]);
    });
  });

  group('decodeCodes', () {
    test('CLEAR STOP decodes to empty output', () {
      expect(decodeCodes(const <int>[clearCode, stopCode]), isEmpty);
    });

    test('CLEAR 65 STOP decodes to A', () {
      expect(decodeCodes(const <int>[clearCode, 65, stopCode]), _enc('A'));
    });

    test('CLEAR 65 66 258 258 STOP decodes to ABABAB', () {
      expect(
        decodeCodes(const <int>[clearCode, 65, 66, 258, 258, stopCode]),
        _enc('ABABAB'),
      );
    });

    test('tricky-token stream decodes to AAAAAAA', () {
      expect(
        decodeCodes(const <int>[clearCode, 65, 258, 259, 65, stopCode]),
        _enc('AAAAAAA'),
      );
    });

    test('CLEAR mid-stream resets the dictionary', () {
      expect(
        decodeCodes(const <int>[clearCode, 65, clearCode, 66, stopCode]),
        _enc('AB'),
      );
    });

    test('requires CLEAR_CODE at the start', () {
      expect(
        () => decodeCodes(const <int>[65, stopCode]),
        throwsA(isA<FormatException>()),
      );
    });

    test('requires STOP_CODE at the end', () {
      expect(
        () => decodeCodes(const <int>[clearCode, 65]),
        throwsA(isA<FormatException>()),
      );
    });

    test('rejects codes after STOP_CODE', () {
      expect(
        () => decodeCodes(const <int>[clearCode, 65, stopCode, 66]),
        throwsA(isA<FormatException>()),
      );
    });

    test('rejects invalid codes beyond the next dictionary slot', () {
      expect(
        () => decodeCodes(const <int>[clearCode, 9999, stopCode]),
        throwsA(isA<FormatException>()),
      );
    });

    test('rejects negative codes', () {
      expect(
        () => decodeCodes(const <int>[clearCode, -1, stopCode]),
        throwsA(isA<FormatException>()),
      );
    });

    test('rejects tricky tokens without a previous code', () {
      expect(
        () => decodeCodes(const <int>[clearCode, 258, stopCode]),
        throwsA(isA<FormatException>()),
      );
    });

    test('rejects negative declared lengths', () {
      expect(
        () => decodeCodes(const <int>[clearCode, stopCode], -2),
        throwsRangeError,
      );
    });

    test('rejects outputs that exceed the declared length', () {
      expect(
        () => decodeCodes(const <int>[clearCode, 65, stopCode], 0),
        throwsA(isA<FormatException>()),
      );
    });

    test('rejects outputs that fall short of the declared length', () {
      expect(
        () => decodeCodes(const <int>[clearCode, 65, stopCode], 2),
        throwsA(isA<FormatException>()),
      );
    });
  });

  group('packCodes and unpackCodes', () {
    test('store original_length in the header', () {
      final packed = packCodes(const <int>[clearCode, stopCode], 42);
      final view = ByteData.sublistView(packed);
      expect(view.getUint32(0, Endian.big), 42);
    });

    test('round-trip ABABAB codes through the wire format', () {
      const codes = <int>[clearCode, 65, 66, 258, 258, stopCode];
      final unpacked = unpackCodes(packCodes(codes, 6));
      expect(unpacked.originalLength, 6);
      expect(unpacked.codes, codes);
    });

    test('round-trip AAAAAAA codes through the wire format', () {
      const codes = <int>[clearCode, 65, 258, 259, 65, stopCode];
      final unpacked = unpackCodes(packCodes(codes, 7));
      expect(unpacked.originalLength, 7);
      expect(unpacked.codes, codes);
    });

    test('empty input packs to a 4-byte header plus 3-byte payload', () {
      expect(compress(Uint8List(0)).length, 7);
    });

    test('rejects incomplete headers', () {
      expect(
        () => unpackCodes(Uint8List.fromList(<int>[0, 0, 0])),
        throwsA(isA<FormatException>()),
      );
    });

    test('rejects empty payloads after the header', () {
      expect(
        () => unpackCodes(Uint8List.fromList(<int>[0, 0, 0, 0])),
        throwsA(isA<FormatException>()),
      );
    });

    test('rejects missing CLEAR_CODE at the start', () {
      final writer = BitWriter()
        ..write(65, 9)
        ..flush();
      final bytes = Uint8List(4 + writer.bytes().length)
        ..setAll(4, writer.bytes());
      expect(() => unpackCodes(bytes), throwsA(isA<FormatException>()));
    });

    test('rejects streams that end before STOP_CODE', () {
      final writer = BitWriter()
        ..write(clearCode, 9)
        ..write(65, 9)
        ..flush();
      final bytes = Uint8List(4 + writer.bytes().length)
        ..setAll(4, writer.bytes());
      expect(() => unpackCodes(bytes), throwsA(isA<FormatException>()));
    });

    test('rejects non-zero padding bits after STOP_CODE', () {
      final packed = packCodes(const <int>[clearCode, stopCode], 0);
      packed[packed.length - 1] |= 0x04;
      expect(() => unpackCodes(packed), throwsA(isA<FormatException>()));
    });

    test('rejects trailing bytes after STOP_CODE', () {
      final packed = packCodes(const <int>[clearCode, stopCode], 0);
      final tampered = Uint8List.fromList(<int>[...packed, 0]);
      expect(() => unpackCodes(tampered), throwsA(isA<FormatException>()));
    });

    test('rejects non-positive decompression limits', () {
      expect(
        () => unpackCodes(packCodes(const <int>[clearCode, stopCode], 0), 0),
        throwsRangeError,
      );
      expect(
        () => decompress(packCodes(const <int>[clearCode, stopCode], 0), 0),
        throwsRangeError,
      );
    });

    test('rejects declared output lengths above the configured cap', () {
      final packed = packCodes(const <int>[clearCode, stopCode], 5);
      expect(() => unpackCodes(packed, 4), throwsA(isA<FormatException>()));
      expect(() => decompress(packed, 4), throwsA(isA<FormatException>()));
    });

    test('packCodes validates stream shape and code widths', () {
      expect(
        () => packCodes(const <int>[65, stopCode], 1),
        throwsA(isA<FormatException>()),
      );
      expect(
        () => packCodes(const <int>[clearCode, 600, stopCode], 1),
        throwsA(isA<FormatException>()),
      );
      expect(
        () => packCodes(const <int>[clearCode, 65], 1),
        throwsA(isA<FormatException>()),
      );
      expect(
        () => packCodes(const <int>[clearCode, -1, stopCode], 1),
        throwsA(isA<FormatException>()),
      );
      expect(
        () => packCodes(const <int>[clearCode, stopCode], -1),
        throwsRangeError,
      );
    });
  });

  group('compress and decompress', () {
    for (final sample in <String>[
      '',
      'A',
      'AB',
      'ABABAB',
      'AAAAAAA',
      'AABABC',
      'hello world',
      'the quick brown fox',
      'ababababab',
      'aaaaaaaaaa',
    ]) {
      test('ascii round-trip: $sample', () {
        expect(_roundTripString(sample), sample);
      });
    }

    test('full byte range round-trips', () {
      final data = Uint8List.fromList(
        List<int>.generate(256, (index) => index),
      );
      expect(decompress(compress(data)), data);
    });

    test('all zeros round-trip', () {
      expect(decompress(compress(Uint8List(100))), Uint8List(100));
    });

    test('all 0xFF bytes round-trip', () {
      final data = Uint8List.fromList(List<int>.filled(100, 0xff));
      expect(decompress(compress(data)), data);
    });

    test('binary repeating data round-trips', () {
      final data = Uint8List.fromList(
        List<int>.generate(512, (index) => index % 256),
      );
      expect(decompress(compress(data)), data);
    });

    test('long repetitive text compresses', () {
      final data = _enc('ABCABC' * 100);
      expect(compress(data).length, lessThan(data.length));
    });

    test('all-same data compresses', () {
      final data = Uint8List.fromList(List<int>.filled(10000, 0x42));
      expect(compress(data).length, lessThan(data.length));
    });

    test('compress is deterministic', () {
      final data = _enc('hello world test');
      expect(compress(data), compress(data));
    });

    test('code-size growth past 9 bits still round-trips', () {
      final data = Uint8List.fromList(
        List<int>.generate(1024, (index) => index % 256),
      );
      expect(decompress(compress(data)), data);
    });
  });
}

Uint8List _enc(String value) => Uint8List.fromList(utf8.encode(value));

String _roundTripString(String value) =>
    utf8.decode(decompress(compress(_enc(value))));
