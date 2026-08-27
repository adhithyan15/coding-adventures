import 'package:coding_adventures_vigenere_cipher/vigenere_cipher.dart';
import 'package:test/test.dart';

const longEnglishText =
    'The quick brown fox jumps over the lazy dog and then runs around the '
    'entire neighborhood looking for more adventures to embark upon while '
    'the sun slowly sets behind the distant mountains casting long shadows '
    'across the valley below where the river winds its way through ancient '
    'forests filled with towering oak trees and singing birds that herald '
    'the coming of spring with their melodious songs echoing through the '
    'canopy above where squirrels chase each other from branch to branch '
    'gathering acorns and other nuts for the long winter months ahead when '
    'the ground will be covered in a thick blanket of pristine white snow '
    'and the children will build snowmen and throw snowballs at each other '
    'laughing and playing until their parents call them inside for dinner '
    'where warm soup and fresh bread await them on the old wooden table';

void main() {
  group('encrypt', () {
    test('matches the shared parity vectors', () {
      expect(encrypt('ATTACKATDAWN', 'LEMON'), 'LXFOPVEFRNHR');
      expect(encrypt('Hello, World!', 'key'), 'Rijvs, Uyvjn!');
    });

    test('preserves ASCII case and wraps both directions', () {
      expect(encrypt('ABC', 'B'), 'BCD');
      expect(encrypt('AB', 'Z'), 'ZA');
      expect(encrypt('attackatdawn', 'LeMoN'), 'lxfopvefrnhr');
    });

    test('does not advance the key on non-ASCII letters or punctuation', () {
      expect(encrypt('A😀-B', 'BC'), 'B😀-D');
      expect(encrypt('AéB', 'BC'), 'BéD');
    });

    test('accepts a key longer than the ASCII letter content', () {
      expect(encrypt('Hi', 'ABCDEFGHIJ'), 'Hj');
      expect(encrypt('', 'KEY'), '');
    });

    test('rejects empty and non-ASCII-letter keys', () {
      for (final key in [
        '',
        'KEY1',
        'KE Y',
        'KEY\n',
        'KEY\u0000',
        'KEY\u2028',
        'KÉY',
        'KEY\u0301',
        '😀',
      ]) {
        expect(() => encrypt('payload', key), throwsArgumentError);
      }
      expect(() => encrypt('', 'bad key'), throwsArgumentError);
    });
  });

  group('decrypt', () {
    test('matches parity vectors and round-trips representative text', () {
      expect(decrypt('LXFOPVEFRNHR', 'LEMON'), 'ATTACKATDAWN');
      expect(decrypt('Rijvs, Uyvjn!', 'key'), 'Hello, World!');

      const original = 'Hello, 😀 café! 123\nMixed CASE.';
      expect(decrypt(encrypt(original, 'SeCrEt'), 'secret'), original);
    });

    test('validates keys before processing ciphertext', () {
      expect(() => decrypt('', ''), throwsArgumentError);
      expect(() => decrypt('ciphertext', 'K3Y'), throwsArgumentError);
    });
  });

  group('findKeyLength', () {
    test('returns one for insufficient alphabetic signal', () {
      expect(findKeyLength(''), 1);
      expect(findKeyLength('A'), 1);
      expect(findKeyLength('A!B'), 1);
      expect(findKeyLength('ABCD'), 1);
      expect(findKeyLength('é😀Ж'), 1);
    });

    test('recovers known key lengths from long English text', () {
      expect(findKeyLength(encrypt(longEnglishText, 'KEY')), 3);
      expect(findKeyLength(encrypt(longEnglishText, 'LEMON')), 5);
      expect(findKeyLength(encrypt(longEnglishText, 'SECRET')), 6);
    });

    test('honors small limits and rejects limits above forty', () {
      final ciphertext = encrypt(longEnglishText, 'LEMON');
      expect(findKeyLength(ciphertext, maxLength: 1), 1);
      expect(findKeyLength(ciphertext, maxLength: 3), inInclusiveRange(1, 3));
      expect(() => findKeyLength(ciphertext, maxLength: 41), throwsRangeError);
    });
  });

  group('findKey', () {
    test('recovers known keys with chi-squared analysis', () {
      expect(findKey(encrypt(longEnglishText, 'KEY'), 3), 'KEY');
      expect(findKey(encrypt(longEnglishText, 'LEMON'), 5), 'LEMON');
      expect(findKey(encrypt(longEnglishText, 'SECRET'), 6), 'SECRET');
    });

    test('handles non-positive and empty positions deterministically', () {
      expect(findKey('ABC', 0), '');
      expect(findKey('ABC', -1), '');
      expect(findKey('E', 3), 'AAA');
    });

    test('rejects lengths above forty before allocation', () {
      expect(() => findKey('ABC', 41), throwsRangeError);
    });
  });

  group('breakCipher', () {
    test('recovers the key and plaintext for long English text', () {
      for (final key in ['LEMON', 'SECRET']) {
        final result = breakCipher(encrypt(longEnglishText, key));
        expect(result, BreakResult(key: key, plaintext: longEnglishText));
        expect(
          result.hashCode,
          BreakResult(key: key, plaintext: longEnglishText).hashCode,
        );
        expect(
          result.toString(),
          'BreakResult(key: <redacted>, plaintext: <redacted>)',
        );
      }
    });

    test('returns a stable result for empty and non-ASCII-only text', () {
      expect(breakCipher(''), const BreakResult(key: 'A', plaintext: ''));
      expect(breakCipher('😀é'), const BreakResult(key: 'A', plaintext: '😀é'));
    });
  });

  test('publishes one positive English frequency per ASCII letter', () {
    expect(englishFrequencies, hasLength(26));
    expect(englishFrequencies.every((frequency) => frequency > 0), isTrue);
  });

  test('CR03 enforces scalar limits after parameter preflight', () {
    final atLimit = List.filled(8192, '😀').join();
    final overLimit = '$atLimit😀';
    expect(findKeyLength(atLimit, maxLength: 40), 1);
    expect(() => findKeyLength(overLimit), throwsArgumentError);
    expect(() => findKeyLength(overLimit, maxLength: 41), throwsRangeError);
    expect(findKey(overLimit, 0), '');
    expect(() => findKey(overLimit, 1), throwsArgumentError);
    expect(() => findKey(overLimit, 41), throwsRangeError);
  });
}
