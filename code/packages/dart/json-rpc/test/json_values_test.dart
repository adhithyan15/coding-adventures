/// Unit tests for the shared JSON-value machinery (`src/json_values.dart`,
/// imported directly since the barrel hides it): serde-shaped deep
/// equality and hashing, the out-of-range integer scan, and lone
/// surrogate rejection.
library;

import 'package:coding_adventures_json_rpc/src/json_values.dart';
import 'package:test/test.dart';

void main() {
  group('jsonValuesEqual', () {
    test('primitives compare by value', () {
      expect(jsonValuesEqual(null, null), isTrue);
      expect(jsonValuesEqual(true, true), isTrue);
      expect(jsonValuesEqual(true, false), isFalse);
      expect(jsonValuesEqual('a', 'a'), isTrue);
      expect(jsonValuesEqual('a', 'b'), isFalse);
      expect(jsonValuesEqual(1, 1), isTrue);
      expect(jsonValuesEqual(1.5, 1.5), isTrue);
    });

    test('ints never equal doubles', () {
      expect(jsonValuesEqual(1, 1.0), isFalse);
      expect(jsonValuesEqual(0, 0.0), isFalse);
      expect(jsonValuesEqual(1, '1'), isFalse);
      expect(jsonValuesEqual(1, true), isFalse);
      expect(jsonValuesEqual(null, 0), isFalse);
    });

    test('maps compare order-independently and deeply', () {
      expect(
          jsonValuesEqual({
            'a': 1,
            'b': [2]
          }, {
            'b': [2],
            'a': 1
          }),
          isTrue);
      expect(jsonValuesEqual({'a': 1}, {'a': 1, 'b': 2}), isFalse);
      expect(jsonValuesEqual({'a': 1}, {'a': 2}), isFalse);
      expect(jsonValuesEqual({'a': 1}, {'b': 1}), isFalse);
      expect(
          jsonValuesEqual({
            'a': {
              'b': [
                1,
                {'c': null}
              ]
            }
          }, {
            'a': {
              'b': [
                1,
                {'c': null}
              ]
            }
          }),
          isTrue);
      expect(jsonValuesEqual({'a': 1}, [1]), isFalse);
    });

    test('lists compare in order', () {
      expect(jsonValuesEqual([1, 'a', null], [1, 'a', null]), isTrue);
      expect(jsonValuesEqual([1, 2], [2, 1]), isFalse);
      expect(jsonValuesEqual([1], [1, 2]), isFalse);
      expect(
          jsonValuesEqual([
            [1]
          ], [
            [1]
          ]),
          isTrue);
      expect(
          jsonValuesEqual([
            [1]
          ], [
            [2]
          ]),
          isFalse);
    });
  });

  group('jsonValueHash', () {
    test('equal values hash equally', () {
      expect(
          jsonValueHash({
            'a': 1,
            'b': [2]
          }),
          jsonValueHash({
            'b': [2],
            'a': 1
          }));
      expect(jsonValueHash([1, 'a']), jsonValueHash([1, 'a']));
      expect(jsonValueHash(1), jsonValueHash(1));
      expect(jsonValueHash(null), jsonValueHash(null));
    });

    test('int and double hash apart', () {
      expect(jsonValueHash(1), isNot(jsonValueHash(1.0)));
    });
  });

  group('checkJsonIntegers', () {
    test('ordinary numbers pass', () {
      expect(() => checkJsonIntegers('{"a":[1,-2,0,007,1.5,-0.25,1e3,2E-2]}'),
          returnsNormally);
    });

    test('int64 extremes pass', () {
      expect(() => checkJsonIntegers('[9223372036854775807]'), returnsNormally);
      expect(
          () => checkJsonIntegers('[-9223372036854775808]'), returnsNormally);
      expect(() => checkJsonIntegers('[0007]'), returnsNormally);
    });

    test('out-of-range integers throw', () {
      for (final literal in [
        '9223372036854775808',
        '-9223372036854775809',
        '18446744073709551615',
        '99999999999999999999999',
        '100000000000000000000000',
      ]) {
        expect(
            () => checkJsonIntegers('{"id":$literal}'), throwsFormatException,
            reason: literal);
      }
    });

    test('digits inside strings never match', () {
      expect(
          () => checkJsonIntegers(
              '{"id":"99999999999999999999999","e":"a\\"99999999999999999999999"}'),
          returnsNormally);
    });

    test('exponents and fractions skip the range check', () {
      expect(
          () => checkJsonIntegers('[1e999,2E5,3.0,-4.5e-2]'), returnsNormally);
      // Every double shape skips wholly: bare, signed, and uppercase
      // exponents, with and without fractions.
      expect(() => checkJsonIntegers('[1e5,1e+5,1E-5,1.5e-3,2E5]'),
          returnsNormally);
    });

    test('unterminated strings scan to the end', () {
      expect(() => checkJsonIntegers('{"a":"99999999999999999999999'),
          returnsNormally);
    });
  });

  group('checkNoLoneSurrogates', () {
    test('clean values pass', () {
      expect(() => checkNoLoneSurrogates(null), returnsNormally);
      expect(() => checkNoLoneSurrogates(1), returnsNormally);
      expect(() => checkNoLoneSurrogates('plain é 𝄞'), returnsNormally);
      expect(
          () => checkNoLoneSurrogates({
                'a': [
                  'x',
                  1,
                  null,
                  {'b': 'ok'}
                ]
              }),
          returnsNormally);
    });

    test('lone surrogates throw anywhere', () {
      final high = String.fromCharCodes([0xD800]);
      final low = String.fromCharCodes([0xDC00]);
      final pair = String.fromCharCodes([0xD800, 0xDC00]);
      expect(() => checkNoLoneSurrogates(high), throwsFormatException);
      expect(() => checkNoLoneSurrogates(low), throwsFormatException);
      expect(() => checkNoLoneSurrogates(pair), returnsNormally);
      expect(() => checkNoLoneSurrogates('a${high}b'), throwsFormatException);
      expect(() => checkNoLoneSurrogates([high]), throwsFormatException);
      expect(() => checkNoLoneSurrogates({'k': low}), throwsFormatException);
      expect(() => checkNoLoneSurrogates({high: 1}), throwsFormatException);
    });
  });
}
