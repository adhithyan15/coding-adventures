import 'package:coding_adventures_atbash_cipher/atbash_cipher.dart';
import 'package:test/test.dart';

void main() {
  group('encrypt', () {
    test('matches the classic HELLO vector', () {
      expect(encrypt('HELLO'), 'SVOOL');
    });

    test('reverses both complete ASCII alphabets', () {
      expect(
        encrypt('ABCDEFGHIJKLMNOPQRSTUVWXYZ'),
        'ZYXWVUTSRQPONMLKJIHGFEDCBA',
      );
      expect(
        encrypt('abcdefghijklmnopqrstuvwxyz'),
        'zyxwvutsrqponmlkjihgfedcba',
      );
    });

    test('preserves case, punctuation, digits, and whitespace', () {
      expect(encrypt('Hello, World! 123\n'), 'Svool, Dliow! 123\n');
    });

    test('passes non-ASCII scalar values and NUL through unchanged', () {
      expect(encrypt('café Ελληνικά 😀\u0000'), 'xzué Ελληνικά 😀\u0000');
    });

    test('handles empty input', () {
      expect(encrypt(''), '');
    });
  });

  group('decrypt', () {
    test('uses the same substitution as encrypt', () {
      expect(decrypt('Svool, Dliow!'), 'Hello, World!');
    });

    test('is an involution over representative text', () {
      const original = 'The Quick Brown Fox jumps over 13 lazy dogs. 😀';
      expect(decrypt(encrypt(original)), original);
      expect(encrypt(encrypt(original)), original);
    });

    test('maps the middle letters across the alphabet boundary', () {
      expect(decrypt('NMnm'), 'MNmn');
    });
  });
}
