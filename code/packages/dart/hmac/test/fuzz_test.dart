/// Differential fuzz campaign for `coding_adventures_hmac`, using
/// the in-lane `fuzz` package: hostile byte strings decode into
/// (key, message, variant, splits) tuples driving every named
/// variant, the generic higher-order core, and both segmented paths.
/// The model asserts exact tag lengths, determinism, hex shape,
/// generic-vs-fixed agreement, segments equivalence, and verify
/// self-acceptance with single-bit-flip rejection. Only
/// [HmacEmptyKey] may ever escape ([HmacInputTooLong] is unreachable:
/// inputs are bounded). Fixed seed, so the campaign reproduces
/// exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_hmac/hmac.dart';
import 'package:coding_adventures_sha256/coding_adventures_sha256.dart'
    as sha256lane;
import 'package:coding_adventures_sha384/sha384.dart' as sha384lane;
import 'package:test/test.dart';

bool _equalBytes(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

final _hexShape = RegExp(r'^[0-9a-f]+$');

/// Decode a hostile input into a (key, message) pair.
///
/// Short inputs collapse to empty pairs so the empty-key and
/// empty-message paths get exercised; otherwise the key length
/// (0..192, spanning both block sizes) comes from the first byte.
(Uint8List, Uint8List) _decode(Uint8List input) {
  if (input.length < 2) {
    return (Uint8List(0), Uint8List.fromList(input));
  }
  final keyLen = input[0] % 193;
  final keyEnd = keyLen + 2 > input.length ? input.length : keyLen + 2;
  final key = Uint8List.fromList(input.sublist(2, keyEnd));
  final msgEnd = keyEnd + 300 > input.length ? input.length : keyEnd + 300;
  final msg = Uint8List.fromList(input.sublist(keyEnd, msgEnd));
  return (key, msg);
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final (key, msg) = _decode(input);
    final variant = input.isEmpty ? 0 : input[0] % 5;
    cov.recordEdge(variant);
    cov.recordEdge(10 + (key.length % 32));
    cov.recordEdge(50 + (msg.length % 32));

    // Empty keys must raise exactly HmacEmptyKey on every entry point.
    if (key.isEmpty) {
      final throws = [
        () => hmacMd5(key, msg),
        () => hmacSha1(key, msg),
        () => hmacSha256(key, msg),
        () => hmacSha384(key, msg),
        () => hmacSha512(key, msg),
        () => hmacSha256Segments(key, [msg]),
        () => hmacSha384Segments(key, [msg]),
      ];
      for (final fn in throws) {
        try {
          fn();
        } on HmacEmptyKey {
          continue;
        } on HmacInputTooLong {
          return const OutcomeCrash('model-divergence', ['spurious-too-long']);
        }
        return const OutcomeCrash('model-divergence', ['empty-key-no-throw']);
      }
      // Generic hmac() takes any key, including empty ones.
      final generic = hmac(sha256lane.sha256, 64, key, msg);
      if (generic.length != 32) {
        return const OutcomeCrash('model-divergence', ['generic-length']);
      }
      return const OutcomePass();
    }

    // Rotating variant under test: length, determinism, hex, verify.
    final Uint8List tag;
    final String hex;
    final int wantLen;
    switch (variant) {
      case 0:
        tag = hmacMd5(key, msg);
        hex = hmacMd5Hex(key, msg);
        wantLen = 16;
      case 1:
        tag = hmacSha1(key, msg);
        hex = hmacSha1Hex(key, msg);
        wantLen = 20;
      case 2:
        tag = hmacSha256(key, msg);
        hex = hmacSha256Hex(key, msg);
        wantLen = 32;
      case 3:
        tag = hmacSha384(key, msg);
        hex = hmacSha384Hex(key, msg);
        wantLen = 48;
      default:
        tag = hmacSha512(key, msg);
        hex = hmacSha512Hex(key, msg);
        wantLen = 64;
    }
    if (tag.length != wantLen) {
      return const OutcomeCrash('model-divergence', ['tag-length']);
    }
    if (hex.length != wantLen * 2 || !_hexShape.hasMatch(hex)) {
      return const OutcomeCrash('model-divergence', ['hex-shape']);
    }
    if (!verifyHmac(tag, tag)) {
      return const OutcomeCrash('model-divergence', ['verify-self']);
    }
    final flipped = Uint8List.fromList(tag);
    flipped[tag.length ~/ 2] ^= 0x01;
    if (verifyHmac(tag, flipped) || verifyHmac(flipped, tag)) {
      return const OutcomeCrash('model-divergence', ['verify-flip']);
    }

    // Fixed-memory paths agree with the generic core.
    if (!_equalBytes(
        hmacSha256(key, msg), hmac(sha256lane.sha256, 64, key, msg))) {
      return const OutcomeCrash('model-divergence', ['generic-256']);
    }
    if (!_equalBytes(
        hmacSha384(key, msg), hmac(sha384lane.sha384, 128, key, msg))) {
      return const OutcomeCrash('model-divergence', ['generic-384']);
    }

    // Hostile splits: two cuts plus an empty segment.
    final cut1 = msg.isEmpty ? 0 : input[input.length - 1] % (msg.length + 1);
    final cut2 = cut1 + (msg.length - cut1) ~/ 2;
    final segments = [
      msg.sublist(0, cut1),
      <int>[],
      msg.sublist(cut1, cut2),
      msg.sublist(cut2),
    ];
    if (!_equalBytes(hmacSha256Segments(key, segments), hmacSha256(key, msg))) {
      return const OutcomeCrash('model-divergence', ['segments-256']);
    }
    if (!_equalBytes(hmacSha384Segments(key, segments), hmacSha384(key, msg))) {
      return const OutcomeCrash('model-divergence', ['segments-384']);
    }
    if (msg.isEmpty) {
      if (!_equalBytes(hmacSha256Segments(key, []), hmacSha256(key, msg))) {
        return const OutcomeCrash('model-divergence', ['segments-256-empty']);
      }
      if (!_equalBytes(hmacSha384Segments(key, []), hmacSha384(key, msg))) {
        return const OutcomeCrash('model-divergence', ['segments-384-empty']);
      }
    }

    cov.recordEdge(90 + (tag[0] % 16));
    return const OutcomePass();
  } on HmacError catch (e) {
    return OutcomeCrash('model-divergence', ['escaped-${e.runtimeType}']);
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('abc'.codeUnits),
      Uint8List.fromList(List<int>.filled(63, 0x00)),
      Uint8List.fromList(List<int>.filled(64, 0x00)),
      Uint8List.fromList(List<int>.filled(65, 0xff)),
      Uint8List.fromList(List<int>.filled(127, 0x00)),
      Uint8List.fromList(List<int>.filled(128, 0xff)),
      Uint8List.fromList(List<int>.filled(129, 0xaa)),
      Uint8List.fromList(List<int>.generate(300, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 711);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 512,
          seed: seed ?? BigInt.from(711)),
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
