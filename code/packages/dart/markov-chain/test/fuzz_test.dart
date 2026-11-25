/// Script-driven differential fuzz campaign for
/// `coding_adventures_markov_chain`, using the in-lane `fuzz` package:
/// hostile byte strings decode into op scripts (train on hostile token
/// sequences, string-API round trips) interleaved with full-state queries,
/// and an independent sliding-window model predicts every observable —
/// alphabet, probabilities, row sums. Only the exact expected
/// [MarkovError] subtypes may ever escape. Fixed seed, so the campaign
/// reproduces exactly.
library;

import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_markov_chain/markov_chain.dart';
import 'package:test/test.dart';

// Hostile token pool: empty, NUL-bearing, astral, lone surrogates (built
// at runtime — source text cannot hold them), long runs, and
// separator-adjacent strings. Non-ASCII entries use escapes because raw
// non-ASCII literals can mangle in transit.
final List<String> _tokenPool = [
  '',
  'a',
  '\x00',
  'x\x00y',
  '\x00\x00',
  'A',
  'longer-token-with-dashes-0123456789',
  '\u{1F600}',
  '\u{E9}',
  '\u{F800}',
  '\u{10FFFF}',
  String.fromCharCodes([0xD800]),
  String.fromCharCodes([0xDFFF]),
  ' ',
  '\n',
  'null',
];

/// Independent model: raw counts plus the alphabet, recomputed with a
/// plain sliding window. Probabilities derive from the same formula the
/// reference documents: (count + α) / (rawTotal + α·|alphabet|).
class _Model {
  final int order;
  final double smoothing;
  final List<String> alphabet = [];
  final Map<String, Map<String, int>> counts = {};

  _Model(this.order, this.smoothing);

  void train(List<String> seq) {
    for (final s in seq) {
      if (!alphabet.contains(s)) alphabet.add(s);
    }
    if (seq.length < order + 1) return;
    for (var i = 0; i <= seq.length - order - 1; i++) {
      final key = seq.sublist(i, i + order).join('\x00');
      final target = seq[i + order];
      final row = counts.putIfAbsent(key, () => {});
      row[target] = (row[target] ?? 0) + 1;
    }
  }

  double probability(String from, String to) {
    final obs = counts[from];
    if (obs == null) return 0.0;
    final rawTotal = obs.values.fold<int>(0, (a, b) => a + b);
    final total = rawTotal + smoothing * alphabet.length;
    if (total == 0.0) return 0.0;
    if (smoothing > 0) {
      if (!alphabet.contains(to)) return 0.0;
      return ((obs[to] ?? 0) + smoothing) / total;
    }
    return (obs[to] ?? 0) / total;
  }
}

/// Decodes one fuzz input into an op script and checks the chain against
/// the model after every op. Any divergence or any unexpected exception
/// becomes an [OutcomeCrash]; [MarkovError] is the only legal escape.
FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final bytes = input;
    var cursor = 0;
    int nextByte() => cursor < bytes.length ? bytes[cursor++] : 0;

    String nextToken() => _tokenPool[nextByte() % _tokenPool.length];

    List<String> nextSeq() {
      final len = nextByte() % 24;
      return List<String>.generate(len, (_) => nextToken());
    }

    // Lone surrogates via fromCharCodes plus malformed-UTF-8-tolerant
    // decoding give us strings Rust `str` could never hold; the string
    // APIs must never throw on them.
    String nextText() {
      final choice = nextByte() % 3;
      final take = nextByte() % 32;
      final end = (cursor + take).clamp(0, bytes.length);
      final chunk = bytes.sublist(cursor, end);
      cursor = end;
      if (choice == 0) return String.fromCharCodes(chunk);
      if (choice == 1) {
        return utf8.decode(chunk, allowMalformed: true);
      }
      return chunk.map((b) => _tokenPool[b % _tokenPool.length]).join();
    }

    final order = (nextByte() % 3) + 1;
    final smoothing = [0.0, 0.5, 1.0][nextByte() % 3];
    final seed = nextByte();

    cov.recordEdge(order);
    cov.recordEdge((smoothing * 2).toInt());

    final chain =
        MarkovChain(order, smoothing: smoothing, random: Random(seed));
    final model = _Model(order, smoothing);

    void checkState(String op) {
      // Alphabet matches (insertion order).
      final got = chain.states();
      if (got.length != model.alphabet.length) {
        throw StateError(
            '$op: alphabet length ${got.length} != ${model.alphabet.length}');
      }
      for (var i = 0; i < got.length; i++) {
        if (got[i] != model.alphabet[i]) {
          throw StateError(
              '$op: alphabet[$i] ${got[i]} != ${model.alphabet[i]}');
        }
      }
      // Every model row matches the chain's probabilities.
      for (final from in model.counts.keys) {
        for (final to in model.alphabet) {
          final want = model.probability(from, to);
          final actual = chain.probability(from, to);
          if ((want - actual).abs() > 1e-12) {
            throw StateError('$op: P($from → $to) = $actual, want $want');
          }
        }
      }
      // Row sums hold on the chain side too.
      for (final row in chain.transitionMatrix().values) {
        final sum = row.values.fold<double>(0, (a, b) => a + b);
        if ((sum - 1.0).abs() > 1e-9) {
          throw StateError('$op: row sums to $sum');
        }
      }
    }

    final opCount = (nextByte() % 12) + 1;
    for (var op = 0; op < opCount; op++) {
      final kind = nextByte() % 8;
      cov.recordEdge(100 + kind);
      try {
        switch (kind) {
          case 0:
            final seq = nextSeq();
            try {
              chain.train(seq);
              model.train(seq);
            } on SeedTooShortError {
              // Short input registers no states in either implementation.
              if (seq.length >= order + 1) rethrow;
            }
          case 1:
            final text = nextText();
            try {
              chain.trainString(text);
              model.train(text.runes.map(String.fromCharCode).toList());
            } on SeedTooShortError {
              if (text.runes.length >= order + 1) rethrow;
            }
          case 2:
            final current = nextToken();
            try {
              final next = chain.nextState(current);
              if (!chain.states().contains(next)) {
                throw StateError('nextState returned unknown token');
              }
            } on UnknownStateError {
              if (model.counts.containsKey(current)) rethrow;
            }
          case 3:
            final start = nextToken();
            final length = nextByte() % 20;
            try {
              final seq = chain.generate(start, length);
              if (length == 0) {
                // Length zero short-circuits to [] before seed checks.
                if (seq.isNotEmpty) {
                  throw StateError('generate(*, 0) not empty');
                }
              } else {
                _expectSeededGenerate(seq, start, length);
              }
            } on SeedTooShortError {
              if (start.split('\x00').length >= order) rethrow;
            } on UnknownStateError {
              if (length != 0 && model.counts.containsKey(start)) rethrow;
            }
          case 4:
            final seedText = nextText();
            final length = nextByte() % 20;
            try {
              final out = chain.generateString(seedText, length);
              // generateString seeds with exactly `order` tokens; a
              // shorter nonzero `length` returns the seed verbatim
              // (reference), while zero short-circuits to ''.
              final wantLen =
                  length == 0 ? 0 : (order > length ? order : length);
              if (out.runes.length != wantLen) {
                throw StateError(
                    'generateString runes ${out.runes.length} != $wantLen');
              }
            } on SeedTooShortError {
              if (seedText.runes.length >= order) rethrow;
            } on UnknownStateError {
              // The first order-rune context was never trained; legal.
            }
          case 5:
            // Seeded determinism: the same seed replays identically.
            List<String> run() {
              final replay = MarkovChain(order,
                  smoothing: smoothing, random: Random(seed));
              const probe = ['a', 'b', 'a', 'c', 'a', 'b'];
              try {
                replay.train(probe);
              } on SeedTooShortError {
                return ['<short>'];
              }
              return _safeGenerate(replay, probe.first, 15);
            }

            if (run().join('\x00') != run().join('\x00')) {
              throw StateError('seeded replay diverged');
            }
          case 6:
            try {
              final dist = chain.stationaryDistribution();
              final sum = dist.values.fold<double>(0, (a, b) => a + b);
              if (sum > 1.0 + 1e-9 || sum < -1e-9) {
                throw StateError('stationary sum out of range: $sum');
              }
              for (final p in dist.values) {
                if (p.isNaN || p.isInfinite || p < -1e-9) {
                  throw StateError('stationary entry invalid: $p');
                }
              }
            } on NotErgodicError {
              // Legal: oscillation or empty state set.
            }
          default:
            // Pure queries must never throw.
            chain.probability(nextToken(), nextToken());
            chain.states();
            chain.transitionMatrix();
        }
      } on MarkovError {
        // Expected vocabulary: already validated per-op above.
      }
      checkState('op$op/kind$kind');
    }
    cov.recordEdge(10 + (model.alphabet.length % 8));
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

/// Asserts reference `generate` shape: the seed verbatim when it exceeds
/// `length`, otherwise exactly `length` tokens starting with the seed.
void _expectSeededGenerate(List<String> seq, String start, int length) {
  final seedTokens = start.split('\x00');
  // Reference behavior: the output starts with the seed tokens, then
  // fills up to `length` — so a seed longer than `length` is returned
  // verbatim.
  final wantLen = seedTokens.length > length ? seedTokens.length : length;
  if (seq.length != wantLen) {
    throw StateError('generate length ${seq.length} != $wantLen');
  }
  for (var i = 0; i < seedTokens.length; i++) {
    if (seq[i] != seedTokens[i]) {
      throw StateError('generate prefix diverged from seed');
    }
  }
}

List<String> _safeGenerate(MarkovChain chain, String start, int length) {
  try {
    return chain.generate(start, length);
  } on UnknownStateError {
    return ['<stuck>'];
  } on SeedTooShortError {
    return ['<short>'];
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList([1, 0, 7, 5, 3, 1, 4, 1, 5, 9, 2, 6]),
      Uint8List.fromList(List<int>.generate(64, (i) => i * 3 + 1)),
      Uint8List.fromList([3, 2, 0, 0, 7, 7, 7, 0]),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 107);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(107)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: model agrees, errors exact, no crashes', () {
      final report = runCampaign();
      expect(report.crashes, isEmpty,
          reason: report.crashes
              .map((c) =>
                  '${c.signature} ${c.kind} ${c.frames} input=${c.minimalInput}')
              .join('; '));
      expect(report.iterations, greaterThan(100));
      expect(report.totalEdges, greaterThan(5));
    });
  });
}
