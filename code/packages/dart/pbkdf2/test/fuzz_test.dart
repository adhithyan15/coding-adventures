/// Differential fuzz campaign for `coding_adventures_pbkdf2`,
/// using the in-lane `fuzz` package: hostile byte strings decode
/// into (variant, password, salt, iterations, key length, flag)
/// tuples driving all three variants, and the model asserts exact
/// lengths, determinism, truncation consistency, and error
/// exactness — with only [Pbkdf2Error] ever escaping. Fixed seed, so
/// the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_pbkdf2/pbkdf2.dart';
import 'package:test/test.dart';

bool _equalBytes(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

Uint8List _derive(
  int variant,
  Uint8List password,
  Uint8List salt, {
  required int iterations,
  required int keyLength,
  bool allowEmptyPassword = false,
}) =>
    switch (variant % 3) {
      0 => pbkdf2HmacSha1(password, salt,
          iterations: iterations,
          keyLength: keyLength,
          allowEmptyPassword: allowEmptyPassword),
      1 => pbkdf2HmacSha256(password, salt,
          iterations: iterations,
          keyLength: keyLength,
          allowEmptyPassword: allowEmptyPassword),
      _ => pbkdf2HmacSha512(password, salt,
          iterations: iterations,
          keyLength: keyLength,
          allowEmptyPassword: allowEmptyPassword),
    };

String _hex(Uint8List bytes) {
  const digits = '0123456789abcdef';
  final buffer = StringBuffer();
  for (final byte in bytes) {
    buffer.write(digits[(byte >> 4) & 0xf]);
    buffer.write(digits[byte & 0xf]);
  }
  return buffer.toString();
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    if (input.length < 4) {
      return const OutcomePass();
    }
    final variant = input[0] % 3;
    cov.recordEdge(variant);

    // Split the hostile input into password | salt; iterations and
    // lengths stay small (unit tests cover 4096-iteration vectors
    // and the 2^20 cap).
    final pwLen = input[1] % (input.length + 1);
    final password = Uint8List.fromList(input.sublist(0, pwLen));
    final salt = Uint8List.fromList(input.sublist(pwLen));
    final iterations = input[2] % 11;
    final keyLength = input[3] % 129;
    final allowEmpty = input.length > 4 && input[4] % 2 == 0;
    cov.recordEdge(10 + (password.length % 8));
    cov.recordEdge(20 + iterations);

    // Empty passwords without the flag must raise exactly
    // Pbkdf2EmptyPassword.
    if (password.isEmpty && !allowEmpty) {
      try {
        _derive(variant, password, salt, iterations: 1, keyLength: 16);
        return const OutcomeCrash('model-divergence', ['empty-accepted']);
      } on Pbkdf2EmptyPassword {
        // Expected.
      }
    }

    // Zero counts reject; otherwise lengths are exact.
    if (iterations == 0 || keyLength == 0) {
      try {
        _derive(variant, password, salt,
            iterations: iterations,
            keyLength: keyLength,
            allowEmptyPassword: allowEmpty);
        return const OutcomeCrash('model-divergence', ['zero-accepted']);
      } on Pbkdf2Error {
        return const OutcomePass();
      }
    }
    if (password.isEmpty && !allowEmpty) {
      return const OutcomePass();
    }

    final dk = _derive(variant, password, salt,
        iterations: iterations,
        keyLength: keyLength,
        allowEmptyPassword: allowEmpty);
    if (dk.length != keyLength) {
      return const OutcomeCrash('model-divergence', ['dk-length']);
    }
    if (!_equalBytes(
        _derive(variant, password, salt,
            iterations: iterations,
            keyLength: keyLength,
            allowEmptyPassword: allowEmpty),
        dk)) {
      return const OutcomeCrash('model-divergence', ['determinism']);
    }

    // Truncation: a shorter key prefixes this one.
    if (keyLength > 1) {
      final halfLen = keyLength ~/ 2;
      final half = _derive(variant, password, salt,
          iterations: iterations,
          keyLength: halfLen,
          allowEmptyPassword: allowEmpty);
      if (!_equalBytes(half, dk.sublist(0, halfLen))) {
        return const OutcomeCrash('model-divergence', ['truncation']);
      }
    }

    // Hex convenience agrees with bytes.
    final String hex;
    switch (variant) {
      case 0:
        hex = pbkdf2HmacSha1Hex(password, salt,
            iterations: iterations,
            keyLength: keyLength,
            allowEmptyPassword: allowEmpty);
      case 1:
        hex = pbkdf2HmacSha256Hex(password, salt,
            iterations: iterations,
            keyLength: keyLength,
            allowEmptyPassword: allowEmpty);
      default:
        hex = pbkdf2HmacSha512Hex(password, salt,
            iterations: iterations,
            keyLength: keyLength,
            allowEmptyPassword: allowEmpty);
    }
    if (hex != _hex(dk)) {
      return const OutcomeCrash('model-divergence', ['hex']);
    }

    cov.recordEdge(30 + (dk[0] % 16));
    return const OutcomePass();
  } on Pbkdf2Error catch (e) {
    return OutcomeCrash('model-divergence', ['escaped-${e.runtimeType}']);
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('passwordsalt'.codeUnits),
      Uint8List.fromList(List<int>.filled(20, 0x00)),
      Uint8List.fromList(List<int>.generate(64, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed
/// 591); longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 128,
          seed: seed ?? BigInt.from(591)),
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
