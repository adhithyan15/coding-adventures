/// Differential fuzz campaign for `coding_adventures_sha384`, using
/// the in-lane `fuzz` package: hostile byte strings decode into
/// messages and chunkings that drive one-shot and streaming APIs, and
/// the model asserts chunking equivalence, idempotence, continuation,
/// 48-byte digests, and determinism. Only [Sha384MessageTooLong] may
/// ever escape (practically unreachable: inputs are bounded). Fixed
/// seed, so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_sha384/sha384.dart';
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
    final message = Uint8List.fromList(input);
    cov.recordEdge(10 + (message.length % 16));

    Uint8List? want;
    try {
      want = sha384(message);
    } on Sha384MessageTooLong {
      // Bounded inputs can never trip the 2^127-byte counter.
      return const OutcomeCrash('model-divergence', ['spurious-too-long']);
    }
    if (want.length != 48) {
      return const OutcomeCrash('model-divergence', ['digest-length']);
    }
    if (sha384Hex(message).length != 96) {
      return const OutcomeCrash('model-divergence', ['hex-length']);
    }

    // Hostile chunking: 1-6 uneven chunks over the message.
    final chunks = 1 + (message.isEmpty ? 0 : message[0] % 5);
    final h = Sha384Hasher();
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

    // Continuation: split the message, digest the prefix, continue.
    final mid = message.length ~/ 2;
    final prefix = Sha384Hasher()..update(message.sublist(0, mid));
    if (!_equalBytes(prefix.digest(), sha384(message.sublist(0, mid)))) {
      return const OutcomeCrash('model-divergence', ['prefix']);
    }
    prefix.update(message.sublist(mid));
    if (!_equalBytes(prefix.digest(), want)) {
      return const OutcomeCrash('model-divergence', ['continuation']);
    }

    // Determinism: hash twice, compare.
    if (!_equalBytes(sha384(message), want)) {
      return const OutcomeCrash('model-divergence', ['determinism']);
    }

    cov.recordEdge(30 + (want[0] % 8));
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('abc'.codeUnits),
      Uint8List.fromList(List<int>.filled(111, 0x41)),
      Uint8List.fromList(List<int>.filled(112, 0x42)),
      Uint8List.fromList(List<int>.filled(128, 0x43)),
      Uint8List.fromList(List<int>.generate(300, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 471);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 512,
          seed: seed ?? BigInt.from(471)),
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
