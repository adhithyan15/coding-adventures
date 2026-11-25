/// Differential fuzz campaign for `coding_adventures_hkdf`, using
/// the in-lane `fuzz` package: hostile byte strings decode into
/// (algorithm, salt, IKM, info, length) tuples driving legacy and
/// fixed extract/expand, and the model asserts exact lengths,
/// determinism, fixed-vs-legacy agreement, error exactness, and
/// output-untouched-on-error — with only [HkdfError] and
/// [HkdfPrkImportError] ever escaping. Fixed seed, so the campaign
/// reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_hkdf/hkdf.dart';
import 'package:test/test.dart';

bool _equalBytes(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

const _algos = [
  HashAlgorithm.sha256,
  HashAlgorithm.sha512,
  HashAlgorithm.blake2b
];

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    if (input.length < 4) {
      return const OutcomePass();
    }
    final algo = _algos[input[0] % 3];
    final hashLen = algo.hashLen;
    cov.recordEdge(input[0] % 3);

    // Split the hostile input into salt | ikm | info; length stays
    // small (unit tests cover the 255-block maxima).
    final saltLen = input[1] % (input.length + 1);
    final ikmLen = input[2] % (input.length - saltLen + 1);
    final salt = Uint8List.fromList(input.sublist(0, saltLen));
    final ikm = Uint8List.fromList(input.sublist(saltLen, saltLen + ikmLen));
    final info = Uint8List.fromList(input.sublist(saltLen + ikmLen));
    final length = 1 + (input[3] % 200);
    cov.recordEdge(10 + (salt.length % 16));
    cov.recordEdge(30 + (length % 16));

    // Legacy round trip: combined == manual, lengths exact.
    final combined = hkdf(salt, ikm, info, length, algo);
    if (combined.length != length) {
      return const OutcomeCrash('model-divergence', ['combined-length']);
    }
    final prk = hkdfExtract(salt, ikm, algo);
    if (prk.length != hashLen) {
      return const OutcomeCrash('model-divergence', ['prk-length']);
    }
    if (!_equalBytes(hkdfExpand(prk, info, length, algo), combined)) {
      return const OutcomeCrash('model-divergence', ['round-trip']);
    }
    if (!_equalBytes(hkdf(salt, ikm, info, length, algo), combined)) {
      return const OutcomeCrash('model-divergence', ['determinism']);
    }

    // Error matrix on hostile lengths.
    try {
      hkdfExpand(prk, info, 0, algo);
      return const OutcomeCrash('model-divergence', ['zero-accepted']);
    } on HkdfOutputTooShort {
      // Expected.
    }
    final maximum = 255 * hashLen;
    try {
      hkdfExpand(prk, info, maximum + 1, algo);
      return const OutcomeCrash('model-divergence', ['overmax-accepted']);
    } on HkdfOutputTooLong catch (e) {
      if (e.requested != maximum + 1 || e.maximum != maximum) {
        return const OutcomeCrash('model-divergence', ['overmax-payload']);
      }
    }

    // Fixed path agreement (sha256 only; sha384 has no legacy twin).
    if (algo == HashAlgorithm.sha256) {
      final fixed = hkdfExtractFixed(salt, ikm, FixedHkdfHash.sha256);
      try {
        if (!_equalBytes(fixed.asBytes(), prk)) {
          return const OutcomeCrash('model-divergence', ['fixed-extract']);
        }
        final fixedOut = Uint8List(length);
        hkdfExpandFixed(fixed, info, fixedOut);
        if (!_equalBytes(fixedOut, combined)) {
          return const OutcomeCrash('model-divergence', ['fixed-expand']);
        }
        // Error paths leave caller output untouched.
        try {
          hkdfExpandFixed(fixed, info, Uint8List(0));
          return const OutcomeCrash('model-divergence', ['fixed-zero']);
        } on HkdfOutputTooShort {
          // Expected.
        }
        final fixedMax = 255 * fixed.algorithm.hashLen;
        final oversized =
            Uint8List.fromList(List<int>.filled(fixedMax + 1, 0xa5));
        try {
          hkdfExpandFixed(fixed, info, oversized);
          return const OutcomeCrash('model-divergence', ['fixed-overmax']);
        } on HkdfOutputTooLong {
          // Expected.
        }
        if (!oversized.every((b) => b == 0xa5)) {
          return const OutcomeCrash('model-divergence', ['output-touched']);
        }
      } finally {
        fixed.dispose();
      }
    } else {
      // Import round trip on both fixed hashes with hostile bytes.
      for (final fixedAlgo in [FixedHkdfHash.sha256, FixedHkdfHash.sha384]) {
        final fixedPrk = hkdfExtractFixed(salt, ikm, fixedAlgo);
        try {
          final imported = HkdfPrk.importExact(fixedAlgo, fixedPrk.asBytes());
          try {
            final a = Uint8List(length);
            final b = Uint8List(length);
            hkdfExpandFixed(fixedPrk, info, a);
            hkdfExpandFixed(imported, info, b);
            if (!_equalBytes(a, b)) {
              return const OutcomeCrash('model-divergence', ['import']);
            }
          } finally {
            imported.dispose();
          }
        } finally {
          fixedPrk.dispose();
        }
      }
    }

    cov.recordEdge(50 + (combined[0] % 16));
    return const OutcomePass();
  } on HkdfError catch (e) {
    return OutcomeCrash('model-divergence', ['escaped-${e.runtimeType}']);
  } on HkdfPrkImportError {
    return const OutcomeCrash('model-divergence', ['escaped-import']);
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('saltikminfo'.codeUnits),
      Uint8List.fromList(List<int>.filled(32, 0x0b)),
      Uint8List.fromList(List<int>.generate(100, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed
/// 571); longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(571)),
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
