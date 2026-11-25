/// Differential fuzz campaign for `coding_adventures_brotli`,
/// using the in-lane `fuzz` package: hostile byte strings drive
/// compress/decompress round-trips plus hostile wire mutations,
/// and the model asserts exact round-trips, determinism, header
/// shape, and error exactness — with only [BrotliError] ever
/// escaping. Inputs stay small enough to avoid the fail-closed
/// 256-symbol buckets (pinned in unit tests). Fixed seed, so the
/// campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_brotli/brotli.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

bool _equalBytes(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    // Small alphabet keeps buckets clear of the 256-symbol guard.
    final data = Uint8List.fromList(input.map((b) => 0x20 + (b % 64)).toList());
    cov.recordEdge(data.length % 16);
    cov.recordEdge(20 + (data.isEmpty ? 0 : data[0] % 8));

    final compressed = compress(data);
    if (compressed.length < 10 && data.isNotEmpty) {
      return const OutcomeCrash('model-divergence', ['too-short']);
    }
    if (data.isEmpty) {
      if (!_equalBytes(compressed, [0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 63, 1, 0])) {
        return const OutcomeCrash('model-divergence', ['empty-wire']);
      }
      return const OutcomePass();
    }
    // Header length matches.
    final headerLen = (compressed[0] << 24) |
        (compressed[1] << 16) |
        (compressed[2] << 8) |
        compressed[3];
    if (headerLen != data.length) {
      return const OutcomeCrash('model-divergence', ['header-length']);
    }
    // Round trip + determinism.
    if (!_equalBytes(decompress(compressed), data)) {
      return const OutcomeCrash('model-divergence', ['roundtrip']);
    }
    if (!_equalBytes(compress(data), compressed)) {
      return const OutcomeCrash('model-divergence', ['determinism']);
    }

    // Hostile wire mutations: exact, or BrotliError — never anything
    // else, never wrong bytes on success... except bit flips, which
    // (like all unprotected formats) may silently alter output.
    final which = data.length % 3;
    if (which == 0) {
      // Truncation: success implies exact bytes (prefix property).
      final cut = compressed.length ~/ 2;
      try {
        final out = decompress(compressed.sublist(0, cut));
        if (!_equalBytes(out, data)) {
          return const OutcomeCrash('model-divergence', ['truncation-exact']);
        }
      } on BrotliError {
        // Expected vocabulary.
      }
    } else if (which == 1 && compressed.length > 10) {
      // Bit flip in the body: must not escape the vocabulary.
      final mutated = Uint8List.fromList(compressed);
      mutated[10 + (data[0] % (compressed.length - 10))] ^=
          1 << (data.length % 8);
      try {
        decompress(mutated);
      } on BrotliError {
        // Expected vocabulary.
      }
    } else if (compressed.length > 12) {
      // Header-count tamper: must not escape the vocabulary.
      final mutated = Uint8List.fromList(compressed);
      mutated[4 + (data.length % 6)] ^= 0xff;
      try {
        decompress(mutated);
      } on BrotliError {
        // Expected vocabulary.
      }
    }

    cov.recordEdge(40 + (compressed.length % 16));
    return const OutcomePass();
  } on BrotliError catch (e) {
    return OutcomeCrash('model-divergence', ['escaped-${e.runtimeType}']);
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('hello world'.codeUnits),
      Uint8List.fromList(List<int>.filled(32, 0x41)),
      Uint8List.fromList(List<int>.generate(96, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed
/// 731); longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 160,
          seed: seed ?? BigInt.from(731)),
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
