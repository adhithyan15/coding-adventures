/// Differential fuzz campaign for `coding_adventures_aes_modes`,
/// using the in-lane `fuzz` package: hostile byte strings decode
/// into (mode, key, IV, AAD, plaintext) tuples driving every mode
/// on both paths, and the model asserts round-trips, fixed-vs-compat
/// agreement, padding exactness, and error exactness — with only
/// [AesModesError], [AesInvalidKeyLength], [GhashInputTooLong],
/// [FixedAes128GcmError], and [ArgumentError] ever escaping. Fixed
/// seed, so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_aes/aes.dart' as aes;
import 'package:coding_adventures_aes_modes/aes_modes.dart';
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
    if (input.length < 4) {
      return const OutcomePass();
    }
    final mode = input[0] % 4;
    final keyLen = [16, 24, 32][input[1] % 3];
    cov.recordEdge(mode * 10 + (input[1] % 3));

    final key = Uint8List.fromList(
        List<int>.generate(keyLen, (i) => input[(i + 2) % input.length]));
    final ptLen = input[3] % 97;
    final pt = Uint8List.fromList(
        List<int>.generate(ptLen, (i) => input[(i + 7) % input.length]));
    final iv = Uint8List.fromList(
        List<int>.generate(16, (i) => input[(i + 11) % input.length]));
    final nonce = Uint8List.fromList(
        List<int>.generate(12, (i) => input[(i + 13) % input.length]));
    final aad = Uint8List.fromList(
        List<int>.generate(input.length % 33, (i) => input[i]));
    cov.recordEdge(40 + (pt.length % 16));

    switch (mode) {
      case 0:
        final ct = ecbEncrypt(pt, key);
        if (ct.length % 16 != 0 || ct.length < pt.length + 1) {
          return const OutcomeCrash('model-divergence', ['ecb-length']);
        }
        if (!_equalBytes(ecbDecrypt(ct, key), pt)) {
          return const OutcomeCrash('model-divergence', ['ecb-roundtrip']);
        }
      case 1:
        final ct = cbcEncrypt(pt, key, iv);
        if (!_equalBytes(cbcDecrypt(ct, key, iv), pt)) {
          return const OutcomeCrash('model-divergence', ['cbc-roundtrip']);
        }
        // Wrong IV decrypts to different bytes (or padding failure).
        final iv2 = Uint8List.fromList(iv)..[0] ^= 1;
        try {
          if (_equalBytes(cbcDecrypt(ct, key, iv2), pt) && pt.isNotEmpty) {
            return const OutcomeCrash('model-divergence', ['cbc-iv']);
          }
        } on AesModesError {
          // Expected vocabulary when padding breaks.
        }
      case 2:
        final ct = ctrEncrypt(pt, key, nonce);
        if (ct.length != pt.length) {
          return const OutcomeCrash('model-divergence', ['ctr-length']);
        }
        if (!_equalBytes(ctrDecrypt(ct, key, nonce), pt)) {
          return const OutcomeCrash('model-divergence', ['ctr-roundtrip']);
        }
      default:
        final (ct, tag) = gcmEncrypt(pt, key, nonce, aad);
        if (ct.length != pt.length || tag.length != 16) {
          return const OutcomeCrash('model-divergence', ['gcm-length']);
        }
        if (!_equalBytes(gcmDecrypt(ct, key, nonce, aad, tag), pt)) {
          return const OutcomeCrash('model-divergence', ['gcm-roundtrip']);
        }
        // Tampered tag must reject.
        final badTag = Uint8List.fromList(tag)..[0] ^= 1;
        try {
          gcmDecrypt(ct, key, nonce, aad, badTag);
          return const OutcomeCrash('model-divergence', ['gcm-tamper']);
        } on GcmAuthenticationFailed {
          // Expected.
        }
        // Fixed path agrees on 128-bit keys.
        if (keyLen == 16) {
          final fixedCt = Uint8List(pt.length);
          final fixedTag = Uint8List(16);
          aes128GcmEncryptFixed(pt, key, nonce, aad, fixedCt, fixedTag);
          if (!_equalBytes(fixedCt, ct) || !_equalBytes(fixedTag, tag)) {
            return const OutcomeCrash('model-divergence', ['fixed-agreement']);
          }
          final opened = Uint8List(pt.length);
          aes128GcmDecryptFixed(ct, key, nonce, aad, tag, opened);
          if (!_equalBytes(opened, pt)) {
            return const OutcomeCrash('model-divergence', ['fixed-open']);
          }
        }
    }

    // Padding round-trips hostile lengths exactly.
    final padded = pkcs7Pad(pt);
    if (!_equalBytes(pkcs7Unpad(padded), pt)) {
      return const OutcomeCrash('model-divergence', ['padding']);
    }

    cov.recordEdge(60 + (pt.isEmpty ? 0 : pt[0] % 8));
    return const OutcomePass();
  } on AesModesError catch (e) {
    return OutcomeCrash('model-divergence', ['escaped-${e.runtimeType}']);
  } on aes.AesInvalidKeyLength {
    return const OutcomeCrash('model-divergence', ['escaped-key-length']);
  } on GhashInputTooLong {
    return const OutcomeCrash('model-divergence', ['escaped-ghash']);
  } on FixedAes128GcmError catch (e) {
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
      Uint8List.fromList('keyivnonceaadplaintext'.codeUnits),
      Uint8List.fromList(List<int>.generate(80, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed
/// 651); longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 192,
          seed: seed ?? BigInt.from(651)),
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
