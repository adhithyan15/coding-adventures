import 'package:coding_adventures_caesar_cipher/coding_adventures_caesar_cipher.dart';
import 'package:test/test.dart';

void main() {
  // ==========================================================================
  // encrypt
  // ==========================================================================
  group('encrypt', () {
    test('shifts uppercase forward', () {
      expect(encrypt('HELLO', 3), equals('KHOOR'));
    });

    test('shifts lowercase forward and preserves case', () {
      expect(encrypt('hello', 3), equals('khoor'));
      expect(encrypt('Hello, World!', 3), equals('Khoor, Zruog!'));
    });

    test('leaves non-alphabetic characters unchanged', () {
      expect(encrypt('abc XYZ 123!', 0), equals('abc XYZ 123!'));
      expect(encrypt('a-b-c', 1), equals('b-c-d'));
    });

    test('shift 0 and shift 26 are the identity', () {
      expect(encrypt('The Quick Brown Fox', 0), equals('The Quick Brown Fox'));
      expect(encrypt('The Quick Brown Fox', 26), equals('The Quick Brown Fox'));
    });

    test('wraps around the end of the alphabet', () {
      expect(encrypt('XYZ', 3), equals('ABC'));
      expect(encrypt('xyz', 3), equals('abc'));
    });

    test('handles negative shifts via normalisation', () {
      expect(encrypt('ABC', -1), equals('ZAB'));
      expect(encrypt('ABC', -1), equals(encrypt('ABC', 25)));
    });

    test('handles large shifts', () {
      expect(encrypt('ABC', 29), equals(encrypt('ABC', 3)));
      expect(encrypt('ABC', 52), equals('ABC'));
    });

    test('passes non-ASCII characters through unchanged', () {
      // c→h, a→f, f→k; 'é', '🎉', 'Ω' are non-ASCII and pass through unchanged.
      expect(encrypt('café 🎉 Ω', 5), equals('hfké 🎉 Ω'));
    });
  });

  // ==========================================================================
  // decrypt
  // ==========================================================================
  group('decrypt', () {
    test('inverts encrypt', () {
      expect(decrypt('KHOOR', 3), equals('HELLO'));
    });

    test('round-trips for a range of shifts', () {
      const original = 'Attack at dawn! (meet by the OLD oak, 5pm)';
      for (var shift = -30; shift <= 30; shift++) {
        expect(decrypt(encrypt(original, shift), shift), equals(original),
            reason: 'round-trip failed for shift $shift');
      }
    });
  });

  // ==========================================================================
  // rot13
  // ==========================================================================
  group('rot13', () {
    test('matches known values', () {
      expect(rot13('Hello'), equals('Uryyb'));
      expect(rot13('123!'), equals('123!'));
    });

    test('is its own inverse', () {
      const text = 'The Quick Brown Fox jumps over 13 lazy dogs.';
      expect(rot13(rot13(text)), equals(text));
    });

    test('equals encrypt with shift 13', () {
      const text = 'Spoiler: the butler did it.';
      expect(rot13(text), equals(encrypt(text, 13)));
    });
  });

  // ==========================================================================
  // bruteForce
  // ==========================================================================
  group('bruteForce', () {
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

    test('BruteForceResult has value equality', () {
      expect(const BruteForceResult(3, 'HELLO'),
          equals(const BruteForceResult(3, 'HELLO')));
      expect(const BruteForceResult(3, 'HELLO'),
          isNot(equals(const BruteForceResult(4, 'HELLO'))));
    });
  });

  // ==========================================================================
  // frequencyAnalysis
  // ==========================================================================
  group('frequencyAnalysis', () {
    test('recovers the shift from a longer ciphertext', () {
      final r = frequencyAnalysis(
          'WKH TXLFN EURZQ IRA MXPSV RYHU WKH ODCB GRJ');
      expect(r.shift, equals(3));
      expect(r.plaintext, equals('THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG'));
    });

    test('round-trips against encrypt for real English', () {
      const plaintext =
          'To be or not to be that is the question whether tis nobler';
      final r = frequencyAnalysis(encrypt(plaintext, 7));
      expect(r.shift, equals(7));
      expect(r.plaintext, equals(plaintext));
    });

    test('falls back to shift 1 when there is no letter signal', () {
      final r = frequencyAnalysis('12345 !!! ???');
      expect(r.shift, equals(1));
      expect(r.plaintext, equals('12345 !!! ???'));
    });
  });

  // ==========================================================================
  // englishFrequencies
  // ==========================================================================
  group('englishFrequencies', () {
    test('has 26 entries that sum to approximately 1.0', () {
      expect(englishFrequencies.length, equals(26));
      final sum = englishFrequencies.fold<double>(0, (a, b) => a + b);
      expect(sum, closeTo(1.0, 0.01));
    });

    test('E is the most common and Z the least', () {
      final maxFreq = englishFrequencies.reduce((a, b) => a > b ? a : b);
      final minFreq = englishFrequencies.reduce((a, b) => a < b ? a : b);
      expect(englishFrequencies[4], equals(maxFreq)); // E
      expect(englishFrequencies[25], equals(minFreq)); // Z
    });
  });
}
