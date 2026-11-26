/// Differential fuzz campaign for `coding_adventures_aes`, using
/// the in-lane `fuzz` package: hostile byte strings decode into
/// (key size, key, block) tuples driving encrypt/decrypt on both
/// paths, and the model asserts round-trips, fixed-vs-compat
/// agreement, determinism, and error exactness — with only
/// [AesInvalidKeyLength] and [ArgumentError] ever escaping. Fixed
/// seed, so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_aes/aes.dart';
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
    if (input.length < 2) {
      return const OutcomePass();
    }
    final keyLen = [16, 24, 32][input[0] % 3];
    cov.recordEdge(input[0] % 3);

    // Hostile key material cycled from the input; hostile block from
    // a rotated window so key and block differ structurally.
    final key = Uint8List.fromList(
        List<int>.generate(keyLen, (i) => input[i % input.length]));
    final block = Uint8List.fromList(
        List<int>.generate(16, (i) => input[(i + 5) % input.length]));
    cov.recordEdge(10 + (key[0] % 8));
    cov.recordEdge(20 + (block[0] % 8));

    final ct = encryptBlock(block, key);
    if (ct.length != 16) {
      return const OutcomeCrash('model-divergence', ['ct-length']);
    }
    if (!_equalBytes(decryptBlock(ct, key), block)) {
      return const OutcomeCrash('model-divergence', ['round-trip']);
    }
    if (!_equalBytes(encryptBlock(block, key), ct)) {
      return const OutcomeCrash('model-divergence', ['determinism']);
    }

    // Fixed path agrees on 128-bit keys.
    if (keyLen == 16) {
      final fixed = Uint8List(16);
      aes128EncryptBlockFixed(block, key, fixed);
      if (!_equalBytes(fixed, ct)) {
        return const OutcomeCrash('model-divergence', ['fixed-agreement']);
      }
    }

    // Truncated key must raise exactly AesInvalidKeyLength.
    try {
      encryptBlock(block, key.sublist(0, keyLen - 1));
      return const OutcomeCrash('model-divergence', ['short-key-accepted']);
    } on AesInvalidKeyLength catch (e) {
      if (e.length != keyLen - 1) {
        return const OutcomeCrash('model-divergence', ['short-key-payload']);
      }
    }

    cov.recordEdge(30 + (ct[0] % 16));
    return const OutcomePass();
  } on AesInvalidKeyLength catch (e) {
    return OutcomeCrash('model-divergence', ['escaped-${e.runtimeType}']);
  } on ArgumentError {
    return const OutcomeCrash('model-divergence', ['escaped-argument']);
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList(List<int>.filled(16, 0x00)),
      Uint8List.fromList(List<int>.filled(32, 0xff)),
      Uint8List.fromList(List<int>.generate(48, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed
/// 611); longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 128,
          seed: seed ?? BigInt.from(611)),
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
