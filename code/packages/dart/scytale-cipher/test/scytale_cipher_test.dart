import 'package:coding_adventures_scytale_cipher/scytale_cipher.dart';
import 'package:test/test.dart';

void main() {
  group('encrypt', () {
    test('matches the HELLO WORLD vector', () {
      expect(encrypt('HELLO WORLD', 3), 'HLWLEOODL R ');
    });

    test('matches evenly divided vectors', () {
      expect(encrypt('ABCDEF', 2), 'ACEBDF');
      expect(encrypt('ABCDEF', 3), 'ADBECF');
      expect(encrypt('ABCDEFGH', 4), 'AEBFCGDH');
    });

    test('pads incomplete rows with literal spaces', () {
      expect(encrypt('HELLO', 3), 'HLEOL ');
    });

    test('counts Unicode scalar values, not UTF-16 code units', () {
      expect(encrypt('A😀e\u0301B', 2), 'AeB😀\u0301 ');
      expect(encrypt('Ae\u0301B', 3), 'ABe \u0301 ');
    });

    test('returns empty text before key validation', () {
      expect(encrypt('', 1), '');
    });
  });

  group('decrypt', () {
    test('recovers reference and uneven ciphertext vectors', () {
      expect(decrypt('HLWLEOODL R ', 3), 'HELLO WORLD');
      expect(decrypt('ACEBDF', 2), 'ABCDEF');
      expect(decrypt('ABCDEFGHIJ', 4), 'ADGIBEHJCF');
    });

    test('round-trips Unicode scalar values', () {
      const original = 'A😀e\u0301B';
      expect(decrypt(encrypt(original, 2), 2), original);
      expect(decrypt('ABe \u0301 ', 3), 'Ae\u0301B');
    });

    test('preserves leading and internal whitespace', () {
      const original = '  A\tB\nC';
      expect(decrypt(encrypt(original, 3), 3), original);
      expect(decrypt(encrypt('DATA\t', 2), 2), 'DATA\t');
    });

    test('documents the trailing literal-space loss in the contract', () {
      expect(decrypt(encrypt('DATA  ', 2), 2), 'DATA');
    });

    test('returns empty text before key validation', () {
      expect(decrypt('', 1), '');
    });
  });

  group('key validation', () {
    test('rejects keys below two without echoing input', () {
      expect(() => encrypt('SECRET', 1), throwsArgumentError);
      expect(() => decrypt('SECRET', 0), throwsArgumentError);
    });

    test('rejects keys above the scalar-value length', () {
      expect(() => encrypt('A😀', 3), throwsArgumentError);
      expect(() => decrypt('A😀', 3), throwsArgumentError);
    });

    test('accepts a key equal to the scalar-value length', () {
      expect(encrypt('A😀B', 3), 'A😀B');
    });
  });

  group('bruteForce', () {
    test('finds the original and tries keys in ascending order', () {
      final ciphertext = encrypt('HELLO WORLD', 3);
      final candidates = bruteForce(ciphertext);
      expect(candidates.map((candidate) => candidate.key), [2, 3, 4, 5, 6]);
      expect(
        candidates,
        contains(const ScytaleCandidate(key: 3, text: 'HELLO WORLD')),
      );
      const expected = ScytaleCandidate(key: 3, text: 'HELLO WORLD');
      expect(expected.hashCode,
          const ScytaleCandidate(key: 3, text: 'HELLO WORLD').hashCode);
      expect(
        expected.toString(),
        'ScytaleCandidate(key: 3, text: <redacted>)',
      );
      for (final candidate in candidates) {
        expect(candidate.text, decrypt(ciphertext, candidate.key));
      }
      expect(
        bruteForce('ABCDEFGHIJ').map((candidate) => candidate.key),
        [2, 3, 4, 5],
      );
    });

    test('uses scalar length and returns no candidates below four', () {
      expect(bruteForce('A😀B'), isEmpty);
      expect(
        bruteForce('A😀BC').map((candidate) => candidate.key),
        [2],
      );
    });

    test('rejects work beyond the documented quadratic-output limit', () {
      final oversized = List.filled(maxBruteForceTextLength + 1, 'A').join();
      expect(() => bruteForce(oversized), throwsRangeError);
    });
  });
}
