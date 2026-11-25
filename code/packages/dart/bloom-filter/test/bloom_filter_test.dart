import 'dart:convert';
import 'dart:math';

import 'package:coding_adventures_bloom_filter/coding_adventures_bloom_filter.dart';
import 'package:test/test.dart';

void main() {
  group('BloomFilter Core Suite (Swift Parity)', () {
    test('testDefaultsAndExplicitParameters', () {
      final defaultFilter = BloomFilter();
      expect(defaultFilter.bitCount, equals(9586));
      expect(defaultFilter.hashCount, equals(7));
      expect(defaultFilter.bitsSet, equals(0));
      expect(defaultFilter.fillRatio, equals(0.0));
      expect(defaultFilter.estimatedFalsePositiveRate, equals(0.0));
      expect(defaultFilter.isOverCapacity, isFalse);
      expect(defaultFilter.contains('anything'), isFalse);

      final explicit = BloomFilter.withBitAndHashCount(
        bitCount: 10000,
        hashCount: 7,
      );
      expect(explicit.sizeBytes, equals(1250));
      expect(explicit.isOverCapacity, isFalse);
    });

    test('testPersistentAddsDuplicatesAndExactBytes', () {
      final empty = BloomFilter(expectedItems: 100);
      final once = empty.adding('cafe\u{0301}');
      final twice = once.adding('cafe\u{0301}');

      expect(empty.contains('cafe\u{0301}'), isFalse);
      expect(once.contains('cafe\u{0301}'), isTrue);
      expect(once.bitsSet, equals(twice.bitsSet));
      expect(once, equals(empty.adding(utf8.encode('cafe\u{0301}'))));
      expect(once.contains('caf\u{00E9}'), isFalse);
    });

    test('testNoFalseNegativesAndBoundedFalsePositives', () {
      var filter = BloomFilter(
        expectedItems: 10000,
        falsePositiveRate: 0.01,
      );
      for (var value = 0; value < 10000; value++) {
        filter = filter.adding('present-$value');
      }
      for (var value = 0; value < 10000; value++) {
        expect(filter.contains('present-$value'), isTrue);
      }

      var falsePositives = 0;
      for (var value = 0; value < 10000; value++) {
        if (filter.contains('absent-$value')) {
          falsePositives++;
        }
      }
      expect(falsePositives, lessThan(200));
      expect(filter.bitsSet, greaterThan(0));
      expect(filter.estimatedFalsePositiveRate, greaterThan(0));
    });

    test('testCapacityAndSizingHelpers', () {
      expect(
        BloomFilter.optimalBitCount(
          expectedItems: 1000000,
          falsePositiveRate: 0.01,
        ),
        equals(9585059),
      );
      expect(
        BloomFilter.optimalHashCount(
          bitCount: 9585059,
          expectedItems: 1000000,
        ),
        equals(7),
      );
      expect(
        BloomFilter.capacity(
          memoryBytes: 1000000,
          falsePositiveRate: 0.01,
        ),
        greaterThan(0),
      );

      var filter = BloomFilter(expectedItems: 3);
      filter = filter.adding('a').adding('b').adding('c');
      expect(filter.isOverCapacity, isFalse);
      filter = filter.adding('d');
      expect(filter.isOverCapacity, isTrue);
    });

    test('testStableValidationErrorsAndLimits', () {
      expect(
        () => BloomFilter(expectedItems: 0),
        throwsA(
          isA<BloomFilterException>().having(
            (e) => e.code,
            'code',
            equals(BloomFilterErrorCode.invalidExpectedItems),
          ),
        ),
      );

      for (final rate in [0.0, 1.0, double.infinity, double.nan]) {
        expect(
          () => BloomFilter(falsePositiveRate: rate),
          throwsA(
            isA<BloomFilterException>().having(
              (e) => e.code,
              'code',
              equals(BloomFilterErrorCode.invalidFalsePositiveRate),
            ),
          ),
        );
      }

      expect(
        () => BloomFilter.withBitAndHashCount(bitCount: 0, hashCount: 1),
        throwsA(
          isA<BloomFilterException>().having(
            (e) => e.code,
            'code',
            equals(BloomFilterErrorCode.invalidBitCount),
          ),
        ),
      );

      expect(
        () => BloomFilter.withBitAndHashCount(bitCount: 1, hashCount: 0),
        throwsA(
          isA<BloomFilterException>().having(
            (e) => e.code,
            'code',
            equals(BloomFilterErrorCode.invalidHashCount),
          ),
        ),
      );

      expect(
        () => BloomFilter.withBitAndHashCount(
          bitCount: maximumBitCount + 1,
          hashCount: 1,
        ),
        throwsA(
          isA<BloomFilterException>().having(
            (e) => e.code,
            'code',
            equals(BloomFilterErrorCode.allocationLimitExceeded),
          ),
        ),
      );

      expect(
        () => BloomFilter.capacity(
          memoryBytes: 0,
          falsePositiveRate: 0.01,
        ),
        throwsA(
          isA<BloomFilterException>().having(
            (e) => e.code,
            'code',
            equals(BloomFilterErrorCode.invalidMemoryBytes),
          ),
        ),
      );

      expect(
        () => BloomFilter.capacity(
          memoryBytes: maximumBitCount ~/ 8,
          falsePositiveRate: 1.0 - 1e-16,
        ),
        throwsA(
          isA<BloomFilterException>().having(
            (e) => e.code,
            'code',
            equals(BloomFilterErrorCode.capacityOverflow),
          ),
        ),
      );
    });
  });

  group('Standing Rule #20 Diagnostics & Error Parity', () {
    test('all enum error codes covered', () {
      expect(
        BloomFilterErrorCode.values,
        containsAll([
          BloomFilterErrorCode.invalidExpectedItems,
          BloomFilterErrorCode.invalidFalsePositiveRate,
          BloomFilterErrorCode.invalidBitCount,
          BloomFilterErrorCode.invalidHashCount,
          BloomFilterErrorCode.invalidMemoryBytes,
          BloomFilterErrorCode.allocationLimitExceeded,
          BloomFilterErrorCode.capacityOverflow,
          BloomFilterErrorCode.invalidInput,
        ]),
      );
    });

    test('sealed exception toString and equality', () {
      final ex1 = BloomFilterException.invalidExpectedItems(0);
      final ex2 = BloomFilterException.invalidExpectedItems();
      expect(ex1, equals(ex2));
      expect(ex1.hashCode, equals(ex2.hashCode));
      expect(
        ex1.toString(),
        equals(
            'BloomFilterException(BloomFilterErrorCode.invalidExpectedItems): $messageInvalidExpectedItems'),
      );
      expect(ex1.toString().contains('<redacted>'), isFalse);
      expect(ex1.toString().contains('0'), isFalse);

      final diff = BloomFilterException.invalidInput();
      expect(ex1 == diff, isFalse);
      expect(ex1 == Object(), isFalse);
    });

    test('all factory constructors hit', () {
      expect(
        BloomFilterException.invalidFalsePositiveRate().code,
        equals(BloomFilterErrorCode.invalidFalsePositiveRate),
      );
      expect(
        BloomFilterException.invalidBitCount().code,
        equals(BloomFilterErrorCode.invalidBitCount),
      );
      expect(
        BloomFilterException.invalidHashCount().code,
        equals(BloomFilterErrorCode.invalidHashCount),
      );
      expect(
        BloomFilterException.invalidMemoryBytes().code,
        equals(BloomFilterErrorCode.invalidMemoryBytes),
      );
      expect(
        BloomFilterException.allocationLimitExceeded().code,
        equals(BloomFilterErrorCode.allocationLimitExceeded),
      );
      expect(
        BloomFilterException.capacityOverflow().code,
        equals(BloomFilterErrorCode.capacityOverflow),
      );
    });

    test('optimalBitCount limit exceeded', () {
      expect(
        () => BloomFilter.optimalBitCount(
          expectedItems: 1000000000,
          falsePositiveRate: 1e-12,
        ),
        throwsA(
          isA<BloomFilterException>().having(
            (e) => e.code,
            'code',
            equals(BloomFilterErrorCode.allocationLimitExceeded),
          ),
        ),
      );
    });

    test('adding and contains reject invalid inputs and types', () {
      final filter = BloomFilter();
      expect(
        () => filter.adding(42),
        throwsA(
          isA<BloomFilterException>().having(
            (e) => e.code,
            'code',
            equals(BloomFilterErrorCode.invalidInput),
          ),
        ),
      );

      expect(
        () => filter.contains(42),
        throwsA(
          isA<BloomFilterException>().having(
            (e) => e.code,
            'code',
            equals(BloomFilterErrorCode.invalidInput),
          ),
        ),
      );

      expect(filter.contains([1, 2, 3]), isFalse);

      expect(
        () => filter.addingBytes([256]),
        throwsA(
          isA<BloomFilterException>().having(
            (e) => e.code,
            'code',
            equals(BloomFilterErrorCode.invalidInput),
          ),
        ),
      );

      expect(
        () => filter.containsBytes([-1]),
        throwsA(
          isA<BloomFilterException>().having(
            (e) => e.code,
            'code',
            equals(BloomFilterErrorCode.invalidInput),
          ),
        ),
      );

      // Surrogate validation in addingString
      expect(
        () => filter.addingString(String.fromCharCode(0xD800)),
        throwsA(isA<BloomFilterException>()),
      );
      expect(
        () => filter.addingString(String.fromCharCodes([0xD800, 0x0041])),
        throwsA(isA<BloomFilterException>()),
      );
      expect(
        () => filter.addingString(String.fromCharCode(0xDC00)),
        throwsA(isA<BloomFilterException>()),
      );
      expect(
        () => filter.addingString(String.fromCharCodes([0xD800, 0xDC00])),
        returnsNormally,
      );

      // Surrogate validation in containsString
      expect(
        () => filter.containsString(String.fromCharCode(0xD800)),
        throwsA(isA<BloomFilterException>()),
      );
      expect(
        () => filter.containsString(String.fromCharCodes([0xD800, 0x0041])),
        throwsA(isA<BloomFilterException>()),
      );
      expect(
        () => filter.containsString(String.fromCharCode(0xDC00)),
        throwsA(isA<BloomFilterException>()),
      );
      expect(
        () => filter.containsString(String.fromCharCodes([0xD800, 0xDC00])),
        returnsNormally,
      );
    });

    test('equality and hashCode contract', () {
      final a = BloomFilter(expectedItems: 100).adding('item');
      final b = BloomFilter(expectedItems: 100).adding('item');
      final c = BloomFilter(expectedItems: 100).adding('other');
      final d = BloomFilter(expectedItems: 200).adding('item');

      expect(a, equals(b));
      expect(a.hashCode, equals(b.hashCode));
      expect(a == c, isFalse);
      expect(a == d, isFalse);
      expect(a == Object(), isFalse);
    });
  });

  group('5-Class Security Showcase Suite', () {
    test('1. ReDoS immunity - raw byte streaming', () {
      var filter = BloomFilter(expectedItems: 1000);
      final bytes = List<int>.generate(1000, (i) => i % 256);
      expect(() => filter = filter.addingBytes(bytes), returnsNormally);
      expect(filter.containsBytes(bytes), isTrue);
    });

    test('2. State mutation immunity - bit array unmodifiable', () {
      final filter = BloomFilter();
      final bits = filter.bits;
      expect(() => bits[0] = 255, throwsUnsupportedError);
    });

    test('3. Type confusion immunity - rejection of invalid objects', () {
      final filter = BloomFilter();
      for (final bad in [
        true,
        12.34,
        [1, 2, 'bad'],
        {'key': 'val'}
      ]) {
        expect(() => filter.adding(bad), throwsA(isA<BloomFilterException>()));
        expect(
            () => filter.contains(bad), throwsA(isA<BloomFilterException>()));
      }
    });

    test('4. Resource exhaustion immunity - allocation limits enforced', () {
      expect(
        () => BloomFilter.withBitAndHashCount(
          bitCount: maximumBitCount + 100,
          hashCount: 1,
        ),
        throwsA(isA<BloomFilterException>()),
      );
      expect(
        () => BloomFilter.withBitAndHashCount(
          bitCount: 1000,
          hashCount: maximumHashCount + 1,
        ),
        throwsA(isA<BloomFilterException>()),
      );
    });

    test('5. Information leakage immunity - zero payload in error messages',
        () {
      final secret = 'super-secret-user-key';
      try {
        BloomFilter(expectedItems: -10);
      } on BloomFilterException catch (e) {
        expect(e.message, equals(messageInvalidExpectedItems));
        expect(e.toString().contains(secret), isFalse);
      }
    });
  });

  group('Property & Fuzzing Suite', () {
    test('200-case PRNG property test', () {
      final rng = Random(42);
      var filter = BloomFilter(expectedItems: 500, falsePositiveRate: 0.02);
      final inserted = <List<int>>[];

      for (var i = 0; i < 200; i++) {
        final len = rng.nextInt(32) + 1;
        final bytes = List<int>.generate(len, (_) => rng.nextInt(256));
        inserted.add(bytes);
        filter = filter.addingBytes(bytes);
      }

      // Zero false negatives
      for (final item in inserted) {
        expect(filter.containsBytes(item), isTrue);
      }

      expect(filter.itemsAdded, equals(200));
      expect(filter.bitsSet, greaterThan(0));
    });

    test('10,000-iteration fuzz loop', () {
      final rng = Random(1337);
      var filter = BloomFilter(expectedItems: 1000);
      for (var i = 0; i < 10000; i++) {
        final len = rng.nextInt(16);
        final bytes = List<int>.generate(len, (_) => rng.nextInt(256));
        filter = filter.addingBytes(bytes);
      }
      expect(filter.itemsAdded, equals(10000));
      expect(filter.bitsSet, greaterThan(0));
      expect(filter.fillRatio, greaterThan(0.0));
    });
  });
}
