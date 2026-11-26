/// Differential fuzz campaign for `coding_adventures_x25519`,
/// using the in-lane `fuzz` package: hostile byte strings decode
/// into scalar/u-coordinate pairs driving key agreement, and the
/// model asserts exact output lengths, determinism, clamp
/// idempotence, and DH agreement — with only
/// [X25519LowOrderPoint] ever escaping. Fixed seed, so the campaign
/// reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_x25519/x25519.dart';
import 'package:test/test.dart';

bool _equalBytes(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// Decodes 32 bytes by cycling the hostile input (short inputs still
/// yield full-length keys with hostile structure).
Uint8List _take32(Uint8List input, int offset) {
  final out = Uint8List(32);
  for (var i = 0; i < 32; i++) {
    out[i] = input[(offset + i) % input.length];
  }
  return out;
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    if (input.isEmpty) {
      return const OutcomePass();
    }
    final scalarA = _take32(input, 0);
    final scalarB = _take32(input, 7);
    cov.recordEdge(scalarA[0] % 16);
    cov.recordEdge(20 + (scalarB[0] % 16));

    // Clamp idempotence and shape on hostile scalars.
    for (final s in [scalarA, scalarB]) {
      final once = clampScalar(s);
      if (!_equalBytes(once, clampScalar(once))) {
        return const OutcomeCrash('model-divergence', ['clamp-idempotent']);
      }
      if (once[0] & 0x07 != 0 ||
          once[31] >> 7 != 0 ||
          (once[31] >> 6) & 1 != 1) {
        return const OutcomeCrash('model-divergence', ['clamp-shape']);
      }
    }

    Uint8List pubA;
    Uint8List pubB;
    try {
      pubA = x25519Base(scalarA);
      pubB = x25519Base(scalarB);
    } on X25519LowOrderPoint {
      // Vanishingly rare (scalar multiple of L) but legal: rejection
      // must be deterministic.
      try {
        x25519Base(scalarA);
        x25519Base(scalarB);
      } on X25519LowOrderPoint {
        return const OutcomePass();
      }
      return const OutcomeCrash('model-divergence', ['flaky-base-reject']);
    }
    if (pubA.length != 32 || pubB.length != 32) {
      return const OutcomeCrash('model-divergence', ['pub-length']);
    }

    // DH agreement in both directions.
    Uint8List ab;
    Uint8List ba;
    try {
      ab = x25519(scalarA, pubB);
      ba = x25519(scalarB, pubA);
    } on X25519LowOrderPoint {
      // Rejection must be deterministic.
      try {
        x25519(scalarA, pubB);
        x25519(scalarB, pubA);
      } on X25519LowOrderPoint {
        return const OutcomePass();
      }
      return const OutcomeCrash('model-divergence', ['flaky-rejection']);
    }
    if (!_equalBytes(ab, ba)) {
      return const OutcomeCrash('model-divergence', ['agreement']);
    }
    if (ab.length != 32) {
      return const OutcomeCrash('model-divergence', ['shared-length']);
    }

    // Determinism: agree twice.
    if (!_equalBytes(x25519(scalarA, pubB), ab)) {
      return const OutcomeCrash('model-divergence', ['determinism']);
    }

    // Raw hostile u-coordinate: lengths and determinism hold, and
    // only the low-order error may escape.
    final u = _take32(input, 13);
    try {
      final tag = x25519(scalarA, u);
      if (tag.length != 32) {
        return const OutcomeCrash('model-divergence', ['raw-length']);
      }
      if (!_equalBytes(x25519(scalarA, u), tag)) {
        return const OutcomeCrash('model-divergence', ['raw-determinism']);
      }
      cov.recordEdge(40 + (tag[0] % 8));
    } on X25519LowOrderPoint {
      // Expected vocabulary for hostile u values.
    }
    return const OutcomePass();
  } on X25519LowOrderPoint catch (e) {
    return OutcomeCrash('model-divergence', ['escaped-${e.runtimeType}']);
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList(List<int>.filled(32, 0x00)),
      Uint8List.fromList(List<int>.filled(32, 0xff)),
      Uint8List.fromList(List<int>.filled(64, 0x01)),
      Uint8List.fromList(List<int>.generate(96, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed
/// 551); longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(551)),
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
