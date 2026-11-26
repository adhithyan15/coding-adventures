// Unit tests mirroring the Rust reference suite
// (`code/packages/rust/markov-chain/src/lib.rs`, tests module) plus
// Dart-lane extras: seeded-RNG determinism, astral-plane characters, and
// scalar-order sampling checks.

import 'dart:math';

import 'package:coding_adventures_markov_chain/markov_chain.dart';
import 'package:test/test.dart';

/// Freshly-trained chain shared across tests.
///
/// sequence: [A, B, A, C, A, B, B, A]
/// Transitions:
///   A→B: 2,  A→C: 1   → A: {B: 2/3, C: 1/3}
///   B→A: 2,  B→B: 1   → B: {A: 2/3, B: 1/3}
///   C→A: 1             → C: {A: 1}
MarkovChain trainAbc() {
  final chain = MarkovChain(1);
  chain.train(['A', 'B', 'A', 'C', 'A', 'B', 'B', 'A']);
  return chain;
}

void main() {
  // Test 1: Construction — empty chain.
  test('construction creates an empty chain', () {
    final chain = MarkovChain(1);
    expect(chain.states(), isEmpty);
    expect(chain.transitionMatrix(), isEmpty);
  });

  test('order zero throws ArgumentError', () {
    expect(() => MarkovChain(0), throwsArgumentError);
  });

  // Test 2: Train a single pair.
  test('train single pair gives probability 1.0', () {
    final chain = MarkovChain(1);
    chain.train(['A', 'B']);

    final p = chain.probability('A', 'B');
    expect(p, closeTo(1.0, 1e-9));
  });

  // Test 3: Train on a longer sequence.
  test('train sequence produces expected probabilities', () {
    final chain = trainAbc();

    expect(chain.probability('A', 'B'), closeTo(2 / 3, 1e-9));
    expect(chain.probability('A', 'C'), closeTo(1 / 3, 1e-9));
    expect(chain.probability('B', 'A'), closeTo(2 / 3, 1e-9));
    expect(chain.probability('B', 'B'), closeTo(1 / 3, 1e-9));
  });

  // Test 4: Laplace smoothing.
  test('laplace smoothing spreads probability over the alphabet', () {
    // Pre-register 3 states: A, B, C. Train on [A, B] with smoothing=1.0.
    //
    // Raw counts: A→B: 1
    // After Laplace smoothing with |S|=3:
    //   total = 1 + 1*3 = 4
    //   P(A→A) = (0+1)/4 = 0.25
    //   P(A→B) = (1+1)/4 = 0.50
    //   P(A→C) = (0+1)/4 = 0.25
    final chain = MarkovChain(1, smoothing: 1.0, states: ['A', 'B', 'C']);
    chain.train(['A', 'B']);

    expect(chain.probability('A', 'C'), closeTo(0.25, 1e-9));
    expect(chain.probability('A', 'B'), closeTo(0.5, 1e-9));
  });

  // Test 5: Generate returns the right length.
  test('generate returns exactly the requested length', () {
    final chain = trainAbc();
    final seq = chain.generate('A', 10);
    expect(seq, hasLength(10));
  });

  // Test 6: Generate string length.
  test('generateString returns the requested char count with seed prefix', () {
    final corpus = 'the quick brown fox ' * 30;
    final chain = MarkovChain(1, smoothing: 1.0);
    chain.trainString(corpus);

    final result = chain.generateString('t', 50);
    expect(result.runes.length, equals(50));
    expect(result.startsWith('t'), isTrue);
  });

  // Test 7: Stationary distribution sums to 1.
  test('stationary distribution sums to one', () {
    // The ABC chain is strongly connected (ergodic), so power
    // iteration should converge.
    final chain = trainAbc();
    final dist = chain.stationaryDistribution();

    final sum = dist.values.fold<double>(0, (a, b) => a + b);
    expect(sum, closeTo(1.0, 1e-6));
    expect(dist, isNotEmpty);
  });

  // Test 8: Order-2 chain on "abcabcabc".
  test('order-2 chain regenerates abcabcabc deterministically', () {
    // With order=2, no smoothing, training on "abcabcabc":
    //   context "a\x00b" → "c" with probability 1.0
    //   context "b\x00c" → "a" with probability 1.0
    //   context "c\x00a" → "b" with probability 1.0
    //
    // generateString("ab", 9) must produce "abcabcabc" because every
    // transition is deterministic (probability 1.0 → no randomness).
    final chain = MarkovChain(2);
    chain.trainString('abcabcabc');

    expect(chain.probability('a\x00b', 'c'), closeTo(1.0, 1e-9));

    final result = chain.generateString('ab', 9);
    expect(result, equals('abcabcabc'));
  });

  // Test 9: Unknown state raises an error.
  test('nextState on unknown state throws UnknownStateError', () {
    final chain = trainAbc();
    expect(
      () => chain.nextState('UNKNOWN'),
      throwsA(
        isA<UnknownStateError>()
            .having((e) => e.state, 'state', 'UNKNOWN')
            .having((e) => e.toString(), 'message', 'Unknown state: UNKNOWN'),
      ),
    );
  });

  // Test 10: Multi-train accumulation.
  test('multiple train calls accumulate counts', () {
    final chain = MarkovChain(1);
    chain.train(['A', 'B']);
    chain.train(['A', 'B']);

    // counts: A→B = 2 → P(A→B) still 1.0.
    expect(chain.probability('A', 'B'), closeTo(1.0, 1e-9));

    // Add a third training run with a new transition A→C.
    // counts: A→B = 2, A→C = 1 → P(A→B) ≈ 2/3.
    chain.train(['A', 'C']);

    expect(chain.probability('A', 'B'), closeTo(2 / 3, 1e-9));
    expect(chain.probability('A', 'C'), closeTo(1 / 3, 1e-9));
  });

  // Extra: trainString for order-1 character chain.
  test('trainString treats each char as a state', () {
    final chain = MarkovChain(1);
    // Train on "aaab" → a→a: 2, a→b: 1
    chain.trainString('aaab');

    expect(chain.probability('a', 'a'), closeTo(2 / 3, 1e-9));
    expect(chain.probability('a', 'b'), closeTo(1 / 3, 1e-9));
  });

  // Extra: states() returns the registered alphabet.
  test('states returns the registered alphabet', () {
    final chain = MarkovChain(1);
    chain.train(['X', 'Y', 'Z', 'X']);
    final states = chain.states()..sort();
    expect(states, equals(['X', 'Y', 'Z']));
  });

  // Extra: transition_matrix() round-trips.
  test('every transition matrix row sums to one', () {
    final chain = trainAbc();
    final matrix = chain.transitionMatrix();

    for (final entry in matrix.entries) {
      final sum = entry.value.values.fold<double>(0, (a, b) => a + b);
      expect(sum, closeTo(1.0, 1e-9), reason: 'row ${entry.key}');
    }
  });

  // Extra: generateString errors on seed shorter than order.
  test('generateString with short seed throws SeedTooShortError', () {
    final chain = MarkovChain(2);
    chain.trainString('abcabc');
    expect(
      () => chain.generateString('a', 5),
      throwsA(
        isA<SeedTooShortError>()
            .having((e) => e.need, 'need', 2)
            .having((e) => e.got, 'got', 1),
      ),
    );
  });

  // --- Dart-lane extras -----------------------------------------------------

  test('train on too-short sequence throws SeedTooShortError', () {
    final chain = MarkovChain(2);
    expect(
      () => chain.train(['A', 'B']),
      throwsA(
        isA<SeedTooShortError>()
            .having((e) => e.need, 'need', 3)
            .having((e) => e.got, 'got', 2),
      ),
    );
  });

  test('stationary distribution of empty chain throws NotErgodicError', () {
    final chain = MarkovChain(1);
    expect(
        () => chain.stationaryDistribution(), throwsA(isA<NotErgodicError>()));
  });

  test('oscillating chain throws NotErgodicError', () {
    // A→B→A swap pair plus a C→A feeder state. Power iteration from
    // uniform reaches (2/3, 1/3, 0), then alternates with (1/3, 2/3, 0)
    // forever — maxDelta stays 1/3, so the iteration never converges.
    final chain = MarkovChain(1);
    chain.train(['A', 'B', 'A']);
    chain.train(['C', 'A']);
    expect(
      () => chain.stationaryDistribution(),
      throwsA(isA<NotErgodicError>()),
    );
  });

  test('same seed reproduces the same sequence', () {
    MarkovChain fresh() {
      final chain = MarkovChain(1, smoothing: 1.0, random: Random(42));
      chain.trainString('the quick brown fox jumps over the lazy dog ' * 10);
      return chain;
    }

    expect(fresh().generateString('t', 60),
        equals(fresh().generateString('t', 60)));
  });

  test('astral characters count as single states', () {
    final chain = MarkovChain(1);
    // "a\u{1F600}b\u{1F600}" → a→\u{1F600}: 1, \u{1F600}→b: 1, b→\u{1F600}: 1
    chain.trainString('a\u{1F600}b\u{1F600}');

    expect(chain.states(), contains('\u{1F600}'));
    expect(chain.probability('a', '\u{1F600}'), closeTo(1.0, 1e-9));
    expect(chain.probability('\u{1F600}', 'b'), closeTo(1.0, 1e-9));

    final result = chain.generateString('\u{1F600}', 4);
    expect(result.runes.length, equals(4));
  });

  test('sampling order matches scalar order across BMP and astral states', () {
    // U+F800 (high BMP) vs U+1F600 (astral): UTF-16 code-unit order puts
    // the astral lead surrogate first, but scalar order puts U+F800 first.
    // Seeded sampling must follow scalar order like the reference sort.
    const highBmp = '\u{F800}';
    const astral = '\u{1F600}';
    for (var seed = 0; seed < 64; seed++) {
      final chain = MarkovChain(1, random: Random(seed));
      chain.train([highBmp, astral, highBmp, astral, highBmp, highBmp, astral]);
      // Row for highBmp: {astral: 3, highBmp: 1} → sorted by scalar:
      // [highBmp (1/4), astral (3/4)].
      final first = chain.nextState(highBmp);
      final check = Random(seed).nextDouble();
      final expected = check < 1 / 4 ? highBmp : astral;
      expect(first, equals(expected), reason: 'seed $seed (r=$check)');
    }
  });

  test('generate with length zero returns an empty list', () {
    final chain = trainAbc();
    expect(chain.generate('A', 0), isEmpty);
  });

  test('generate from unknown start throws UnknownStateError', () {
    final chain = trainAbc();
    expect(() => chain.generate('ghost', 5), throwsA(isA<UnknownStateError>()));
  });

  test('probability of unknown transitions is zero', () {
    final chain = trainAbc();
    expect(chain.probability('A', 'ghost'), equals(0.0));
    expect(chain.probability('ghost', 'A'), equals(0.0));
  });
}
