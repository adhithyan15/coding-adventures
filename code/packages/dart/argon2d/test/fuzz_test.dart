/// Differential fuzz campaign for `coding_adventures_argon2d`,
/// using the in-lane `fuzz` package: hostile byte strings decode into
/// passwords, salts, keys, associated data, and cost parameters (valid
/// and invalid) that drive hashing, and the model asserts
/// determinism, tag lengths, input binding, and exact error values.
/// Only [Argon2Error] may ever escape. Fixed seed, so the campaign
/// reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_argon2d/argon2d.dart';
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
    if (input.length < 12) {
      // Short inputs: validation must reject or accept cleanly.
      try {
        argon2d(input, input,
            timeCost: 1, memoryCost: 8, parallelism: 1, tagLength: 16);
      } on Argon2Error {
        // Expected vocabulary (short salts, usually).
      }
      return const OutcomePass();
    }

    final pwLen = input[0] % 40;
    final saltLen = input[1] % 24;
    final password =
        Uint8List.fromList(input.sublist(2, 2 + (pwLen % (input.length - 2))));
    final salt = Uint8List.fromList(
        List<int>.generate(saltLen, (i) => input[(i + 2) % input.length]));
    final timeCost = input[3] % 3;
    final parallelism = 1 + (input[5] % 2);
    // Memory always satisfies minimum (unit tests cover the violation).
    final memoryCost = 8 * parallelism * (1 + (input[4] % 2));
    final tagLength = 4 + (input[6] % 64);
    final keyLen = input[7] % 12;
    final adLen = input[8] % 12;

    cov.recordEdge(10 + (password.length % 8));
    cov.recordEdge(20 + timeCost);

    Uint8List? tag;
    try {
      tag = argon2d(password, salt,
          timeCost: timeCost,
          memoryCost: memoryCost,
          parallelism: parallelism,
          tagLength: tagLength,
          opts: Argon2Options(
            key: keyLen == 0
                ? null
                : Uint8List.fromList(List<int>.filled(keyLen, 0x23)),
            associatedData: adLen == 0
                ? null
                : Uint8List.fromList(List<int>.filled(adLen, 0x24)),
          ));
    } on SaltTooShort {
      if (salt.length >= 8) {
        return const OutcomeCrash('model-divergence', ['spurious-salt']);
      }
      return const OutcomePass();
    } on TimeCostZero {
      if (timeCost != 0) {
        return const OutcomeCrash('model-divergence', ['spurious-time']);
      }
      return const OutcomePass();
    } on Argon2Error {
      return const OutcomeCrash('model-divergence', ['unexpected-error']);
    }

    if (tag.length != tagLength) {
      return const OutcomeCrash('model-divergence', ['tag-length']);
    }

    // Determinism: hash twice, compare.
    final again = argon2d(password, salt,
        timeCost: timeCost,
        memoryCost: memoryCost,
        parallelism: parallelism,
        tagLength: tagLength,
        opts: Argon2Options(
          key: keyLen == 0
              ? null
              : Uint8List.fromList(List<int>.filled(keyLen, 0x23)),
          associatedData: adLen == 0
              ? null
              : Uint8List.fromList(List<int>.filled(adLen, 0x24)),
        ));
    if (!_equalBytes(tag, again)) {
      return const OutcomeCrash('model-divergence', ['determinism']);
    }

    // Binding: flipping a password bit changes the tag.
    if (password.isNotEmpty) {
      final flipped = Uint8List.fromList(password);
      flipped[input[9] % flipped.length] ^= 1;
      final other = argon2d(flipped, salt,
          timeCost: timeCost,
          memoryCost: memoryCost,
          parallelism: parallelism,
          tagLength: tagLength,
          opts: Argon2Options(
            key: keyLen == 0
                ? null
                : Uint8List.fromList(List<int>.filled(keyLen, 0x23)),
            associatedData: adLen == 0
                ? null
                : Uint8List.fromList(List<int>.filled(adLen, 0x24)),
          ));
      if (_equalBytes(tag, other)) {
        return const OutcomeCrash('model-divergence', ['binding']);
      }
    }

    cov.recordEdge(30 + (tag[0] % 8));
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('passwordsaltsalt'.codeUnits),
      Uint8List.fromList(List<int>.generate(40, (i) => (i * 7 + 3) & 0xff)),
      Uint8List.fromList(List<int>.filled(64, 0x01)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (300 iterations, seed 511);
/// longer ad-hoc runs import this file and pass bigger budgets.
/// Fewer iterations than sibling lanes: each case hashes up to three
/// times at real cost parameters.
FuzzReport runCampaign({int maxIterations = 300, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 128,
          seed: seed ?? BigInt.from(511)),
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
