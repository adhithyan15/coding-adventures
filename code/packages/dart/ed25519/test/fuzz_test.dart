/// Differential fuzz campaign for `coding_adventures_ed25519`, using
/// the in-lane `fuzz` package: hostile byte strings decode into seeds,
/// messages, and mutations (bit flips, truncations, swaps) that drive
/// keygen/sign/verify, and the model asserts round-trip acceptance,
/// tamper rejection, and total non-panicking over hostile keys and
/// signatures. `verify` never throws; only [ArgumentError] from the
/// length-checked constructors may ever escape. Fixed seed, so the
/// campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_ed25519/ed25519.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    if (input.length < 40) {
      // Short inputs: hostile key/signature parsing must never panic.
      try {
        final key = Ed25519PublicKey(
          Uint8List.fromList([...input, ...List<int>.filled(32, 0)]).sublist(
            0,
            32,
          ),
        );
        isValidPublicKey(key.bytes);
      } on ArgumentError {
        // Length-checked constructors: legal escape.
      }
      try {
        verify(
          input,
          Ed25519Signature(Uint8List.fromList(
              List<int>.filled(64, input.isEmpty ? 0 : input[0]))),
          Ed25519PublicKey(Uint8List.fromList(List<int>.filled(32, 1))),
        );
      } on ArgumentError {
        // Legal escape (cannot happen here, but total).
      }
      return const OutcomePass();
    }

    final seed = Uint8List.fromList(input.sublist(0, 32));
    final message =
        Uint8List.fromList(input.sublist(32, 32 + ((input.length - 32) % 200)));
    cov.recordEdge(10 + (message.length % 16));

    final (pub, sec) = generateKeypair(seed);
    final sig = sign(message, sec);
    if (!verify(message, sig, pub)) {
      return const OutcomeCrash('model-divergence', ['round-trip']);
    }

    // Mutate one byte of the signature: must reject, never throw.
    final badSig = Uint8List.fromList(sig.bytes);
    badSig[input[32] % 64] ^= 1 << (input.length % 8);
    if (verify(message, Ed25519Signature(badSig), pub)) {
      // A single-bit flip that still verifies would break Ed25519.
      return const OutcomeCrash('model-divergence', ['tamper-accept']);
    }

    // Mutate one byte of the message (when non-empty): must reject.
    if (message.isNotEmpty) {
      final badMsg = Uint8List.fromList(message);
      badMsg[input.length % message.length] ^= 1;
      if (verify(badMsg, sig, pub)) {
        return const OutcomeCrash('model-divergence', ['msg-tamper-accept']);
      }
    }

    // Cross-check: a second keypair must not accept this signature.
    final seed2 = Uint8List.fromList(
      List<int>.generate(32, (i) => (seed[i] + 1) & 0xff),
    );
    final (pub2, _) = generateKeypair(seed2);
    var same = true;
    for (var i = 0; i < 32; i++) {
      if (pub.bytes[i] != pub2.bytes[i]) same = false;
    }
    if (!same && verify(message, sig, pub2)) {
      return const OutcomeCrash('model-divergence', ['cross-accept']);
    }

    cov.recordEdge(30 + (sig.bytes[0] % 8));
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList(List<int>.filled(32, 0)),
      Uint8List.fromList(List<int>.generate(32, (i) => i)),
      Uint8List.fromList(List<int>.generate(96, (i) => (i * 7 + 3) & 0xff)),
      Uint8List.fromList(List<int>.generate(300, (i) => (i * 13 + 1) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (500 iterations, seed 451);
/// longer ad-hoc runs import this file and pass bigger budgets.
/// Fewer iterations than sibling lanes because each case performs
/// several scalar multiplications (~20ms per case).
FuzzReport runCampaign({int maxIterations = 500, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(451)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: model agrees, no crashes, no panics', () {
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
