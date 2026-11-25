/// Differential fuzz campaign for `coding_adventures_blake2b`, using
/// the in-lane `fuzz` package: hostile byte strings decode into
/// messages, chunkings, and option shapes (valid and invalid) that
/// drive both one-shot and streaming APIs, and the model asserts
/// chunking equivalence, idempotence, digest lengths, and exact error
/// values. Only [Blake2bError] may ever escape. Fixed seed, so the
/// campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_blake2b/blake2b.dart';
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
    if (input.isEmpty) {
      // Empty input still hashes (the empty message).
      if (blake2bHex([]).length != 128) {
        return const OutcomeCrash('model-divergence', ['empty-hex']);
      }
      return const OutcomePass();
    }

    // Decode: message bytes, chunk count, option-shape selectors.
    final split = input.length ~/ 2;
    final message = input.sublist(0, split);
    final meta = input.sublist(split);
    int pick(int i) => i < meta.length ? meta[i] : 0;

    final digestSize = pick(0) % 70; // 0..69: under, valid, and over.
    final keyLen = pick(1) % 70;
    final saltLen = [0, 8, 15, 16, 17, 32][pick(2) % 6];
    final personalLen = [0, 8, 15, 16, 17, 32][pick(3) % 6];
    final options = Blake2bOptions(
      digestSize: digestSize,
      key: Uint8List(keyLen),
      salt: Uint8List(saltLen),
      personal: Uint8List(personalLen),
    );

    cov.recordEdge(10 + (message.length % 16));
    cov.recordEdge(30 + digestSize % 8);

    // Expected validation outcome, computed independently.
    String? expectError;
    if (digestSize < 1 || digestSize > 64) {
      expectError = 'digest';
    } else if (keyLen > 64) {
      expectError = 'key';
    } else if (saltLen != 0 && saltLen != 16) {
      expectError = 'salt';
    } else if (personalLen != 0 && personalLen != 16) {
      expectError = 'personal';
    }

    Uint8List? want;
    try {
      want = blake2b(message, options);
    } on InvalidDigestSize {
      if (expectError != 'digest') {
        return const OutcomeCrash('model-divergence', ['wrong-digest-error']);
      }
    } on KeyTooLong {
      if (expectError != 'key') {
        return const OutcomeCrash('model-divergence', ['wrong-key-error']);
      }
    } on InvalidSaltLength {
      if (expectError != 'salt') {
        return const OutcomeCrash('model-divergence', ['wrong-salt-error']);
      }
    } on InvalidPersonalLength {
      if (expectError != 'personal') {
        return const OutcomeCrash('model-divergence', ['wrong-personal-error']);
      }
    }
    if (expectError != null && want != null) {
      return const OutcomeCrash('model-divergence', ['missing-error']);
    }
    if (want == null) return const OutcomePass();
    if (want.length != digestSize) {
      return const OutcomeCrash('model-divergence', ['digest-length']);
    }

    // Streaming with hostile chunking must agree.
    final chunks = 1 + (pick(4) % 5);
    final h = Blake2bHasher(options);
    for (var c = 0; c < chunks; c++) {
      final from = (message.length * c) ~/ chunks;
      final to = (message.length * (c + 1)) ~/ chunks;
      h.update(message.sublist(from, to));
    }
    if (!_equalBytes(h.digest(), want)) {
      return const OutcomeCrash('model-divergence', ['streaming']);
    }
    if (!_equalBytes(h.digest(), want)) {
      return const OutcomeCrash('model-divergence', ['idempotent']);
    }

    // Copy-then-diverge: the original is unaffected.
    final clone = h.copy();
    h.update([7, 7, 7]);
    if (_equalBytes(h.digest(), clone.digest())) {
      return const OutcomeCrash('model-divergence', ['copy-alias']);
    }

    cov.recordEdge(50 + (want[0] % 8));
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('abc'.codeUnits),
      Uint8List.fromList(List<int>.filled(128, 0x41)),
      Uint8List.fromList(List<int>.filled(129, 0x42)),
      Uint8List.fromList(List<int>.generate(64, (i) => i)),
      Uint8List.fromList(List<int>.generate(300, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 411);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 512,
          seed: seed ?? BigInt.from(411)),
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
