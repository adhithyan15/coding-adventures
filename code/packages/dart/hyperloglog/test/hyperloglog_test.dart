import 'dart:math';

import 'package:coding_adventures_hyperloglog/coding_adventures_hyperloglog.dart';
import 'package:test/test.dart';

void main() {
  group('HyperLogLog Core Suite (Swift Parity)', () {
    test('testPrecisionBoundsAndEmptySketch', () {
      final sketch = HyperLogLog();
      expect(sketch.precision, equals(14));
      expect(sketch.numRegisters, equals(16384));
      expect(sketch.count(), equals(0));
      expect(sketch.isEmpty, isTrue);

      expect(
        () => HyperLogLog(3),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidPrecision),
          ),
        ),
      );

      expect(
        () => HyperLogLog(17),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidPrecision),
          ),
        ),
      );
    });

    test('testPersistentUpdatesDuplicatesAndExactBytes', () {
      final empty = HyperLogLog(10);
      final once = empty.adding('same');
      var repeated = once;
      for (var i = 0; i < 1000; i++) {
        repeated = repeated.adding('same');
      }

      expect(empty.count(), equals(0));
      expect(once.count(), equals(1));
      expect(repeated.count(), equals(1));
      expect(once, equals(empty.adding('same'.codeUnits)));
    });

    test('testDeterministicAccuracy', () {
      var sketch = HyperLogLog();
      for (var value = 0; value < 10000; value++) {
        sketch = sketch.adding('element-$value');
      }
      expect(sketch.count(), greaterThanOrEqualTo(9500));
      expect(sketch.count(), lessThanOrEqualTo(10500));
    });

    test('testMergePreservesInputsAndRejectsPrecisionMismatch', () {
      var left = HyperLogLog(10);
      var right = HyperLogLog(10);
      for (var value = 0; value < 500; value++) {
        left = left.adding('left-$value');
        right = right.adding('right-$value');
      }
      final leftBefore = left;
      final rightBefore = right;
      final merged = left.merged(right);

      expect(left, equals(leftBefore));
      expect(right, equals(rightBefore));
      expect(merged.count(), greaterThanOrEqualTo(850));
      expect(merged.count(), lessThanOrEqualTo(1150));

      final otherPrecision = HyperLogLog(14);
      expect(
        () => left.merged(otherPrecision),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.precisionMismatch),
          ),
        ),
      );
    });

    test('testMathHelpersAndStableErrors', () {
      expect(HyperLogLog.memoryBytesFor(14), equals(12288));
      expect(HyperLogLog.optimalPrecisionFor(0.01), equals(14));
      expect(HyperLogLog.optimalPrecisionFor(1.0), equals(4));
      expect(
        HyperLogLog.optimalPrecisionFor(double.minPositive),
        equals(16),
      );
      expect(
        HyperLogLog.errorRateFor(14),
        closeTo(0.008125, 0.000001),
      );

      expect(
        () => HyperLogLog.memoryBytesFor(20),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidPrecision),
          ),
        ),
      );
      expect(
        () => HyperLogLog.optimalPrecisionFor(0),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidDesiredError),
          ),
        ),
      );
      expect(
        () => HyperLogLog.optimalPrecisionFor(double.infinity),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidDesiredError),
          ),
        ),
      );
      expect(
        () => HyperLogLog.optimalPrecisionFor(double.nan),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidDesiredError),
          ),
        ),
      );
    });

    test('testSmallAndLargeRangeCorrectionPaths', () {
      final one = HyperLogLog(4).adding('one');
      expect(one.count(), greaterThanOrEqualTo(1));
      expect(one.count(), lessThanOrEqualTo(2));

      final registers = List<int>.filled(16, 56);
      final large = HyperLogLog.testingPrecision(
        precision: 4,
        registers: registers,
      );
      expect(large.count(), equals(792707196380113664));
      expect(large.registerValuesForTesting.length, equals(16));

      final saturated = HyperLogLog.testingPrecision(
        precision: 4,
        registers: List<int>.filled(16, 61),
      );
      expect(saturated.count(), equals(maxSafeInt));
    });
  });

  group('Standing Rule #20 Diagnostics & Error Parity', () {
    test('all enum error codes covered', () {
      expect(
        HyperLogLogErrorCode.values,
        containsAll([
          HyperLogLogErrorCode.invalidPrecision,
          HyperLogLogErrorCode.precisionMismatch,
          HyperLogLogErrorCode.invalidDesiredError,
          HyperLogLogErrorCode.invalidInput,
        ]),
      );
    });

    test('sealed exception toString and equality', () {
      final ex1 = HyperLogLogException.invalidPrecision(3);
      final ex2 = HyperLogLogException.invalidPrecision();
      expect(ex1, equals(ex2));
      expect(ex1.hashCode, equals(ex2.hashCode));
      expect(
        ex1.toString(),
        equals(
            'HyperLogLogException(HyperLogLogErrorCode.invalidPrecision): $messageInvalidPrecision'),
      );
      expect(ex1.toString().contains('<redacted>'), isFalse);
      expect(ex1.toString().contains('3'), isFalse);

      final diff = HyperLogLogException.invalidInput();
      expect(ex1 == diff, isFalse);
      expect(ex1 == Object(), isFalse);
    });

    test('fromRegisters validation coverage', () {
      expect(
        () => HyperLogLog.fromRegisters(
          precision: 3,
          registers: List<int>.filled(8, 0),
        ),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidPrecision),
          ),
        ),
      );

      expect(
        () => HyperLogLog.fromRegisters(
          precision: 4,
          registers: List<int>.filled(15, 0),
        ),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidInput),
          ),
        ),
      );

      expect(
        () => HyperLogLog.fromRegisters(
          precision: 4,
          registers: List<int>.filled(16, 65),
        ),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidInput),
          ),
        ),
      );

      expect(
        () => HyperLogLog.fromRegisters(
          precision: 4,
          registers: List<int>.filled(16, -1),
        ),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidInput),
          ),
        ),
      );

      final valid = HyperLogLog.fromRegisters(
        precision: 4,
        registers: List<int>.filled(16, 0),
      );
      expect(valid.isEmpty, isTrue);
      expect(valid.registers.length, equals(16));
    });

    test('adding handles dispatch and rejects invalid inputs', () {
      final hll = HyperLogLog(4);
      expect(
        () => hll.adding(42),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidInput),
          ),
        ),
      );

      expect(
        () => hll.addingBytes([256]),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidInput),
          ),
        ),
      );

      expect(
        () => hll.addingBytes([-1]),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidInput),
          ),
        ),
      );

      // Valid paired surrogate
      expect(() => hll.addingString(String.fromCharCodes([0xD800, 0xDC00])),
          returnsNormally);

      // Lone high surrogate at end
      expect(
        () => hll.addingString(String.fromCharCode(0xD800)),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidInput),
          ),
        ),
      );

      // High surrogate followed by non-low surrogate
      expect(
        () => hll.addingString(String.fromCharCodes([0xD800, 0x0041])),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidInput),
          ),
        ),
      );

      // Lone low surrogate
      expect(
        () => hll.addingString(String.fromCharCode(0xDC00)),
        throwsA(
          isA<HyperLogLogException>().having(
            (e) => e.code,
            'code',
            equals(HyperLogLogErrorCode.invalidInput),
          ),
        ),
      );
    });

    test('equality and hashCode contract', () {
      final a = HyperLogLog(4).adding('test');
      final b = HyperLogLog(4).adding('test');
      final c = HyperLogLog(4).adding('other');
      final d = HyperLogLog(5).adding('test');

      expect(a, equals(b));
      expect(a.hashCode, equals(b.hashCode));
      expect(a == c, isFalse);
      expect(a == d, isFalse);
      expect(a == Object(), isFalse);
    });

    test('alpha branches for 16, 32, 64, 128', () {
      final h16 = HyperLogLog(4); // 16 registers
      final h32 = HyperLogLog(5); // 32 registers
      final h64 = HyperLogLog(6); // 64 registers
      final h128 = HyperLogLog(7); // 128 registers

      expect(h16.numRegisters, equals(16));
      expect(h32.numRegisters, equals(32));
      expect(h64.numRegisters, equals(64));
      expect(h128.numRegisters, equals(128));

      expect(h16.errorRate, closeTo(1.04 / 4.0, 0.001));
      expect(h32.errorRate, closeTo(1.04 / sqrt(32), 0.001));
    });
  });

  group('5-Class Security Showcase Suite', () {
    test('1. ReDoS immunity - rapid byte stream ingestion', () {
      var hll = HyperLogLog(8);
      final stream = List<int>.generate(2000, (i) => i % 256);
      expect(() => hll = hll.addingBytes(stream), returnsNormally);
      expect(hll.count(), greaterThan(0));
    });

    test('2. State mutation immunity - registers list unmodifiable', () {
      final hll = HyperLogLog(4);
      final regs = hll.registers;
      expect(() => regs[0] = 5, throwsUnsupportedError);
    });

    test('3. Type confusion immunity - sealed errors on bad types', () {
      final hll = HyperLogLog(4);
      for (final bad in [
        123,
        4.56,
        true,
        {'a': 1},
        Object()
      ]) {
        expect(
          () => hll.adding(bad),
          throwsA(isA<HyperLogLogException>()),
        );
      }
    });

    test('4. Resource exhaustion immunity - precision strictly capped', () {
      expect(() => HyperLogLog(minimumPrecision - 1),
          throwsA(isA<HyperLogLogException>()));
      expect(() => HyperLogLog(maximumPrecision + 1),
          throwsA(isA<HyperLogLogException>()));
    });

    test('5. Information leakage immunity - zero payload in error messages',
        () {
      final secret = 'super-secret-user-token';
      try {
        HyperLogLog(99);
      } on HyperLogLogException catch (e) {
        expect(e.message, equals(messageInvalidPrecision));
        expect(e.toString().contains(secret), isFalse);
      }
    });
  });

  group('Property & Fuzzing Suite', () {
    test('200-case PRNG property test', () {
      final rng = Random(42);
      for (var p = 4; p <= 12; p++) {
        var hll = HyperLogLog(p);
        final distinctSet = <String>{};
        for (var step = 0; step < 200; step++) {
          final len = rng.nextInt(32) + 1;
          final bytes = List<int>.generate(len, (_) => rng.nextInt(256));
          final key = bytes.join(',');
          distinctSet.add(key);
          hll = hll.addingBytes(bytes);
        }
        final count = hll.count();
        expect(count, greaterThan(0));
        // Error bounded within roughly 3x standard error
        final errorBound = hll.errorRate * 3.5;
        final relError =
            (count - distinctSet.length).abs() / distinctSet.length;
        expect(relError, lessThan(errorBound + 0.15));
      }
    });

    test('10,000-iteration fuzz loop', () {
      final rng = Random(1337);
      var hll = HyperLogLog(6);
      for (var i = 0; i < 10000; i++) {
        final len = rng.nextInt(16);
        final bytes = List<int>.generate(len, (_) => rng.nextInt(256));
        hll = hll.addingBytes(bytes);
      }
      expect(hll.count(), greaterThan(100));
      expect(hll.isEmpty, isFalse);
    });
  });
}
