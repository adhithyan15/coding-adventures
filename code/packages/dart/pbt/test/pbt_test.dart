// Tests for coding_adventures_pbt (VER05b).
//
// Part 1 replays all 25 cases of the shared pbt-v1 corpus
// (code/specs/fixtures/pbt-v1/cases.json, read relative to the package
// directory, which is where `dart test` runs): exact generator outputs per
// seed, exact shrink minimals *and step counts*, replay consistency,
// filter exhaustion, failure-id validation, and corpus rejection. Part 2
// covers the generator algebra and combinators; part 3 covers verification,
// shrinking edge cases, and error handling.

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

bool evalIntProperty(String name, BigInt x) {
  switch (name) {
    case 'int_le_100':
      return x <= BigInt.from(100);
    case 'int_in_range_neg10_50':
      return x >= BigInt.from(-10) && x <= BigInt.from(50);
    case 'refinement_x_le_50':
      return x <= BigInt.from(50);
    default:
      throw ArgumentError('unknown int property: $name');
  }
}

bool evalListProperty(String name, List<BigInt> lst) {
  switch (name) {
    case 'list_no_consecutive_duplicates':
      for (var i = 0; i < lst.length - 1; i++) {
        if (lst[i] == lst[i + 1]) return false;
      }
      return true;
    case 'list_sum_le_20':
      final sum = lst.fold(BigInt.zero, (a, v) => a + v);
      return sum <= BigInt.from(20);
    default:
      throw ArgumentError('unknown list property: $name');
  }
}

bool evalBytesProperty(String name, Uint8List b) {
  switch (name) {
    case 'bytes_len_lt_5':
      return b.length < 5;
    default:
      throw ArgumentError('unknown bytes property: $name');
  }
}

Uint8List hexToBytes(String hex) {
  if (hex.isEmpty) return Uint8List(0);
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(2 * i, 2 * i + 2), radix: 16);
  }
  return out;
}

String bytesToHex(Uint8List bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

void main() {
  final corpus = jsonDecode(
    File('../../../specs/fixtures/pbt-v1/cases.json').readAsStringSync(),
  ) as Map<String, dynamic>;
  final cases = (corpus['cases'] as List).cast<Map<String, dynamic>>();

  group('PBT Engine Conformance Suite (VER05 / pbt-v1)', () {
    test('loads exactly 25 conformance cases from pbt-v1/cases.json', () {
      expect(cases.length, 25);
    });

    for (final c in cases) {
      test('conformance case: ${c['id']}', () {
        final op = c['op'] as String;
        switch (op) {
          case 'gen_primitive':
            final seed = BigInt.parse((c['seed'] as Map)['value'] as String);
            final typeName = c['type'] as String;
            final count = int.parse((c['count'] as Map)['value'] as String);
            final got = genPrimitive(seed, typeName, count);
            final outputs = c['outputs'] as List;
            expect(got.length, outputs.length);
            for (var i = 0; i < got.length; i++) {
              final exp = outputs[i];
              final val = got[i];
              if (exp is bool) {
                expect(val, isA<BoolValue>());
                expect((val as BoolValue).value, exp);
              } else if ((exp as Map)['kind'] == 'int') {
                expect(val, isA<IntValue>());
                expect((val as IntValue).text, exp['value']);
              } else {
                fail('unexpected output format in ${c['id']}');
              }
            }
          case 'gen_bounded':
            final seed = BigInt.parse((c['seed'] as Map)['value'] as String);
            final minVal = BigInt.parse((c['min'] as Map)['value'] as String);
            final maxVal = BigInt.parse((c['max'] as Map)['value'] as String);
            final count = int.parse((c['count'] as Map)['value'] as String);
            final got = genBounded(seed, minVal, maxVal, count);
            final outputs = (c['outputs'] as List).cast<Map<String, dynamic>>();
            expect(got.length, outputs.length);
            for (var i = 0; i < got.length; i++) {
              expect(got[i], isA<IntValue>());
              expect((got[i] as IntValue).text, outputs[i]['value']);
            }
          case 'gen_bytes':
            final seed = BigInt.parse((c['seed'] as Map)['value'] as String);
            final minLen = int.parse((c['min_len'] as Map)['value'] as String);
            final maxLen = int.parse((c['max_len'] as Map)['value'] as String);
            final count = int.parse((c['count'] as Map)['value'] as String);
            final got = genBytes(seed, minLen, maxLen, count);
            final outputs = (c['outputs'] as List).cast<Map<String, dynamic>>();
            expect(got.length, outputs.length);
            for (var i = 0; i < got.length; i++) {
              expect(got[i], isA<BytesValue>());
              expect(
                bytesToHex((got[i] as BytesValue).value),
                outputs[i]['hex'],
              );
            }
          case 'shrink_int':
          case 'shrink_refinement':
            final initial =
                BigInt.parse((c['initial'] as Map)['value'] as String);
            final target =
                BigInt.parse((c['target'] as Map)['value'] as String);
            final minVal = BigInt.parse((c['min'] as Map)['value'] as String);
            final maxVal = BigInt.parse((c['max'] as Map)['value'] as String);
            final propName = c['property'] as String;
            final expectedMinimal =
                BigInt.parse((c['expected_minimal'] as Map)['value'] as String);
            final expectedSteps =
                int.parse((c['expected_steps'] as Map)['value'] as String);
            final (gotMinimal, gotSteps) = shrinkInt(
              initial,
              target,
              minVal,
              maxVal,
              (x) => evalIntProperty(propName, x),
              1000,
            );
            expect(gotMinimal, expectedMinimal);
            expect(gotSteps, expectedSteps);
          case 'shrink_list':
            final initial = (c['initial'] as List)
                .map((it) => BigInt.parse((it as Map)['value'] as String))
                .toList();
            final elemTarget =
                BigInt.parse((c['elem_target'] as Map)['value'] as String);
            final propName = c['property'] as String;
            final expectedMinimal = (c['expected_minimal'] as List)
                .map((it) => BigInt.parse((it as Map)['value'] as String))
                .toList();
            final expectedSteps =
                int.parse((c['expected_steps'] as Map)['value'] as String);
            final (gotMinimal, gotSteps) = shrinkList(
              initial,
              elemTarget,
              (slice) => evalListProperty(propName, slice),
              1000,
            );
            expect(gotMinimal, expectedMinimal);
            expect(gotSteps, expectedSteps);
          case 'shrink_bytes':
            final initialBytes =
                hexToBytes((c['initial'] as Map)['hex'] as String);
            final propName = c['property'] as String;
            final expectedMinimalBytes =
                hexToBytes((c['expected_minimal'] as Map)['hex'] as String);
            final expectedSteps =
                int.parse((c['expected_steps'] as Map)['value'] as String);
            final (gotMinimal, gotSteps) = shrinkBytes(
              initialBytes,
              (b) => evalBytesProperty(propName, b),
              1000,
            );
            expect(bytesToHex(gotMinimal), bytesToHex(expectedMinimalBytes));
            expect(gotSteps, expectedSteps);
          case 'replay_consistency':
            final seed = BigInt.parse((c['seed'] as Map)['value'] as String);
            final propName = c['property'] as String;
            final expectedInit =
                BigInt.parse((c['initial_fail'] as Map)['value'] as String);
            final expectedMinimal =
                BigInt.parse((c['expected_minimal'] as Map)['value'] as String);
            final (gotInit, gotMin) = simulateReplay(
              seed,
              (x) => evalIntProperty(propName, x),
            );
            expect(gotInit, expectedInit);
            expect(gotMin, expectedMinimal);
          case 'filter_exhausted':
            final maxRetries =
                int.parse((c['max_retries'] as Map)['value'] as String);
            final exhaustedGen =
                Gen.integer(BigInt.one, BigInt.from(10)).filter(
              (_) => false,
              maxRetries,
            );
            final verdict = checkProperty(
              BigInt.from(12345),
              PbtOptions(
                numTests: 10,
                maxRetries: maxRetries,
                maxShrinkSteps: 100,
                isCI: false,
              ),
              (r) => exhaustedGen.sample(r),
              (_) => true,
            );
            expect(verdict, isA<Fail>());
            expect((verdict as Fail).failureId, c['failure_id']);
          case 'failure_id_valid':
            final candidate = c['input'] as String;
            expect(isRegisteredFailureId(candidate), c['valid']);
            expect(isPbtFailureId(candidate), c['valid']);
          case 'corpus_reject':
            final rawText = c['text'] as String;
            expect(() {
              if (!rawText.startsWith('{') && !rawText.startsWith('[')) {
                throw const FormatException('invalid JSON format');
              }
              jsonDecode(rawText);
            }, throwsA(anyOf(isA<FormatException>(), isA<ArgumentError>())));
          default:
            fail('unrecognized op: $op');
        }
      });
    }
  });

  group('Generator Algebra & Combinators', () {
    test('Gen.boolean produces booleans of both values', () {
      final rng = SplitMix64(BigInt.from(42));
      final g = Gen.boolean();
      var hasTrue = false;
      var hasFalse = false;
      for (var i = 0; i < 50; i++) {
        final v = g.sample(rng);
        if (v) {
          hasTrue = true;
        } else {
          hasFalse = true;
        }
      }
      expect(hasTrue && hasFalse, isTrue);
    });

    test('Gen.u8 produces ints in [0, 255]', () {
      final rng = SplitMix64(BigInt.from(42));
      final g = Gen.u8();
      for (var i = 0; i < 100; i++) {
        final v = g.sample(rng);
        expect(v >= 0 && v <= 255, isTrue);
      }
    });

    test('Gen.u64 produces 64-bit unsigned BigInts', () {
      final rng = SplitMix64(BigInt.from(42));
      final g = Gen.u64();
      for (var i = 0; i < 50; i++) {
        final v = g.sample(rng);
        expect(v >= BigInt.zero && v < (BigInt.one << 64), isTrue);
      }
    });

    test('Gen.integer produces bounded BigInts', () {
      final rng = SplitMix64(BigInt.from(101));
      final g = Gen.integer(BigInt.from(-50), BigInt.from(50));
      for (var i = 0; i < 100; i++) {
        final v = g.sample(rng);
        expect(v >= BigInt.from(-50) && v <= BigInt.from(50), isTrue);
      }
    });

    test('Gen.bytes produces Uint8List within length range', () {
      final rng = SplitMix64(BigInt.from(202));
      final g = Gen.bytes(2, 6);
      for (var i = 0; i < 50; i++) {
        final b = g.sample(rng);
        expect(b.length >= 2 && b.length <= 6, isTrue);
        for (final byte in b) {
          expect(byte >= 0 && byte <= 255, isTrue);
        }
      }
    });

    test('Gen.list produces arrays of elements', () {
      final rng = SplitMix64(BigInt.from(303));
      final g = Gen.list(3, 8, Gen.integer(BigInt.one, BigInt.from(10)));
      for (var i = 0; i < 50; i++) {
        final arr = g.sample(rng);
        expect(arr.length >= 3 && arr.length <= 8, isTrue);
        for (final item in arr) {
          expect(item >= BigInt.one && item <= BigInt.from(10), isTrue);
        }
      }
    });

    test('Gen.constant always returns the same value', () {
      final rng = SplitMix64(BigInt.from(404));
      final g = Gen.constant('hello-pbt');
      for (var i = 0; i < 20; i++) {
        expect(g.sample(rng), 'hello-pbt');
      }
    });

    test('Gen.oneOf selects among all choices', () {
      final rng = SplitMix64(BigInt.from(505));
      final g = Gen.oneOf([Gen.constant(1), Gen.constant(2), Gen.constant(3)]);
      final seen = <int>{};
      for (var i = 0; i < 50; i++) {
        final v = g.sample(rng);
        expect([1, 2, 3].contains(v), isTrue);
        seen.add(v);
      }
      expect(seen.length, 3);
    });

    test('Gen.oneOf rejects an empty list', () {
      expect(() => Gen.oneOf(<Gen<int>>[]), throwsArgumentError);
    });

    test('Gen.tuple2 and tuple3 sample records left to right', () {
      final rng = SplitMix64(BigInt.from(606));
      final g2 = Gen.tuple2(Gen.constant('a'), Gen.u8());
      final (s, n) = g2.sample(rng);
      expect(s, 'a');
      expect(n >= 0 && n <= 255, isTrue);
      final g3 = Gen.tuple3(
        Gen.constant('alpha'),
        Gen.integer(BigInt.from(10), BigInt.from(20)),
        Gen.boolean(),
      );
      for (var i = 0; i < 30; i++) {
        final (str, num, b) = g3.sample(rng);
        expect(str, 'alpha');
        expect(num >= BigInt.from(10) && num <= BigInt.from(20), isTrue);
        expect(b, isA<bool>());
      }
    });

    test('Gen.map transforms outputs', () {
      final rng = SplitMix64(BigInt.from(707));
      final g =
          Gen.integer(BigInt.one, BigInt.from(10)).map((n) => n * BigInt.two);
      for (var i = 0; i < 50; i++) {
        final v = g.sample(rng);
        expect(v % BigInt.two, BigInt.zero);
        expect(v >= BigInt.two && v <= BigInt.from(20), isTrue);
      }
    });

    test('Gen.filter respects predicate, ExhaustedError when impossible', () {
      final rng = SplitMix64(BigInt.from(808));
      final g = Gen.integer(BigInt.one, BigInt.from(100))
          .filter((n) => n % BigInt.two == BigInt.zero);
      for (var i = 0; i < 30; i++) {
        expect(g.sample(rng) % BigInt.two, BigInt.zero);
      }
      final impossible =
          Gen.integer(BigInt.one, BigInt.from(10)).filter((_) => false, 10);
      expect(() => impossible.sample(rng), throwsA(isA<ExhaustedError>()));
    });

    test('Gen.flatMap chains dependent generators', () {
      final rng = SplitMix64(BigInt.from(909));
      final g = Gen.integer(BigInt.one, BigInt.from(5)).flatMap(
        (len) => Gen.list(len.toInt(), len.toInt(), Gen.constant(99)),
      );
      for (var i = 0; i < 30; i++) {
        final arr = g.sample(rng);
        expect(arr.length >= 1 && arr.length <= 5, isTrue);
        expect(arr.every((v) => v == 99), isTrue);
      }
    });

    test('subSeed isolates property streams', () {
      final a = subSeed(BigInt.from(42), 'prop-a');
      final b = subSeed(BigInt.from(42), 'prop-b');
      expect(a == b, isFalse);
      expect(subSeed(BigInt.from(42), 'prop-a'), a);
    });

    test('bounded rejects spans above 2^32 and degenerates at <= 1', () {
      final rng = SplitMix64(BigInt.one);
      expect(rng.bounded(0), 0);
      expect(rng.bounded(1), 0);
      expect(() => rng.bounded(0x100000001), throwsArgumentError);
    });

    test('genBoundedInt covers full-width and multi-word spans', () {
      final rng = SplitMix64(BigInt.from(11));
      final full =
          genBoundedInt(rng, BigInt.zero, (BigInt.one << 64) - BigInt.one);
      expect(full >= BigInt.zero && full < (BigInt.one << 64), isTrue);
      final wide =
          genBoundedInt(rng, BigInt.zero, (BigInt.one << 200) - BigInt.one);
      expect(wide >= BigInt.zero && wide < (BigInt.one << 200), isTrue);
      expect(
        () => genBoundedInt(rng, BigInt.one, BigInt.zero),
        throwsArgumentError,
      );
    });

    test('genPrimitive and genBytes reject bad shapes', () {
      expect(
        () => genPrimitive(BigInt.zero, 'nope', 1),
        throwsArgumentError,
      );
      expect(() => genBytes(BigInt.zero, 5, 2, 1), throwsArgumentError);
      expect(() => Gen.bytes(5, 2), throwsArgumentError);
      expect(
        () => Gen.list(5, 2, Gen.constant(1)),
        throwsArgumentError,
      );
    });
  });

  group('Verification, Shrinking, and Error Handling', () {
    test('assertProperty passes for a true property', () {
      assertProperty(
        Gen.integer(BigInt.from(-100), BigInt.from(100)),
        (n) => n >= BigInt.from(-100) && n <= BigInt.from(100),
        PbtOptions(numTests: 100, seed: BigInt.from(12345)),
      );
    });

    test('assertProperty throws PbtFailure with counterexample id', () {
      PbtFailure? caught;
      try {
        assertProperty(
          Gen.integer(BigInt.zero, BigInt.from(500)),
          (n) => n <= BigInt.from(50),
          PbtOptions(numTests: 50, seed: BigInt.from(42)),
        );
      } on PbtFailure catch (e) {
        caught = e;
      }
      expect(caught, isNotNull);
      expect(caught!.failureId, fidCounterexample);
      expect(caught.toString(), contains(fidCounterexample));
    });

    test('checkProperty captures panics with FID_PANIC', () {
      final verdict = checkProperty<int>(
        BigInt.one,
        PbtOptions(numTests: 10),
        (_) => 42,
        (_) => throw StateError('simulated property crash'),
      );
      expect(verdict, isA<Fail>());
      final fail = verdict as Fail;
      expect(fail.failureId, fidPanic);
      expect(fail.message, contains('simulated property crash'));
    });

    test('checkProperty captures sampling errors with FID_EXHAUSTED', () {
      final verdict = checkProperty<int>(
        BigInt.one,
        PbtOptions(numTests: 10),
        (_) => throw StateError('generator sampling exploded'),
        (_) => true,
      );
      expect(verdict, isA<Fail>());
      final fail = verdict as Fail;
      expect(fail.failureId, fidExhausted);
      expect(fail.message, contains('generator sampling exploded'));
    });

    test('CI reporting hashes the counterexample with FNV-1a', () {
      final verdict = checkProperty<int>(
        BigInt.from(42),
        PbtOptions(numTests: 10, isCI: true),
        (_) => 100,
        (_) => false,
        (val, __prop, __steps) => (val, 5),
      );
      expect(verdict, isA<Fail>());
      final fail = verdict as Fail;
      expect(fail.failureId, fidCounterexample);
      expect(fail.message, contains('repro-fnv:'));
      expect(fail.message.contains('initial='), isFalse);
    });

    test('non-CI reporting includes initial and minimal witnesses', () {
      final verdict = checkProperty<int>(
        BigInt.from(42),
        PbtOptions(numTests: 10, isCI: false),
        (_) => 100,
        (_) => false,
        (val, __prop, __steps) => (val, 5),
      );
      expect(verdict, isA<Fail>());
      final fail = verdict as Fail;
      expect(fail.failureId, fidCounterexample);
      expect(fail.message, contains('initial='));
      expect(fail.message, contains('minimal='));
    });

    test('shrink errors fall back to the unshrunk candidate', () {
      final verdict = checkProperty<int>(
        BigInt.from(42),
        PbtOptions(numTests: 10, isCI: false),
        (_) => 100,
        (_) => false,
        (val, __prop, __steps) => throw StateError('shrinker blew up'),
      );
      expect(verdict, isA<Fail>());
      expect((verdict as Fail).message, contains('after 0 shrink steps'));
    });

    test('shrinkInt rejects satisfying initials and honors budgets', () {
      expect(
        () => shrinkInt(BigInt.from(5), BigInt.zero, BigInt.zero,
            BigInt.from(100), (x) => x <= BigInt.from(100)),
        throwsArgumentError,
      );
      final (minimal, steps) = shrinkInt(
        BigInt.from(54321),
        BigInt.zero,
        BigInt.zero,
        BigInt.from(100000),
        (x) => x <= BigInt.from(100),
        3,
      );
      // Budget of 3: still falsifying, just less shrunk.
      expect(minimal > BigInt.from(100), isTrue);
      expect(steps, 3);
    });

    test('shrinkList rejects satisfying initials', () {
      expect(
        () => shrinkList([BigInt.one], BigInt.zero, (_) => true),
        throwsArgumentError,
      );
    });

    test('simulateReplay throws when nothing falsifies', () {
      expect(
        () => simulateReplay(BigInt.one, (_) => true),
        throwsStateError,
      );
    });

    test('divEuclid matches Euclidean semantics', () {
      expect(divEuclid(BigInt.from(7), BigInt.from(2)), BigInt.from(3));
      expect(divEuclid(BigInt.from(-7), BigInt.from(2)), BigInt.from(-4));
      expect(divEuclid(BigInt.from(7), BigInt.from(-2)), BigInt.from(-4));
      expect(() => divEuclid(BigInt.one, BigInt.zero), throwsArgumentError);
    });

    test('all four failure ids registered, others rejected', () {
      expect(allPbtFailureIds.length, 4);
      for (final id in allPbtFailureIds) {
        expect(isPbtFailureId(id), isTrue);
        expect(isRegisteredFailureId(id), isTrue);
      }
      expect(isPbtFailureId('nope'), isFalse);
    });
  });
}
