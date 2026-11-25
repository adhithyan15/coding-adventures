/// Unit tests for `coding_adventures_brotli`: all 15 reference
/// tests mirrored case-for-case (round-trips, wire format, context
/// classification, long-distance matches, compression ratios),
/// plus lane extras (error values, corrupt-input handling, the
/// fail-closed guards).
library;

import 'dart:collection';
import 'dart:typed_data';

import 'package:coding_adventures_brotli/brotli.dart';
import 'package:test/test.dart';

void _roundtrip(List<int> data) {
  final compressed = compress(data);
  final decompressed = decompress(compressed);
  expect(decompressed, equals(data),
      reason:
          'roundtrip mismatch for ${data.sublist(0, data.length < 20 ? data.length : 20)}');
}

const _prose = 'In the beginning God created the heavens and the earth. '
    'Now the earth was formless and empty, darkness was over the surface '
    'of the deep, and the Spirit of God was hovering over the waters. '
    'And God said, Let there be light, and there was light. '
    'God saw that the light was good, and he separated the light from the darkness. '
    'God called the light day, and the darkness he called night. '
    'And there was evening, and there was morning, the first day. '
    'The quick brown fox jumps over the lazy dog. '
    'Pack my box with five dozen liquor jugs. '
    'How valiantly little Jack Rabbit would fight against great odds, '
    'lying on his stomach, kicking his feet, and screaming loudly. '
    'Peter Piper picked a peck of pickled peppers. A peck of pickled peppers '
    'Peter Piper picked. If Peter Piper picked a peck of pickled peppers, '
    'where is the peck that Peter Piper picked? '
    'She sells seashells by the seashore. '
    'How much wood would a woodchuck chuck if a woodchuck could chuck wood. '
    'A woodchuck would chuck as much wood as a woodchuck could chuck if '
    'a woodchuck could chuck wood. The rain in Spain stays mainly in the plain. '
    'I am the very model of a modern Major-General, '
    'I have information vegetable, animal, and mineral.';

/// A list reporting 2^32 elements without allocating them. Any
/// element access throws: validation must fail on the length first.
class _HugeList extends ListBase<int> {
  @override
  int get length => 0x100000000;

  @override
  set length(int _) => throw StateError('must not be resized');

  @override
  int operator [](int i) => throw StateError('must not be read');

  @override
  void operator []=(int i, int v) => throw StateError('must not be written');
}

void main() {
  group('round trips', () {
    test('empty', () {
      final compressed = compress([]);
      final decompressed = decompress(compressed);
      expect(decompressed, equals(Uint8List(0)));
    });

    test('single byte', () {
      _roundtrip([0x42]);
      _roundtrip([0x00]);
      _roundtrip([0xff]);
      _roundtrip('A'.codeUnits);
    });

    test('all 256 bytes', () {
      final data = Uint8List.fromList(List<int>.generate(256, (i) => i));
      _roundtrip(data);
    });

    test('1024 As', () {
      final data = Uint8List.fromList(List<int>.filled(1024, 0x41));
      _roundtrip(data);
      final compressed = compress(data);
      expect(compressed.length, lessThan(200),
          reason:
              '1024×\'A\' should compress well, got ${compressed.length} bytes');
    });

    test('english prose', () {
      final data = Uint8List.fromList(_prose.codeUnits);
      expect(data.length, greaterThanOrEqualTo(1024),
          reason: 'prose must be >= 1024 bytes, got ${data.length}');
      _roundtrip(data);
      final compressed = compress(data);
      final ratio = compressed.length / data.length;
      expect(ratio, lessThan(0.80),
          reason:
              'English prose should compress below 80%: ${(ratio * 100).toStringAsFixed(1)}% (${compressed.length}/${data.length} bytes)');
    });

    test('512 byte deterministic blob', () {
      var state = 0xdeadbeef;
      final data = Uint8List.fromList(List<int>.generate(512, (_) {
        state = (state * 1664525 + 1013904223) & 0xffffffff;
        return (state >> 24) & 0xff;
      }));
      _roundtrip(data);
    });

    test('context transitions', () {
      _roundtrip('abc123ABC'.codeUnits);
      expect(literalContext(null), equals(0), reason: 'start → bucket 0');
      expect(literalContext(0x20), equals(0), reason: 'space → bucket 0');
      expect(literalContext(0x21), equals(0), reason: 'punct → bucket 0');
      expect(literalContext(0x30), equals(1), reason: "digit '0' → bucket 1");
      expect(literalContext(0x39), equals(1), reason: "digit '9' → bucket 1");
      expect(literalContext(0x41), equals(2), reason: "upper 'A' → bucket 2");
      expect(literalContext(0x5a), equals(2), reason: "upper 'Z' → bucket 2");
      expect(literalContext(0x61), equals(3), reason: "lower 'a' → bucket 3");
      expect(literalContext(0x7a), equals(3), reason: "lower 'z' → bucket 3");
    });

    test('long distance match', () {
      final signature = 'XYZXYZXYZ!'.codeUnits;
      final data = <int>[...signature];
      for (var i = 0; i < 4096 + 50; i++) {
        var b = (i % 170 + 40) & 0xff;
        if (b == 88 || b == 89 || b == 90 || b == 33) {
          b = 200;
        }
        data.add(b);
      }
      data.addAll(signature);
      _roundtrip(data);
      final compressed = compress(data);
      expect(compressed.length, lessThan(data.length),
          reason:
              'long-distance data should compress: ${compressed.length} vs ${data.length}');
    });

    test('repeated pattern', () {
      final base = 'the quick brown fox '.codeUnits;
      final data = Uint8List.fromList(
          List<int>.generate(base.length * 20, (i) => base[i % base.length]));
      _roundtrip(data);
    });

    test('alternating bytes', () {
      final data = Uint8List.fromList(
          List<int>.generate(200, (i) => i % 2 == 0 ? 0x41 : 0x42));
      _roundtrip(data);
    });

    test('binary sequence', () {
      final data = Uint8List.fromList(List<int>.generate(300, (i) => i % 256));
      _roundtrip(data);
    });

    test('short strings', () {
      for (final s in [
        'A',
        'AB',
        'ABC',
        'ABCD',
        'hello',
        'hello world',
        'AABCBBABC',
      ]) {
        _roundtrip(s.codeUnits);
      }
    });

    test('various run lengths', () {
      for (final n in [1, 2, 3, 4, 5, 8, 10, 17, 32, 100, 256]) {
        _roundtrip(Uint8List.fromList(List<int>.filled(n, 0x58)));
      }
    });
  });

  group('wire format', () {
    test('header check', () {
      final data = 'hello world hello world'.codeUnits;
      final compressed = compress(data);
      final origLen = (compressed[0] << 24) |
          (compressed[1] << 16) |
          (compressed[2] << 8) |
          compressed[3];
      expect(origLen, equals(data.length), reason: 'original_length in header');
      expect(compressed[4] > 0 && compressed[4] <= 64, isTrue,
          reason: 'icc_entry_count');
      expect(compressed[5] <= 33, isTrue, reason: 'dist_entry_count');
      _roundtrip(data);
    });

    test('empty wire format exact bytes', () {
      final compressed = compress([]);
      expect(compressed.sublist(0, 4), equals([0, 0, 0, 0]));
      expect(compressed[4], equals(1));
      expect(compressed[5], equals(0));
      expect(compressed[6], equals(0));
      expect(compressed[7], equals(0));
      expect(compressed[8], equals(0));
      expect(compressed[9], equals(0));
      expect(compressed[10], equals(63));
      expect(compressed[11], equals(1));
      expect(compressed[12], equals(0));
      expect(compressed.length, equals(13));
    });
  });

  group('lane extras', () {
    test('truncations decode exactly or throw', () {
      // Decode is a deterministic function of the consumed bit
      // prefix, and success requires the full header length — so a
      // truncation either throws or reproduces the input exactly.
      final data = 'hello world, hello words'.codeUnits;
      final good = compress(data);
      for (var len = 0; len < good.length; len++) {
        try {
          expect(decompress(good.sublist(0, len)), equals(data),
              reason: 'truncation to $len');
        } on BrotliError {
          continue;
        }
      }
    });

    test('full bucket fails closed instead of corrupting', () {
      // Space-interleaved alphabet: all 256 byte values land in
      // bucket 0 as literals, which the 1-byte table count cannot
      // represent. The reference truncates the count to 0 and emits
      // undecodable output (proven on random 2KB+ inputs); the lane
      // refuses loudly instead.
      final data = Uint8List.fromList([
        for (var v = 0; v < 256; v++) ...[0x20, v]
      ]);
      expect(
        () => compress(data),
        throwsA(equals(BrotliTooManyLiteralSymbols(0))),
      );
    });

    test('over-4GiB input rejected without reading', () {
      expect(
        () => compress(_HugeList()),
        throwsA(isA<BrotliInputTooLarge>()),
      );
    });

    test('bit flips never escape the error vocabulary', () {
      // Flips can silently alter bytes (compression has no
      // integrity tag), but must never crash outside BrotliError.
      final good = compress('hello world, hello words'.codeUnits);
      for (var byte = 0; byte < good.length; byte++) {
        for (final bit in [0, 3, 7]) {
          final mutated = Uint8List.fromList(good);
          mutated[byte] ^= 1 << bit;
          try {
            decompress(mutated);
          } on BrotliError {
            continue;
          } catch (e) {
            fail('non-BrotliError on corrupt input: $e');
          }
        }
      }
    });

    test('exact error messages', () {
      expect(BrotliDataTooShort(3).toString(),
          equals('brotli: data too short (3 bytes)'));
      expect(const BrotliIccTableTruncated().toString(),
          equals('brotli: ICC table truncated'));
      expect(const BrotliDistTableTruncated().toString(),
          equals('brotli: dist table truncated'));
      expect(BrotliLitTableTruncated(2).toString(),
          equals('brotli: lit table 2 truncated'));
      expect(const BrotliBitStreamExhausted().toString(),
          equals('brotli: bit stream exhausted'));
      expect(const BrotliHuffmanExhausted().toString(),
          equals('brotli: bit stream exhausted (huffman)'));
      expect(const BrotliHuffmanOverrun().toString(),
          equals('brotli: Huffman decode overrun'));
      expect(BrotliInvalidCopyDistance(9, 4).toString(),
          equals('brotli: copy distance 9 > output length 4'));
      expect(BrotliLengthMismatch(10, 4).toString(),
          equals('brotli: length mismatch: expected 10, got 4'));
      expect(BrotliTooManyLiteralSymbols(0).toString(),
          equals('brotli: literal table 0 exceeds the 255-symbol wire limit'));
      expect(
          BrotliInputTooLarge(1 << 33).toString(),
          equals(
              'brotli: input length 8589934592 exceeds the 4 GiB wire limit'));
      expect(BrotliInvalidTableSymbol('ICC', 70).toString(),
          equals('brotli: invalid symbol 70 in ICC table'));
    });

    test('error values compare by type and payload', () {
      final errors = [
        BrotliDataTooShort(3),
        const BrotliIccTableTruncated(),
        const BrotliDistTableTruncated(),
        BrotliLitTableTruncated(2),
        const BrotliBitStreamExhausted(),
        const BrotliHuffmanExhausted(),
        const BrotliHuffmanOverrun(),
        BrotliInvalidCopyDistance(9, 4),
        BrotliLengthMismatch(10, 4),
        BrotliTooManyLiteralSymbols(0),
        BrotliInputTooLarge(9),
        BrotliInvalidTableSymbol('ICC', 70),
      ];
      for (final error in errors) {
        expect(error == error, isTrue);
        expect(error.hashCode, equals(error.hashCode));
        expect(error.toString(), isNotEmpty);
      }
      for (var i = 0; i < errors.length; i++) {
        for (var j = 0; j < errors.length; j++) {
          if (i != j) {
            expect(errors[i] == errors[j], isFalse, reason: '$i vs $j');
          }
        }
      }
      expect(BrotliDataTooShort(3), isNot(equals(BrotliDataTooShort(4))));
      expect(BrotliLengthMismatch(10, 4),
          isNot(equals(BrotliLengthMismatch(10, 5))));
      expect(BrotliInvalidTableSymbol('ICC', 70),
          isNot(equals(BrotliInvalidTableSymbol('dist', 70))));
    });
  });
}
