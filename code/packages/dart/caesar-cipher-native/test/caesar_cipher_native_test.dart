import 'package:coding_adventures_caesar_cipher_native/coding_adventures_caesar_cipher_native.dart';
import 'package:test/test.dart';

/// These tests exercise the *Rust* implementation through dart:ffi. They assert
/// the same behaviour as the pure-Dart port, proving the two stay in lock-step.
void main() {
  group('encrypt (native)', () {
    test('shifts letters forward, preserves case', () {
      expect(encrypt('HELLO', 3), equals('KHOOR'));
      expect(encrypt('hello', 3), equals('khoor'));
      expect(encrypt('Hello, World!', 3), equals('Khoor, Zruog!'));
    });

    test('leaves non-alphabetic characters unchanged', () {
      expect(encrypt('abc XYZ 123!', 0), equals('abc XYZ 123!'));
      expect(encrypt('a-b-c', 1), equals('b-c-d'));
    });

    test('wraps around and normalises negative/large shifts', () {
      expect(encrypt('XYZ', 3), equals('ABC'));
      expect(encrypt('ABC', -1), equals('ZAB'));
      expect(encrypt('ABC', -1), equals(encrypt('ABC', 25)));
      expect(encrypt('ABC', 26), equals('ABC'));
      expect(encrypt('ABC', 29), equals(encrypt('ABC', 3)));
    });

    test('passes non-ASCII (UTF-8) through unchanged', () {
      // c→h, a→f, f→k; 'é', '🎉', 'Ω' are non-ASCII and unchanged.
      expect(encrypt('café 🎉 Ω', 5), equals('hfké 🎉 Ω'));
    });
  });

  group('decrypt (native)', () {
    test('inverts encrypt', () {
      expect(decrypt('KHOOR', 3), equals('HELLO'));
    });

    test('round-trips over a range of shifts', () {
      const original = 'Attack at dawn! (meet by the OLD oak, 5pm)';
      for (var shift = -30; shift <= 30; shift++) {
        expect(decrypt(encrypt(original, shift), shift), equals(original),
            reason: 'round-trip failed for shift $shift');
      }
    });
  });

  group('rot13 (native)', () {
    test('matches known values and is self-inverse', () {
      expect(rot13('Hello'), equals('Uryyb'));
      expect(rot13('123!'), equals('123!'));
      const text = 'The Quick Brown Fox jumps over 13 lazy dogs.';
      expect(rot13(rot13(text)), equals(text));
    });

    test('equals encrypt with shift 13', () {
      const text = 'Spoiler: the butler did it.';
      expect(rot13(text), equals(encrypt(text, 13)));
    });
  });

  group('frequencyAnalysis (native)', () {
    test('recovers the shift from a longer ciphertext', () {
      final r = frequencyAnalysis(
          'WKH TXLFN EURZQ IRA MXPSV RYHU WKH ODCB GRJ');
      expect(r.shift, equals(3));
      expect(r.plaintext,
          equals('THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG'));
    });

    test('round-trips against encrypt for real English', () {
      const plaintext =
          'To be or not to be that is the question whether tis nobler';
      final r = frequencyAnalysis(encrypt(plaintext, 7));
      expect(r.shift, equals(7));
      expect(r.plaintext, equals(plaintext));
    });

    test('falls back to shift 1 with no letter signal', () {
      final r = frequencyAnalysis('12345 !!! ???');
      expect(r.shift, equals(1));
      expect(r.plaintext, equals('12345 !!! ???'));
    });
  });

  group('bruteForce (native)', () {
    test('returns all 25 non-trivial shifts in order', () {
      final results = bruteForce('KHOOR');
      expect(results.length, equals(25));
      expect(results.first.shift, equals(1));
      expect(results.last.shift, equals(25));
    });

    test('the correct shift appears among candidates', () {
      final results = bruteForce('KHOOR');
      expect(results[2].shift, equals(3)); // index 2 → shift 3
      expect(results[2].plaintext, equals('HELLO'));
    });

    test('preserves punctuation and spaces in candidates', () {
      final results = bruteForce(encrypt('Meet at 5!', 4));
      expect(results[3].shift, equals(4));
      expect(results[3].plaintext, equals('Meet at 5!'));
    });

    test('handles ciphertext containing tabs and newlines', () {
      // Non-letters (\t, \n) pass through the cipher unchanged. Composing
      // bruteForce from native decrypt calls keeps this correct — a serialised
      // payload could not.
      const plaintext = 'line1\tcol2\nline2';
      final results = bruteForce(encrypt(plaintext, 9));
      expect(results.length, equals(25));
      expect(results[8].shift, equals(9));
      expect(results[8].plaintext, equals(plaintext));
    });
  });

  group('parity with the pure port', () {
    test('encrypt/decrypt agree on a fuzz of shifts and inputs', () {
      const samples = [
        'The quick brown fox',
        'ATTACK AT DAWN',
        'mixed Case 123 & symbols!',
        '',
      ];
      for (final s in samples) {
        for (final shift in [0, 1, 3, 13, 25, 26, -1, -13, 100]) {
          final ct = encrypt(s, shift);
          expect(decrypt(ct, shift), equals(s),
              reason: 'round-trip failed for "$s" shift $shift');
        }
      }
    });
  });
}
