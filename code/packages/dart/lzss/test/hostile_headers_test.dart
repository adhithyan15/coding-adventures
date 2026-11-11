// Security regression tests: hostile headers must not drive work or memory.
//
// Each case is a few bytes whose header claims an enormous count or length.
// The Dart port validates headers strictly, so hostile input must be rejected
// with a FormatException promptly (never allocate or loop from the claim).
// Also round-trips every input of length <= 2 and a seeded random sample.
import 'dart:math';
import 'dart:typed_data';

import 'package:coding_adventures_lzss/lzss.dart';
import 'package:test/test.dart';

Uint8List _b(List<int> v) => Uint8List.fromList(v);

Uint8List _u32(int v) => _b([(v >> 24) & 255, (v >> 16) & 255, (v >> 8) & 255, v & 255]);

Uint8List _cat(List<Uint8List> parts) =>
    Uint8List.fromList([for (final p in parts) ...p]);

void main() {
  group('hostile headers', () {
    test('max header only is rejected promptly', () {
      expect(() => decompress(_b(List.filled(8, 255))), throwsFormatException);
    });

    test('max length with one literal is rejected', () {
      final d = _cat([_u32(0xFFFFFFFF), _u32(1), _b([0, 120])]);
      expect(() => decompress(d), throwsFormatException);
    });

    test('block count larger than the data is rejected', () {
      final d = _cat([_u32(2), _u32(0xFFFFFFFF), _b([0, 104, 105])]);
      expect(() => decompress(d), throwsFormatException);
    });

    test('invalid match offsets are rejected', () {
      expect(() => decompress(_cat([_u32(100), _u32(1), _b([1, 0, 0, 5])])), throwsFormatException);
      expect(() => decompress(_cat([_u32(100), _u32(1), _b([1, 255, 255, 255])])), throwsFormatException);
    });
  });

  group('round trips', () {
    test('every input of length <= 2', () {
      final inputs = <Uint8List>[Uint8List(0)];
      for (var a = 0; a < 256; a++) {
        inputs.add(_b([a]));
        for (var c = 0; c < 256; c++) {
          inputs.add(_b([a, c]));
        }
      }
      for (final d in inputs) {
        expect(decompress(compress(d)), equals(d));
      }
    });

    test('seeded random sample', () {
      final rng = Random(0x5EED);
      const alphabets = [2, 4, 16, 256];
      for (var i = 0; i < 300; i++) {
        final d = Uint8List.fromList(List.generate(
            3 + rng.nextInt(597), (_) => rng.nextInt(alphabets[i % 4])));
        expect(decompress(compress(d)), equals(d));
      }
    });
  });
}
