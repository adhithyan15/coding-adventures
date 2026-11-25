/// Differential fuzz campaign for `coding_adventures_chacha20_poly1305`,
/// using the in-lane `fuzz` package: hostile byte strings decode
/// into (operation, key, nonce, AAD, message) tuples driving the
/// stream, MAC, AEAD, and XChaCha20 paths on both implementations,
/// and the model asserts round-trips, fixed-vs-compat agreement,
/// tamper rejection, and error exactness — with only
/// [FixedChaCha20Error], [FixedAeadError], and [ArgumentError] ever
/// escaping. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_chacha20_poly1305/chacha20_poly1305.dart';
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
    final op = input[0] % 4;
    cov.recordEdge(op);

    final key = Uint8List.fromList(
        List<int>.generate(32, (i) => input[(i + 1) % input.length]));
    final nonce = Uint8List.fromList(
        List<int>.generate(12, (i) => input[(i + 5) % input.length]));
    final msgLen = input[2] % 129;
    final message = Uint8List.fromList(
        List<int>.generate(msgLen, (i) => input[(i + 9) % input.length]));
    final aad = Uint8List.fromList(
        List<int>.generate(input[3] % 33, (i) => input[i % input.length]));
    cov.recordEdge(10 + (message.length % 16));

    switch (op) {
      case 0:
        final counter = input[1] % 100;
        final ct = chacha20Encrypt(message, key, nonce, counter);
        if (ct.length != message.length) {
          return const OutcomeCrash('model-divergence', ['stream-length']);
        }
        if (!_equalBytes(chacha20Encrypt(ct, key, nonce, counter), message)) {
          return const OutcomeCrash('model-divergence', ['stream-roundtrip']);
        }
        final fixed = Uint8List(message.length);
        chacha20XorFixed(message, key, nonce, counter, fixed);
        if (!_equalBytes(fixed, ct)) {
          return const OutcomeCrash('model-divergence', ['stream-agreement']);
        }
      case 1:
        final tag = poly1305Mac(message, key);
        if (tag.length != 16) {
          return const OutcomeCrash('model-divergence', ['mac-length']);
        }
        if (!_equalBytes(poly1305Mac(message, key), tag)) {
          return const OutcomeCrash('model-divergence', ['mac-determinism']);
        }
        final fixedTag = Uint8List(16);
        poly1305MacFixed(message, key, fixedTag);
        if (!_equalBytes(fixedTag, tag)) {
          return const OutcomeCrash('model-divergence', ['mac-agreement']);
        }
      case 2:
        final (ct, tag) = aeadEncrypt(message, key, nonce, aad);
        if (ct.length != message.length || tag.length != 16) {
          return const OutcomeCrash('model-divergence', ['aead-length']);
        }
        final opened = aeadDecrypt(ct, key, nonce, aad, tag);
        if (opened == null || !_equalBytes(opened, message)) {
          return const OutcomeCrash('model-divergence', ['aead-roundtrip']);
        }
        if (ct.isNotEmpty) {
          final tampered = Uint8List.fromList(ct);
          tampered[0] ^= 1;
          if (aeadDecrypt(tampered, key, nonce, aad, tag) != null) {
            return const OutcomeCrash('model-divergence', ['aead-tamper']);
          }
        }
        final fixedCt = Uint8List(message.length);
        final fixedTag = Uint8List(16);
        aeadEncryptFixed(message, key, nonce, aad, fixedCt, fixedTag);
        if (!_equalBytes(fixedCt, ct) || !_equalBytes(fixedTag, tag)) {
          return const OutcomeCrash('model-divergence', ['aead-agreement']);
        }
      default:
        final nonce24 = Uint8List.fromList(
            List<int>.generate(24, (i) => input[(i + 13) % input.length]));
        final ct = xchacha20Encrypt(message, key, nonce24, 1);
        if (!_equalBytes(xchacha20Encrypt(ct, key, nonce24, 1), message)) {
          return const OutcomeCrash('model-divergence', ['xchacha-roundtrip']);
        }
        final (act, tag) =
            xchacha20Poly1305AeadEncrypt(message, key, nonce24, aad);
        final opened =
            xchacha20Poly1305AeadDecrypt(act, key, nonce24, aad, tag);
        if (opened == null || !_equalBytes(opened, message)) {
          return const OutcomeCrash(
              'model-divergence', ['xchacha-aead-roundtrip']);
        }
    }

    cov.recordEdge(30 + (message.isEmpty ? 0 : message[0] % 8));
    return const OutcomePass();
  } on FixedChaCha20Error catch (e) {
    return OutcomeCrash('model-divergence', ['escaped-${e.runtimeType}']);
  } on FixedAeadError catch (e) {
    return OutcomeCrash('model-divergence', ['escaped-${e.runtimeType}']);
  } on ArgumentError {
    return const OutcomeCrash('model-divergence', ['escaped-argument']);
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList(List<int>.filled(32, 0x00)),
      Uint8List.fromList(List<int>.filled(32, 0xff)),
      Uint8List.fromList('keynonceaadmessagepayload'.codeUnits),
      Uint8List.fromList(List<int>.generate(96, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed
/// 671); longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 192,
          seed: seed ?? BigInt.from(671)),
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
