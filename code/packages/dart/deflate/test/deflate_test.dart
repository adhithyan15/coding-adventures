import 'dart:typed_data';

import 'package:coding_adventures_deflate/deflate.dart';
import 'package:test/test.dart';

Uint8List _ascii(String value) => Uint8List.fromList(value.codeUnits);

void _roundTrip(Uint8List data) {
  final compressed = compress(data);
  final result = decompress(compressed);
  expect(result, data);
}

void main() {
  group('helper tables', () {
    test('maps representative lengths to the expected LL symbols', () {
      expect(lengthSymbol(3), 257);
      expect(lengthSymbol(11), 265);
      expect(lengthSymbol(19), 269);
      expect(lengthSymbol(67), 277);
      expect(lengthSymbol(255), 284);
    });

    test('maps representative distances to the expected distance codes', () {
      expect(distCode(1), 0);
      expect(distCode(5), 4);
      expect(distCode(17), 8);
      expect(distCode(257), 16);
      expect(distCode(4096), 23);
    });

    test('rejects out-of-range helper inputs', () {
      expect(() => lengthSymbol(2), throwsRangeError);
      expect(() => lengthSymbol(256), throwsRangeError);
      expect(() => distCode(0), throwsRangeError);
      expect(() => distCode(4097), throwsRangeError);
    });
  });

  group('round-trip compression', () {
    test('empty input', () {
      final compressed = compress(Uint8List(0));
      expect(compressed.length, 12);
      expect(decompress(compressed), Uint8List(0));
    });

    test('single byte cases', () {
      _roundTrip(Uint8List.fromList(<int>[0x00]));
      _roundTrip(Uint8List.fromList(<int>[0xff]));
      _roundTrip(Uint8List(20)..fillRange(0, 20, 65));
    });

    test('spec examples', () {
      final aaabbc = compress(_ascii('AAABBC'));
      expect(ByteData.sublistView(aaabbc).getUint16(6, Endian.big), 0);
      expect(decompress(aaabbc), _ascii('AAABBC'));

      final mixed = compress(_ascii('AABCBBABC'));
      final mixedHeader = ByteData.sublistView(mixed);
      expect(mixedHeader.getUint32(0, Endian.big), 9);
      expect(mixedHeader.getUint16(6, Endian.big), greaterThan(0));
      expect(decompress(mixed), _ascii('AABCBBABC'));
    });

    test('overlapping and repeated matches', () {
      _roundTrip(_ascii('AAAAAAA'));
      _roundTrip(_ascii('ABABABABABAB'));
      _roundTrip(_ascii('ABCABCABCABC'));
      _roundTrip(_ascii('hello hello hello world'));
    });

    test('all byte values and binary data', () {
      _roundTrip(Uint8List.fromList(List<int>.generate(256, (index) => index)));
      _roundTrip(
        Uint8List.fromList(List<int>.generate(1000, (index) => index % 256)),
      );
    });

    test('larger repeated text compresses significantly', () {
      final base = _ascii('ABCABC');
      final data = Uint8List(base.length * 100);
      for (var index = 0; index < 100; index += 1) {
        data.setRange(index * base.length, (index + 1) * base.length, base);
      }
      final compressed = compress(data);
      expect(compressed.length, lessThan(data.length ~/ 2));
      expect(decompress(compressed), data);
    });

    test('various match lengths survive round-trip', () {
      for (final length in <int>[3, 4, 10, 11, 13, 19, 35, 67, 131, 227, 255]) {
        final prefix = Uint8List(length)..fillRange(0, length, 65);
        final separator = Uint8List.fromList(<int>[66, 66, 66]);
        final data = Uint8List(prefix.length * 2 + separator.length)
          ..setRange(0, prefix.length, prefix)
          ..setRange(prefix.length, prefix.length + separator.length, separator)
          ..setRange(
            prefix.length + separator.length,
            prefix.length * 2 + separator.length,
            prefix,
          );
        _roundTrip(data);
      }
    });
  });

  group('decoder validation', () {
    test('rejects truncated headers', () {
      expect(
        () => decompress(Uint8List.fromList(<int>[0x00])),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('header is incomplete'),
          ),
        ),
      );
    });

    test('rejects oversized declared output', () {
      final bad = Uint8List(8);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 5, Endian.big);
      expect(
        () => decompress(bad, 4),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('exceeds the configured limit 4'),
          ),
        ),
      );
    });

    test('rejects truncated LL tables', () {
      final bad = Uint8List(10);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint16(4, 1, Endian.big);
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('LL code-length table is truncated'),
          ),
        ),
      );
    });

    test('rejects truncated distance tables', () {
      final bad = Uint8List(11);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint16(4, 1, Endian.big);
      header.setUint16(6, 1, Endian.big);
      header.setUint16(8, 65, Endian.big);
      bad[10] = 1;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('distance code-length table is truncated'),
          ),
        ),
      );
    });

    test('rejects non-canonical empty payloads with trailing data', () {
      final bad = Uint8List(13);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 0, Endian.big);
      header.setUint16(4, 1, Endian.big);
      header.setUint16(6, 0, Endian.big);
      header.setUint16(8, 256, Endian.big);
      bad[10] = 1;
      bad[11] = 0x00;
      bad[12] = 0x99;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('canonical zero-length encoding'),
          ),
        ),
      );
    });

    test('rejects invalid table symbols and lengths', () {
      final invalidLl = Uint8List(11);
      final llHeader = ByteData.sublistView(invalidLl);
      llHeader.setUint32(0, 1, Endian.big);
      llHeader.setUint16(4, 1, Endian.big);
      llHeader.setUint16(8, 400, Endian.big);
      invalidLl[10] = 1;
      expect(() => decompress(invalidLl), throwsFormatException);

      final invalidDist = Uint8List(14);
      final distHeader = ByteData.sublistView(invalidDist);
      distHeader.setUint32(0, 1, Endian.big);
      distHeader.setUint16(4, 1, Endian.big);
      distHeader.setUint16(6, 1, Endian.big);
      distHeader.setUint16(8, 65, Endian.big);
      invalidDist[10] = 1;
      distHeader.setUint16(11, 99, Endian.big);
      invalidDist[13] = 1;
      expect(() => decompress(invalidDist), throwsFormatException);

      final zeroLength = Uint8List(11);
      final zeroHeader = ByteData.sublistView(zeroLength);
      zeroHeader.setUint32(0, 1, Endian.big);
      zeroHeader.setUint16(4, 1, Endian.big);
      zeroHeader.setUint16(8, 65, Endian.big);
      zeroLength[10] = 0;
      expect(() => decompress(zeroLength), throwsFormatException);

      final longLength = Uint8List(11);
      final longHeader = ByteData.sublistView(longLength);
      longHeader.setUint32(0, 1, Endian.big);
      longHeader.setUint16(4, 1, Endian.big);
      longHeader.setUint16(8, 65, Endian.big);
      longLength[10] = 17;
      expect(() => decompress(longLength), throwsFormatException);
    });

    test('rejects unsorted and duplicate table entries', () {
      final bad = Uint8List(14);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint16(4, 2, Endian.big);
      header.setUint16(8, 66, Endian.big);
      bad[10] = 2;
      header.setUint16(11, 65, Endian.big);
      bad[13] = 1;
      expect(() => decompress(bad), throwsFormatException);

      final duplicate = Uint8List(14);
      final duplicateHeader = ByteData.sublistView(duplicate);
      duplicateHeader.setUint32(0, 1, Endian.big);
      duplicateHeader.setUint16(4, 2, Endian.big);
      duplicateHeader.setUint16(8, 65, Endian.big);
      duplicate[10] = 1;
      duplicateHeader.setUint16(11, 65, Endian.big);
      duplicate[13] = 2;
      expect(() => decompress(duplicate), throwsFormatException);
    });

    test('rejects incomplete canonical tables', () {
      final bad = Uint8List(14);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint16(4, 2, Endian.big);
      header.setUint16(8, 65, Endian.big);
      bad[10] = 1;
      header.setUint16(11, 256, Endian.big);
      bad[13] = 3;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('do not form a complete prefix tree'),
          ),
        ),
      );
    });

    test('rejects oversubscribed canonical tables', () {
      final bad = Uint8List(17);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint16(4, 3, Endian.big);
      header.setUint16(8, 65, Endian.big);
      bad[10] = 1;
      header.setUint16(11, 256, Endian.big);
      bad[13] = 1;
      header.setUint16(14, 257, Endian.big);
      bad[16] = 1;
      expect(() => decompress(bad), throwsFormatException);
    });

    test('rejects single-symbol tables with multi-bit codes', () {
      final bad = Uint8List(11);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint16(4, 1, Endian.big);
      header.setUint16(8, 65, Endian.big);
      bad[10] = 2;
      expect(() => decompress(bad), throwsFormatException);
    });

    test('rejects length symbols when the distance tree is absent', () {
      final bad = Uint8List(15);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 3, Endian.big);
      header.setUint16(4, 2, Endian.big);
      header.setUint16(8, 256, Endian.big);
      bad[10] = 1;
      header.setUint16(11, 257, Endian.big);
      bad[13] = 1;
      bad[14] = 0x01;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('requires a distance tree'),
          ),
        ),
      );
    });

    test('rejects truncated symbol streams', () {
      final bad = Uint8List(14);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint16(4, 2, Endian.big);
      header.setUint16(8, 65, Endian.big);
      bad[10] = 1;
      header.setUint16(11, 256, Endian.big);
      bad[13] = 1;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('bit stream exhausted while reading a LL Huffman symbol'),
          ),
        ),
      );
    });

    test('rejects literal output that exceeds the declared length', () {
      final bad = Uint8List(18);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint16(4, 3, Endian.big);
      header.setUint16(8, 65, Endian.big);
      bad[10] = 1;
      header.setUint16(11, 256, Endian.big);
      bad[13] = 2;
      header.setUint16(14, 257, Endian.big);
      bad[16] = 2;
      bad[17] = 0x00;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('decoded output exceeds declared length 1'),
          ),
        ),
      );
    });

    test('rejects invalid backreference offsets', () {
      final bad = Uint8List(21);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint16(4, 3, Endian.big);
      header.setUint16(6, 1, Endian.big);
      header.setUint16(8, 65, Endian.big);
      bad[10] = 1;
      header.setUint16(11, 256, Endian.big);
      bad[13] = 2;
      header.setUint16(14, 257, Endian.big);
      bad[16] = 2;
      header.setUint16(17, 0, Endian.big);
      bad[19] = 1;
      bad[20] = 0x03;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('match offset 1 is invalid'),
          ),
        ),
      );
    });

    test('rejects match output that exceeds the declared length', () {
      final bad = Uint8List(21);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint16(4, 3, Endian.big);
      header.setUint16(6, 1, Endian.big);
      header.setUint16(8, 65, Endian.big);
      bad[10] = 1;
      header.setUint16(11, 256, Endian.big);
      bad[13] = 2;
      header.setUint16(14, 257, Endian.big);
      bad[16] = 2;
      header.setUint16(17, 0, Endian.big);
      bad[19] = 1;
      bad[20] = 0x06;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('decoded output exceeds declared length 1'),
          ),
        ),
      );
    });

    test(
      'rejects early end-of-data markers that underflow the declared length',
      () {
        final bad = Uint8List(15);
        final header = ByteData.sublistView(bad);
        header.setUint32(0, 1, Endian.big);
        header.setUint16(4, 2, Endian.big);
        header.setUint16(8, 65, Endian.big);
        bad[10] = 1;
        header.setUint16(11, 256, Endian.big);
        bad[13] = 1;
        bad[14] = 0x01;
        expect(
          () => decompress(bad),
          throwsA(
            isA<FormatException>().having(
              (error) => error.message,
              'message',
              contains(
                'decoded output length 0 does not match declared length 1',
              ),
            ),
          ),
        );
      },
    );

    test('rejects truncated extra-bit payloads', () {
      final bad = Uint8List(18);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 20, Endian.big);
      header.setUint16(4, 2, Endian.big);
      header.setUint16(6, 1, Endian.big);
      header.setUint16(8, 256, Endian.big);
      bad[10] = 1;
      header.setUint16(11, 257, Endian.big);
      bad[13] = 1;
      header.setUint16(14, 23, Endian.big);
      bad[16] = 1;
      bad[17] = 0x01;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('bit stream exhausted while reading distance extra bits'),
          ),
        ),
      );
    });

    test('rejects non-zero trailing padding bits', () {
      final bad = Uint8List(15);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint16(4, 2, Endian.big);
      header.setUint16(8, 65, Endian.big);
      bad[10] = 1;
      header.setUint16(11, 256, Endian.big);
      bad[13] = 1;
      bad[14] = 0x06;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('padding bits must be zero'),
          ),
        ),
      );
    });

    test('rejects non-zero trailing bytes after ending on a byte boundary', () {
      final bad = Uint8List(21);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 3, Endian.big);
      header.setUint16(4, 4, Endian.big);
      header.setUint16(8, 65, Endian.big);
      bad[10] = 2;
      header.setUint16(11, 66, Endian.big);
      bad[13] = 2;
      header.setUint16(14, 67, Endian.big);
      bad[16] = 2;
      header.setUint16(17, 256, Endian.big);
      bad[19] = 2;
      bad[20] = 0xe4;
      final extended = Uint8List.fromList(<int>[...bad, 0x80]);
      expect(
        () => decompress(extended),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('padding bits must be zero'),
          ),
        ),
      );
    });

    test('rejects non-positive decompressed-size caps', () {
      expect(
        () => decompress(Uint8List(8), 0),
        throwsA(
          isA<RangeError>().having(
            (error) => error.message,
            'message',
            contains('Must be positive'),
          ),
        ),
      );
    });

    test('rejects unknown public length and distance lookups', () {
      expect(() => lengthBase(999), throwsRangeError);
      expect(() => lengthExtraBits(999), throwsRangeError);
      expect(() => distBase(999), throwsRangeError);
      expect(() => distExtraBits(999), throwsRangeError);
    });
  });
}
