/// Differential fuzz campaign for `coding_adventures_scrypt`,
/// using the in-lane `fuzz` package: hostile byte strings decode
/// into (password, salt, N, r, p, dkLen) tuples driving derivation,
/// and the model asserts exact lengths, determinism, and error
/// exactness — with only [ScryptError] ever escaping. Cost
/// parameters stay small (N ≤ 64); the heavy RFC vectors live in
/// the unit suite. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_scrypt/scrypt.dart';
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
    if (input.length < 6) {
      return const OutcomePass();
    }
    final pwLen = input[0] % (input.length + 1);
    final password = Uint8List.fromList(input.sublist(0, pwLen));
    final salt = Uint8List.fromList(input.sublist(pwLen));
    // N is sometimes a power of two, sometimes hostile garbage.
    final n = input[1] % 2 == 0 ? (1 << (1 + (input[1] % 6))) : input[1];
    final r = input[2] % 5;
    final p = 1 + (input[3] % 2);
    final dkLen = 1 + (input[4] % 96);
    final flip = input.length > 6 ? input[5] % 3 : 0;
    cov.recordEdge(n % 32);
    cov.recordEdge(40 + r);

    Uint8List dk;
    try {
      dk = scrypt(password, salt, n: n, r: r, p: p, dkLen: dkLen);
    } on ScryptError {
      // Expected vocabulary for hostile N/r (p and dkLen are valid
      // by construction here).
      if (r != 0 && (n >= 2 && (n & (n - 1)) == 0)) {
        return const OutcomeCrash('model-divergence', ['spurious-error']);
      }
      return const OutcomePass();
    }
    if (dk.length != dkLen) {
      return const OutcomeCrash('model-divergence', ['dk-length']);
    }
    if (!_equalBytes(
        scrypt(password, salt, n: n, r: r, p: p, dkLen: dkLen), dk)) {
      return const OutcomeCrash('model-divergence', ['determinism']);
    }

    // Sensitivity: one flipped input bit changes the key. Which
    // input flips rotates with the hostile bytes.
    final Uint8List other;
    if (flip == 0 && password.isNotEmpty) {
      final pw2 = Uint8List.fromList(password);
      pw2[input[1] % pw2.length] ^= 1;
      other = scrypt(pw2, salt, n: n, r: r, p: p, dkLen: dkLen);
    } else if (flip == 1 && salt.isNotEmpty) {
      final salt2 = Uint8List.fromList(salt);
      salt2[input[2] % salt2.length] ^= 1;
      other = scrypt(password, salt2, n: n, r: r, p: p, dkLen: dkLen);
    } else {
      // Cost step that stays valid: double N (still a power of two).
      other = scrypt(password, salt, n: n * 2, r: r, p: p, dkLen: dkLen);
    }
    if (_equalBytes(other, dk)) {
      return const OutcomeCrash('model-divergence', ['sensitivity']);
    }

    cov.recordEdge(50 + (dk[0] % 16));
    return const OutcomePass();
  } on ScryptError catch (e) {
    return OutcomeCrash('model-divergence', ['escaped-${e.runtimeType}']);
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('passwordsalt'.codeUnits),
      Uint8List.fromList(List<int>.filled(32, 0x42)),
      Uint8List.fromList(List<int>.generate(64, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed
/// 631); longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 128,
          seed: seed ?? BigInt.from(631)),
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
