/// Property tests for `coding_adventures_markov_chain`, using the in-lane
/// `pbt` package: row-sum laws, probability/count consistency against an
/// independent sliding-window recount, multi-train accumulation, smoothing
/// laws, generation invariants, seeded determinism, stationary-distribution
/// fixed-point checks, and exact error values — all over random corpora
/// with fixed seeds.
library;

import 'dart:math';

import 'package:coding_adventures_markov_chain/markov_chain.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

// Corpus drawn from a small alphabet so contexts repeat and rows gain
// multiple entries; includes the empty string and a NUL-containing token to
// exercise key-joining edge cases.
const List<String> _alphabet = [
  'a',
  'b',
  'c',
  '',
  'x\x00y',
  'longer-token',
];

Gen<String> _token() =>
    Gen.integer(BigInt.zero, BigInt.from(_alphabet.length - 1))
        .map((i) => _alphabet[i.toInt()]);

Gen<List<String>> _corpus() => Gen.list(2, 30, _token());

Gen<int> _order() =>
    Gen.integer(BigInt.one, BigInt.from(3)).map((o) => o.toInt());

Gen<double> _smoothing() =>
    Gen.oneOf([Gen.constant(0.0), Gen.constant(0.5), Gen.constant(1.0)]);

/// Independent sliding-window recount: context key → target → count.
Map<String, Map<String, int>> _recount(List<String> seq, int order) {
  final counts = <String, Map<String, int>>{};
  if (seq.length < order + 1) return counts;
  for (var i = 0; i <= seq.length - order - 1; i++) {
    final key = seq.sublist(i, i + order).join('\x00');
    final target = seq[i + order];
    final row = counts.putIfAbsent(key, () => {});
    row[target] = (row[target] ?? 0) + 1;
  }
  return counts;
}

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('every transition row sums to one', () {
      final gen = Gen.tuple3(_order(), _smoothing(), _corpus());
      final verdict = checkProperty<(int, double, List<String>)>(
        BigInt.from(201),
        PbtOptions(numTests: 200, seed: BigInt.from(201)),
        (rng) => gen.sample(rng),
        (input) {
          final order = input.$1;
          final smoothing = input.$2;
          final corpus = input.$3;
          final chain = MarkovChain(order, smoothing: smoothing);
          try {
            chain.train(corpus);
          } on SeedTooShortError {
            return chain.transitionMatrix().isEmpty;
          }
          for (final row in chain.transitionMatrix().values) {
            final sum = row.values.fold<double>(0, (a, b) => a + b);
            if ((sum - 1.0).abs() > 1e-9) return false;
          }
          return true;
        },
      );
      _check('row-sums', verdict);
    });

    test('probabilities match an independent recount', () {
      final gen = Gen.tuple3(_order(), _smoothing(), _corpus());
      final verdict = checkProperty<(int, double, List<String>)>(
        BigInt.from(202),
        PbtOptions(numTests: 200, seed: BigInt.from(202)),
        (rng) => gen.sample(rng),
        (input) {
          final order = input.$1;
          final smoothing = input.$2;
          final corpus = input.$3;
          if (corpus.length < order + 1) return true;
          final chain = MarkovChain(order, smoothing: smoothing);
          chain.train(corpus);
          final expected = _recount(corpus, order);
          final states = chain.states();
          for (final key in expected.keys) {
            final obs = expected[key]!;
            final rawTotal = obs.values.fold<int>(0, (a, b) => a + b);
            final total = rawTotal + smoothing * states.length;
            if (smoothing > 0) {
              for (final s in states) {
                final want = ((obs[s] ?? 0) + smoothing) / total;
                if ((chain.probability(key, s) - want).abs() > 1e-12) {
                  return false;
                }
              }
            } else {
              for (final target in obs.keys) {
                final want = obs[target]! / total;
                if ((chain.probability(key, target) - want).abs() > 1e-12) {
                  return false;
                }
              }
              // Unobserved pairs stay exactly zero without smoothing.
              for (final s in states) {
                if (!obs.containsKey(s) && chain.probability(key, s) != 0.0) {
                  return false;
                }
              }
            }
          }
          return true;
        },
      );
      _check('recount', verdict);
    });

    test('multi-train accumulates both corpora', () {
      final gen = Gen.tuple3(_order(), _corpus(), _corpus());
      final verdict = checkProperty<(int, List<String>, List<String>)>(
        BigInt.from(203),
        PbtOptions(numTests: 200, seed: BigInt.from(203)),
        (rng) => gen.sample(rng),
        (input) {
          final order = input.$1;
          final first = input.$2;
          final second = input.$3;
          final chain = MarkovChain(order);
          for (final seq in [first, second]) {
            try {
              chain.train(seq);
            } on SeedTooShortError {
              // Too short to contribute any window; skip.
            }
          }
          // Merge the two independent recounts.
          final merged = _recount(first, order);
          for (final entry in _recount(second, order).entries) {
            final row = merged.putIfAbsent(entry.key, () => {});
            for (final target in entry.value.entries) {
              row[target.key] = (row[target.key] ?? 0) + target.value;
            }
          }
          for (final key in merged.keys) {
            final obs = merged[key]!;
            final total = obs.values.fold<int>(0, (a, b) => a + b);
            for (final target in obs.keys) {
              final want = obs[target]! / total;
              if ((chain.probability(key, target) - want).abs() > 1e-12) {
                return false;
              }
            }
          }
          return true;
        },
      );
      _check('accumulation', verdict);
    });

    test('smoothing covers the whole alphabet in every row', () {
      final gen = Gen.tuple3(_order(), _corpus(), _token());
      final verdict = checkProperty<(int, List<String>, String)>(
        BigInt.from(204),
        PbtOptions(numTests: 200, seed: BigInt.from(204)),
        (rng) => gen.sample(rng),
        (input) {
          final order = input.$1;
          final corpus = input.$2;
          if (corpus.length < order + 1) return true;
          final chain = MarkovChain(order, smoothing: 1.0);
          chain.train(corpus);
          final states = chain.states();
          for (final row in chain.transitionMatrix().values) {
            // Every alphabet state present with positive probability.
            for (final s in states) {
              final p = row[s];
              if (p == null || p <= 0.0) return false;
            }
            if (row.length != states.length) return false;
          }
          return true;
        },
      );
      _check('smoothing-coverage', verdict);
    });

    test('generated sequences respect length and alphabet', () {
      final gen = Gen.tuple3(_order(), _corpus(), Gen.u8());
      final verdict = checkProperty<(int, List<String>, int)>(
        BigInt.from(205),
        PbtOptions(numTests: 200, seed: BigInt.from(205)),
        (rng) => gen.sample(rng),
        (input) {
          final order = input.$1;
          final corpus = input.$2;
          final length = (input.$3 % 40) + 1;
          if (corpus.length < order + 1) return true;
          final chain = MarkovChain(order, random: Random(7));
          chain.train(corpus);
          final keys = chain.transitionMatrix().keys.toList();
          if (keys.isEmpty) return true;
          final start = keys[input.$3 % keys.length];
          List<String> seq;
          try {
            seq = chain.generate(start, length);
          } on UnknownStateError {
            // A sliding window can walk off the trained table when a
            // generated context was never observed; the error is the
            // specified behavior.
            return true;
          }
          // Reference behavior: the output starts with the seed tokens,
          // then fills up to `length` — so a seed longer than `length`
          // is returned verbatim.
          final seedTokens = start.split('\x00');
          if (seedTokens.length > length) {
            if (seq.join('\x00') != start) return false;
          } else if (seq.length != length) {
            return false;
          }
          // Output tokens are key fragments; they are corpus members
          // only when no corpus token contains the separator (otherwise
          // the join is ambiguous — same as the reference split).
          if (!corpus.any((t) => t.contains('\x00'))) {
            final alphabet = chain.states().toSet();
            for (final token in seq) {
              if (!alphabet.contains(token)) return false;
            }
          }
          return true;
        },
      );
      _check('generation-shape', verdict);
    });

    test('same seed reproduces the same output', () {
      final gen = Gen.tuple2(_corpus(), Gen.u8());
      final verdict = checkProperty<(List<String>, int)>(
        BigInt.from(206),
        PbtOptions(numTests: 200, seed: BigInt.from(206)),
        (rng) => gen.sample(rng),
        (input) {
          final corpus = input.$1;
          final seed = input.$2;
          if (corpus.length < 2) return true;
          List<String> run() {
            final chain = MarkovChain(1, random: Random(seed));
            chain.train(corpus);
            try {
              return chain.generate(corpus.first, 25);
            } on UnknownStateError {
              return ['<stuck>'];
            }
          }

          final a = run().join('\x00');
          final b = run().join('\x00');
          return a == b;
        },
      );
      _check('seeded-determinism', verdict);
    });

    test('stationary distribution is a fixed point when it converges', () {
      final gen = Gen.tuple2(_corpus(), _smoothing());
      final verdict = checkProperty<(List<String>, double)>(
        BigInt.from(207),
        PbtOptions(numTests: 100, seed: BigInt.from(207)),
        (rng) => gen.sample(rng),
        (input) {
          final corpus = input.$1;
          final smoothing = input.$2;
          if (corpus.length < 2) return true;
          final chain = MarkovChain(1, smoothing: smoothing);
          chain.train(corpus);
          Map<String, double> pi;
          try {
            pi = chain.stationaryDistribution();
          } on NotErgodicError {
            return true;
          }
          final states = chain.states();
          final sum = pi.values.fold<double>(0, (a, b) => a + b);
          // The reference lets mass drain out of states with no outgoing
          // row, so the sum can be below 1; it must never exceed 1.
          if (sum > 1.0 + 1e-9) return false;
          if ((sum - 1.0).abs() > 1e-6) return true; // drained; skip
          // Fixed point: πT ≈ π.
          for (final sj in states) {
            var acc = 0.0;
            for (final si in states) {
              acc += (pi[si] ?? 0.0) * chain.probability(si, sj);
            }
            if ((acc - (pi[sj] ?? 0.0)).abs() > 1e-6) return false;
          }
          return true;
        },
      );
      _check('stationary-fixed-point', verdict);
    });

    test('error values are exact', () {
      final gen = Gen.tuple2(_order(), _corpus());
      final verdict = checkProperty<(int, List<String>)>(
        BigInt.from(208),
        PbtOptions(numTests: 200, seed: BigInt.from(208)),
        (rng) => gen.sample(rng),
        (input) {
          final order = input.$1;
          final corpus = input.$2;
          final chain = MarkovChain(order);
          // Short training input reports need=order+1, got=len.
          if (corpus.length < order + 1) {
            try {
              chain.train(corpus);
              return false;
            } on SeedTooShortError catch (e) {
              if (e.need != order + 1 || e.got != corpus.length) return false;
            }
            return true;
          }
          chain.train(corpus);
          // Unknown context reports the queried key.
          const ghost = 'ghost-\x01-never-trained';
          try {
            chain.nextState(ghost);
            return false;
          } on UnknownStateError catch (e) {
            if (e.state != ghost) return false;
          }
          // A seed shorter than order reports need=order.
          final shortSeed = List<String>.filled(order - 1, 'a').join('\x00');
          try {
            chain.generate(order == 1 ? '' : shortSeed, 5);
            // order 1 with seed '' (one token): '' may be trained (it is a
            // corpus token above) — only a missing key may throw here.
            if (order == 1) return true;
            return false;
          } on SeedTooShortError catch (e) {
            if (order == 1) return false; // '' is one token, never short
            if (e.need != order || e.got != order - 1) return false;
          } on UnknownStateError {
            if (order != 1) return false;
          }
          // Zero-length generation is always empty.
          if (chain.generate(corpus.first, 0).isNotEmpty) return false;
          return true;
        },
      );
      _check('error-values', verdict);
    });

    test('order-k contexts are order-grams of the corpus', () {
      final gen = Gen.tuple2(_order(), _corpus());
      final verdict = checkProperty<(int, List<String>)>(
        BigInt.from(209),
        PbtOptions(numTests: 200, seed: BigInt.from(209)),
        (rng) => gen.sample(rng),
        (input) {
          final order = input.$1;
          final corpus = input.$2;
          if (corpus.length < order + 1) return true;
          final chain = MarkovChain(order);
          chain.train(corpus);
          // Every context key splits into exactly `order` tokens...
          for (final key in chain.transitionMatrix().keys) {
            if (key.split('\x00').length != order) {
              // ...unless a token itself contains NUL (ambiguous join,
              // same as the reference); then the window must still occur.
              if (!corpus.join('\x00').contains(key)) return false;
            }
          }
          // ...and every corpus window left a row.
          for (var i = 0; i <= corpus.length - order - 1; i++) {
            final key = corpus.sublist(i, i + order).join('\x00');
            if (!chain.transitionMatrix().containsKey(key)) return false;
          }
          return true;
        },
      );
      _check('context-shape', verdict);
    });
  });
}
