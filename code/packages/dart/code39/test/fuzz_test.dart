/// Differential fuzz campaign for `coding_adventures_code_39`, using
/// the in-lane `fuzz` package: hostile byte strings decode into hostile
/// inputs (Latin-1, malformed UTF-8, lone surrogates, `*` placements,
/// over-limit repeats) that drive every API entry point, and an
/// independent oracle rechecks every success — folding, framing, width
/// patterns, run geometry, scene metadata. Only
/// [InvalidCode39InputError] with a stable id, [ArgumentError] for
/// ill-formed host strings, or [Barcode1DV1Error] for hostile scene
/// options may ever escape. Fixed seed, so the campaign reproduces
/// exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_code_39/code_39.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

const Set<String> _stableIds = {
  'input-too-long',
  'invalid-character',
  'reserved-character',
};

/// Independent oracle: ASCII fold + alphabet scan, left to right.
String? _oracleNormalize(String text) {
  final runes = text.runes.toList();
  if (runes.length > 4096) return null;
  final folded = StringBuffer();
  for (final rune in runes) {
    final upper = rune >= 0x61 && rune <= 0x7a ? rune - 0x20 : rune;
    final ch = String.fromCharCode(upper);
    if (ch == '*' || !code39Patterns.containsKey(ch)) return null;
    folded.write(ch);
  }
  return folded.toString();
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final texts = [
      String.fromCharCodes(input),
      utf8.decode(input, allowMalformed: true),
      if (input.isNotEmpty) '${String.fromCharCodes(input)}*',
      if (input.isNotEmpty)
        String.fromCharCodes([
          ...input,
          0xd800 + (input[0] % 0x400),
        ]),
    ];
    if (input.length > 4) {
      texts.add('A' * (4097 + (input[0] % 8)));
    }

    for (var t = 0; t < texts.length; t++) {
      final text = texts[t];
      cov.recordEdge(10 + (text.length % 16));
      String? normalized;
      try {
        normalized = normalizeCode39(text);
      } on InvalidCode39InputError catch (e) {
        if (!_stableIds.contains(e.errorId)) {
          return const OutcomeCrash('bad-error-id', ['normalize']);
        }
        // The oracle must agree this input is unencodable... unless the
        // input holds lone surrogates, which the oracle cannot see
        // (it iterates runes; the encoder rejects the host string).
        if (_oracleNormalize(text) != null && !_hasLoneSurrogate(text)) {
          return const OutcomeCrash('model-divergence', ['oracle-accept']);
        }
        continue;
      } on ArgumentError {
        if (!_hasLoneSurrogate(text)) {
          return const OutcomeCrash('model-divergence', ['spurious-arg-error']);
        }
        continue;
      }

      if (_oracleNormalize(text) != normalized) {
        return const OutcomeCrash('model-divergence', ['oracle-normalize']);
      }

      final encoded = encodeCode39(text);
      if (encoded.length != normalized.length + 2) {
        return const OutcomeCrash('model-divergence', ['encode-length']);
      }
      for (final entry in encoded) {
        if (entry.pattern.length != 9) {
          return const OutcomeCrash('model-divergence', ['pattern-width']);
        }
      }

      final runs = expandCode39Runs(text);
      if (runs.length != encoded.length * 9 + (encoded.length - 1)) {
        return const OutcomeCrash('model-divergence', ['run-count']);
      }
      for (var i = 0; i < runs.length; i++) {
        if (runs[i].modules != 1 && runs[i].modules != 3) {
          return const OutcomeCrash('model-divergence', ['run-width']);
        }
        if (i > 0 && runs[i].color == runs[i - 1].color) {
          return const OutcomeCrash('model-divergence', ['alternation']);
        }
      }

      final hostileLabel = texts[(t + 1) % texts.length];
      final options = Barcode1DSceneOptions(
        label: hostileLabel.length > 5000
            ? hostileLabel.substring(0, 5000)
            : hostileLabel,
        metadata: {'k': texts.last},
      );
      try {
        final scene = layoutCode39(text, options);
        if (scene.metadata['symbology'] != 'code39') {
          return const OutcomeCrash('model-divergence', ['scene-symbology']);
        }
        if (scene.metadata['encodedText'] != normalized) {
          return const OutcomeCrash('model-divergence', ['scene-text']);
        }
        if (scene.metadata['humanReadableText'] != normalized) {
          return const OutcomeCrash('model-divergence', ['scene-hrt']);
        }
        if (scene.instructions.isEmpty) {
          return const OutcomeCrash('model-divergence', ['scene-empty']);
        }
      } on Barcode1DV1Error {
        // Hostile label/metadata tripped a lane ceiling: legal escape.
      }
    }

    cov.recordEdge(30 + (texts.length % 8));
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

bool _hasLoneSurrogate(String text) {
  final units = text.codeUnits;
  for (var i = 0; i < units.length; i++) {
    final unit = units[i];
    if (unit >= 0xd800 && unit <= 0xdbff) {
      if (++i >= units.length || units[i] < 0xdc00 || units[i] > 0xdfff) {
        return true;
      }
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      return true;
    }
  }
  return false;
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('A'.codeUnits),
      Uint8List.fromList('*'.codeUnits),
      Uint8List.fromList('@*'.codeUnits),
      Uint8List.fromList('hello-world'.codeUnits),
      Uint8List.fromList([0xff, 0xfe, 0x00, 0x41, 0x80, 0x81]),
      Uint8List.fromList(List<int>.filled(5000, 0x41)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 351);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(351)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: oracle agrees, errors exact, no crashes', () {
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
