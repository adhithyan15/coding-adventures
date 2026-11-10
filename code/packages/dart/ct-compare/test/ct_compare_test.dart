// Tests for coding_adventures_ct_compare (CT01).
//
// Part 1 replays every case of the shared CT01 corpus
// (code/specs/fixtures/ct-compare-v1/cases.json, read relative to the package
// directory, which is where `dart test` runs). Part 2 adds the checks the
// corpus only samples: every byte position, all 256x256 byte pairs through
// selection, length mismatches, all 64 single-bit flips of a u64, and input
// validation.

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_ct_compare/coding_adventures_ct_compare.dart';
import 'package:test/test.dart';

Uint8List fromHex(String hex) {
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(2 * i, 2 * i + 2), radix: 16);
  }
  return out;
}

String toHex(Uint8List bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

Uint8List bytes(List<int> values) => Uint8List.fromList(values);

// Corpus values are public test data, so mapping a bool to a bit with a
// conditional is fine here (adapters are test-only code).
int bitOf(bool b) => b ? 1 : 0;

final BigInt u64Max = (BigInt.one << 64) - BigInt.one;

BigInt u64(String hex) => BigInt.parse(hex, radix: 16);

void main() {
  final corpus = jsonDecode(
    File('../../../specs/fixtures/ct-compare-v1/cases.json').readAsStringSync(),
  ) as Map<String, dynamic>;
  final cases = (corpus['cases'] as List).cast<Map<String, dynamic>>();

  group('CT01 corpus', () {
    test('is the ct-compare-v1 profile with 21 cases', () {
      expect(corpus['profile'], 'ct-compare-v1');
      expect(cases.length, 21);
    });

    for (final c in cases) {
      test(c['id'] as String, () {
        final op = c['operation'] as String;
        final expected = c['expected'] as Map<String, dynamic>;
        switch (op) {
          case 'eq':
          case 'eq_fixed':
            final l = fromHex(c['left_hex'] as String);
            final r = fromHex(c['right_hex'] as String);
            final want = expected['equal'] as bool;
            if (op == 'eq') {
              expect(ctEq(l, r), want);
              expect(ctEqBit(l, r), bitOf(want));
            } else {
              expect(l.length, c['fixed_size_bytes']);
              expect(ctEqFixed(l, r), want);
              expect(ctEqFixedBit(l, r), bitOf(want));
            }
          case 'select_bytes':
            final l = fromHex(c['left_hex'] as String);
            final r = fromHex(c['right_hex'] as String);
            final choice = bitOf(c['choice'] as bool);
            if (expected.containsKey('error_id')) {
              expect(expected['error_id'], 'CT_SELECT_LENGTH_MISMATCH');
              expect(
                () => ctSelectBytes(l, r, choice),
                throwsA(
                  isA<CtCompareError>()
                      .having((e) => e.id, 'id', 'CT_SELECT_LENGTH_MISMATCH'),
                ),
              );
            } else {
              final out = ctSelectBytes(l, r, choice);
              expect(toHex(out), expected['output_hex']);
              // Fresh storage: mutating the result leaves both inputs alone.
              if (out.isNotEmpty) {
                out[0] ^= 0xFF;
              }
              expect(toHex(l), c['left_hex']);
              expect(toHex(r), c['right_hex']);
            }
          case 'eq_u64':
            final l = u64(c['left_u64'] as String);
            final r = u64(c['right_u64'] as String);
            final want = expected['equal'] as bool;
            expect(ctEqU64(l, r), want);
            expect(ctEqU64Bit(l, r), bitOf(want));
          default:
            fail('unknown corpus operation $op');
        }
      });
    }
  });

  group('ctEq', () {
    test('accepts equal and empty strings', () {
      expect(ctEq(Uint8List(0), Uint8List(0)), isTrue);
      expect(ctEq(bytes([1, 2, 3]), bytes([1, 2, 3])), isTrue);
      expect(ctEqBit(bytes([9]), bytes([9])), 1);
      expect(ctEqFixedBit(Uint8List(0), Uint8List(0)), 1);
    });

    test('detects a mismatch at every byte position of 64 bytes', () {
      final base = bytes([for (var i = 1; i <= 64; i++) (i * 7) % 256]);
      for (var pos = 0; pos < 64; pos++) {
        final other = Uint8List.fromList(base);
        other[pos] ^= 1;
        expect(ctEq(base, other), isFalse, reason: 'position $pos');
        expect(ctEqBit(base, other), 0);
        expect(ctEqFixed(base, other), isFalse);
      }
    });

    test('detects every single-bit difference of every byte value', () {
      for (var v = 0; v < 256; v++) {
        expect(ctEq(bytes([v]), bytes([v])), isTrue);
        for (var bit = 0; bit < 8; bit++) {
          expect(ctEq(bytes([v]), bytes([v ^ (1 << bit)])), isFalse);
        }
      }
    });

    test('treats every non-zero accumulator 1..255 as different', () {
      for (var v = 1; v < 256; v++) {
        expect(ctEqBit(bytes([0, 0]), bytes([0, v])), 0);
      }
    });

    test('treats a length mismatch as false in both directions', () {
      expect(ctEq(bytes([1]), bytes([1, 2])), isFalse);
      expect(ctEq(bytes([1, 2]), bytes([1])), isFalse);
      expect(ctEq(Uint8List(0), bytes([0])), isFalse);
      expect(ctEqBit(Uint8List(0), bytes([0])), 0);
      expect(ctEqFixed(bytes([1, 2, 3]), bytes([1, 2, 3, 4])), isFalse);
    });

    test('treats NUL and high-bit bytes as data', () {
      expect(ctEq(bytes([0x61, 0, 0x62]), bytes([0x61, 0, 0x63])), isFalse);
      expect(ctEq(bytes([0xff, 0x80]), bytes([0xff, 0x80])), isTrue);
      expect(ctEq(bytes([0xff]), bytes([0x7f])), isFalse);
    });

    test('works on views of a larger buffer', () {
      final buffer = bytes([9, 1, 2, 3, 9]);
      final view = Uint8List.sublistView(buffer, 1, 4);
      expect(ctEq(view, bytes([1, 2, 3])), isTrue);
      expect(ctEq(view, bytes([1, 2, 4])), isFalse);
    });
  });

  group('ctSelectBytes', () {
    test('picks left for 1 and right for 0', () {
      expect(ctSelectBytes(bytes([65, 66]), bytes([67, 68]), 1), [65, 66]);
      expect(ctSelectBytes(bytes([65, 66]), bytes([67, 68]), 0), [67, 68]);
    });

    test('is correct for all 256 byte values against all 256 byte values', () {
      final right = bytes([for (var i = 0; i < 256; i++) i]);
      for (var lv = 0; lv < 256; lv++) {
        final left = Uint8List(256)..fillRange(0, 256, lv);
        expect(ctSelectBytes(left, right, 1), left);
        expect(ctSelectBytes(left, right, 0), right);
      }
    });

    test('handles empty inputs', () {
      expect(ctSelectBytes(Uint8List(0), Uint8List(0), 1), isEmpty);
      expect(ctSelectBytes(Uint8List(0), Uint8List(0), 0), isEmpty);
    });

    test('throws CT_SELECT_LENGTH_MISMATCH for unequal lengths', () {
      for (final pair in [
        [
          bytes([1]),
          bytes([1, 2])
        ],
        [
          bytes([1, 2]),
          bytes([1])
        ],
        [
          Uint8List(0),
          bytes([1])
        ],
      ]) {
        expect(
          () => ctSelectBytes(pair[0], pair[1], 1),
          throwsA(
            isA<CtCompareError>()
                .having((e) => e.id, 'id', 'CT_SELECT_LENGTH_MISMATCH'),
          ),
        );
      }
    });

    test('checks lengths before the choice', () {
      expect(
        () => ctSelectBytes(bytes([1]), bytes([1, 2]), 7),
        throwsA(isA<CtCompareError>()
            .having((e) => e.id, 'id', 'CT_SELECT_LENGTH_MISMATCH')),
      );
    });

    test('rejects every choice other than 0 and 1', () {
      final bad = [
        2,
        3,
        -1,
        -2,
        255,
        256,
        65536,
        4294967296,
        4294967297,
        1 << 40,
        9007199254740991,
        -9007199254740991,
      ];
      for (final choice in bad) {
        expect(
          () => ctSelectBytes(bytes([1]), bytes([2]), choice),
          throwsA(isA<CtCompareError>()
              .having((e) => e.id, 'id', 'CT_INVALID_CHOICE')),
          reason: 'choice $choice',
        );
      }
    });

    test('composes with the bit-returning equalities', () {
      final a = bytes([1]);
      final b = bytes([2]);
      expect(ctSelectBytes(a, b, ctEqBit(bytes([7]), bytes([7]))), a);
      expect(ctSelectBytes(a, b, ctEqBit(bytes([7]), bytes([8]))), b);
      expect(
          ctSelectBytes(a, b, ctEqU64Bit(BigInt.from(5), BigInt.from(6))), b);
    });

    test('returns fresh storage that does not alias an input', () {
      final l = bytes([1, 2, 3]);
      final r = bytes([4, 5, 6]);
      final out = ctSelectBytes(l, r, 1);
      expect(identical(out, l), isFalse);
      out[0] = 99;
      expect(l, [1, 2, 3]);
      final outRight = ctSelectBytes(l, r, 0);
      expect(identical(outRight, r), isFalse);
      outRight[0] = 99;
      expect(r, [4, 5, 6]);
    });
  });

  group('ctEqU64', () {
    test('is true for equal values including 0, max and the sign bit', () {
      for (final v in [
        BigInt.zero,
        BigInt.one,
        u64Max,
        BigInt.one << 63,
        (BigInt.one << 63) - BigInt.one,
        u64('0123456789abcdef'),
      ]) {
        expect(ctEqU64(v, v), isTrue);
        expect(ctEqU64Bit(v, v), 1);
      }
    });

    test('detects each of the 64 single-bit flips from several bases', () {
      for (final base in [
        BigInt.zero,
        u64Max,
        u64('5555555555555555'),
        u64('0123456789abcdef'),
        BigInt.one << 63,
      ]) {
        for (var bit = 0; bit < 64; bit++) {
          final flipped = base ^ (BigInt.one << bit);
          expect(ctEqU64(base, flipped), isFalse, reason: 'bit $bit');
          expect(ctEqU64(flipped, base), isFalse);
          expect(ctEqU64Bit(base, flipped), 0);
        }
      }
    });

    test('distinguishes values that differ only in the high or low half', () {
      expect(ctEqU64(u64('0000000100000000'), BigInt.zero), isFalse);
      expect(ctEqU64(u64('00000000ffffffff'), BigInt.zero), isFalse);
      expect(
          ctEqU64(u64('ffffffff00000000'), u64('ffffffff00000001')), isFalse);
    });

    test('rejects values outside 0..2^64-1', () {
      for (final bad in [-BigInt.one, u64Max + BigInt.one, BigInt.one << 80]) {
        for (final call in [
          () => ctEqU64(bad, BigInt.one),
          () => ctEqU64(BigInt.one, bad),
          () => ctEqU64Bit(bad, BigInt.one),
        ]) {
          expect(
            call,
            throwsA(isA<CtCompareError>()
                .having((e) => e.id, 'id', 'CT_INVALID_U64')),
          );
        }
      }
    });
  });

  test('CtCompareError renders its id', () {
    expect(CtCompareError('X', 'y').toString(), 'CtCompareError(X): y');
  });
}
