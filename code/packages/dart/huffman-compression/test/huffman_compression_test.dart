import 'dart:typed_data';

import 'package:coding_adventures_huffman_compression/huffman_compression.dart';
import 'package:test/test.dart';

Uint8List _ascii(String value) => Uint8List.fromList(value.codeUnits);

String _decodeAscii(Uint8List bytes) => String.fromCharCodes(bytes);

int _readUint32BE(Uint8List data, int offset) =>
    ByteData.sublistView(data).getUint32(offset, Endian.big);

void main() {
  group('round-trip compression', () {
    test('round-trips AAABBC', () {
      final original = _ascii('AAABBC');
      expect(_decodeAscii(decompress(compress(original))), 'AAABBC');
    });

    test('round-trips hello world', () {
      final original = _ascii('hello world');
      expect(_decodeAscii(decompress(compress(original))), 'hello world');
    });

    test('round-trips repeated text', () {
      final original = _ascii('aaaaabbbbbcccccdddddeeeeefffffggggg');
      expect(
        _decodeAscii(decompress(compress(original))),
        'aaaaabbbbbcccccdddddeeeeefffffggggg',
      );
    });

    test('round-trips unique characters', () {
      final original = _ascii('abcdefghijklmnopqrstuvwxyz');
      expect(
        _decodeAscii(decompress(compress(original))),
        'abcdefghijklmnopqrstuvwxyz',
      );
    });

    test('round-trips all 256 byte values', () {
      final original = Uint8List.fromList(List<int>.generate(256, (i) => i));
      expect(decompress(compress(original)), original);
    });

    test('round-trips a single repeated byte', () {
      final original = Uint8List.fromList(<int>[0, 0, 0, 0, 0]);
      expect(decompress(compress(original)), original);
    });
  });

  group('wire format for AAABBC', () {
    final compressed = compress(_ascii('AAABBC'));

    test('stores the expected header', () {
      expect(_readUint32BE(compressed, 0), 6);
      expect(_readUint32BE(compressed, 4), 3);
    });

    test('stores the sorted code-length table', () {
      expect(compressed[8], 65);
      expect(compressed[9], 1);
      expect(compressed[10], 66);
      expect(compressed[11], 2);
      expect(compressed[12], 67);
      expect(compressed[13], 2);
    });

    test('packs the bit stream LSB-first', () {
      expect(compressed[14], 0xa8);
      expect(compressed[15], 0x01);
    });
  });

  group('edge cases', () {
    test('empty input compresses to the 8-byte header', () {
      final compressed = compress(Uint8List(0));
      expect(compressed.length, 8);
      expect(_readUint32BE(compressed, 0), 0);
      expect(_readUint32BE(compressed, 4), 0);
      expect(decompress(compressed), Uint8List(0));
      expect(decompress(Uint8List(0)), Uint8List(0));
    });

    test('single distinct symbol uses a one-bit canonical code', () {
      final compressed = compress(_ascii('AAAA'));
      expect(_readUint32BE(compressed, 0), 4);
      expect(_readUint32BE(compressed, 4), 1);
      expect(compressed[8], 65);
      expect(compressed[9], 1);
      expect(_decodeAscii(decompress(compressed)), 'AAAA');
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

    test('rejects contradictory empty headers', () {
      final bad = Uint8List(8);
      ByteData.sublistView(bad).setUint32(4, 1, Endian.big);
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('empty output cannot declare symbols'),
          ),
        ),
      );
    });

    test('rejects symbol count zero for non-empty output', () {
      final bad = Uint8List(8);
      ByteData.sublistView(bad).setUint32(0, 1, Endian.big);
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('requires at least one symbol'),
          ),
        ),
      );
    });

    test('rejects symbol counts above the byte alphabet', () {
      final bad = Uint8List(8);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint32(4, 257, Endian.big);
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('exceeds the byte alphabet'),
          ),
        ),
      );
    });

    test('rejects truncated code-length tables', () {
      final bad = Uint8List(9);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint32(4, 1, Endian.big);
      bad[8] = 65;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('code-length table is truncated'),
          ),
        ),
      );
    });

    test('rejects code length 0', () {
      final bad = Uint8List(10);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint32(4, 1, Endian.big);
      bad[8] = 65;
      bad[9] = 0;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('invalid code length 0'),
          ),
        ),
      );
    });

    test('rejects code lengths above the CMP04 limit', () {
      final bad = Uint8List(10);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint32(4, 1, Endian.big);
      bad[8] = 65;
      bad[9] = 17;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('exceeds 16'),
          ),
        ),
      );
    });

    test('rejects duplicate symbols', () {
      final bad = Uint8List(14);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 2, Endian.big);
      header.setUint32(4, 2, Endian.big);
      bad[8] = 65;
      bad[9] = 1;
      bad[10] = 65;
      bad[11] = 1;
      bad[12] = 0;
      bad[13] = 0;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('duplicate symbol 65'),
          ),
        ),
      );
    });

    test('rejects unsorted code-length tables', () {
      final bad = Uint8List(14);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 2, Endian.big);
      header.setUint32(4, 2, Endian.big);
      bad[8] = 66;
      bad[9] = 2;
      bad[10] = 65;
      bad[11] = 1;
      bad[12] = 0;
      bad[13] = 0;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('must be sorted by (length, symbol)'),
          ),
        ),
      );
    });

    test('rejects incomplete canonical code spaces', () {
      final bad = Uint8List(12);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint32(4, 2, Endian.big);
      bad[8] = 65;
      bad[9] = 1;
      bad[10] = 66;
      bad[11] = 3;
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

    test('rejects truncated bit streams', () {
      final bad = Uint8List(10);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 10, Endian.big);
      header.setUint32(4, 1, Endian.big);
      bad[8] = 65;
      bad[9] = 1;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('bit stream exhausted'),
          ),
        ),
      );
    });

    test('rejects invalid prefixes that never match a code', () {
      final bad = Uint8List(11);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint32(4, 1, Endian.big);
      bad[8] = 65;
      bad[9] = 1;
      bad[10] = 0x01;
      expect(
        () => decompress(bad),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('matches no canonical code'),
          ),
        ),
      );
    });

    test('rejects non-zero padding bits after the final symbol', () {
      final bad = Uint8List(15);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint32(4, 1, Endian.big);
      bad[8] = 65;
      bad[9] = 1;
      bad[14] = 0x02;
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

    test('enforces the decompressed-size cap', () {
      final bad = Uint8List(10);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 5, Endian.big);
      header.setUint32(4, 1, Endian.big);
      bad[8] = 65;
      bad[9] = 1;
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

    test('rejects non-positive decompressed-size caps', () {
      expect(
        () => decompress(Uint8List(0), 0),
        throwsA(
          isA<RangeError>().having(
            (error) => error.message,
            'message',
            contains('Must be positive'),
          ),
        ),
      );
    });

    test('rejects non-zero trailing bytes after finishing at a byte boundary',
        () {
      final bad = Uint8List(12);
      final header = ByteData.sublistView(bad);
      header.setUint32(0, 1, Endian.big);
      header.setUint32(4, 1, Endian.big);
      bad[8] = 65;
      bad[9] = 1;
      bad[10] = 0x00;
      bad[11] = 0x80;
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
  });
}
