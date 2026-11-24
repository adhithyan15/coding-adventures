/// Differential fuzz campaign for `coding_adventures_ean_13`, using the
/// in-lane `fuzz` package: hostile byte strings decode into hostile
/// inputs (Latin-1, malformed UTF-8, lone surrogates, over-limit
/// repeats) that drive every API entry point, and an independent oracle
/// rechecks every success — check digits, parity selection, 95-module
/// geometry, guard placement, alternation, scene metadata. Only
/// [InvalidEan13InputError] with a stable id, [ArgumentError] for
/// ill-formed host strings, or [Barcode1DV1Error] for hostile scene
/// options may ever escape. Fixed seed, so the campaign reproduces
/// exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_ean_13/ean_13.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

const Set<String> _stableIds = {
  'input-too-long',
  'invalid-length',
  'invalid-character',
  'invalid-check-digit',
};

String _oracleCheck(String payload12) {
  var total = 0;
  for (var index = 0; index < 12; index++) {
    final value = payload12.codeUnitAt(11 - index) - 0x30;
    total += value * (index.isEven ? 3 : 1);
  }
  return ((10 - (total % 10)) % 10).toString();
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final texts = [
      String.fromCharCodes(input),
      utf8.decode(input, allowMalformed: true),
      String.fromCharCodes(input.map((b) => b % 10 + 0x30)),
      if (input.isNotEmpty)
        String.fromCharCodes([
          ...input.map((b) => b % 10 + 0x30),
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
        normalized = normalizeEan13(text);
      } on InvalidEan13InputError catch (e) {
        if (!_stableIds.contains(e.errorId)) {
          return const OutcomeCrash('bad-error-id', ['normalize']);
        }
        continue;
      } on ArgumentError {
        continue;
      }

      if (normalized.length != 13) {
        return const OutcomeCrash('model-divergence', ['normalize-length']);
      }
      for (final unit in normalized.codeUnits) {
        if (unit < 0x30 || unit > 0x39) {
          return const OutcomeCrash('model-divergence', ['normalize-digits']);
        }
      }
      if (normalized[12] != _oracleCheck(normalized.substring(0, 12))) {
        return const OutcomeCrash('model-divergence', ['check-oracle']);
      }

      final leading = normalized.codeUnitAt(0) - 0x30;
      final wantParity = ean13LeftParityPatterns[leading];
      if (leftParityPattern(text) != wantParity) {
        return const OutcomeCrash('model-divergence', ['parity']);
      }

      final encoded = encodeEan13(text);
      if (encoded.length != 12) {
        return const OutcomeCrash('model-divergence', ['encode-length']);
      }
      final modules = StringBuffer(ean13SideGuard);
      for (var i = 0; i < 12; i++) {
        if (i == 6) modules.write(ean13CenterGuard);
        final entry = encoded[i];
        if (entry.digit != normalized[i + 1]) {
          return const OutcomeCrash('model-divergence', ['encode-digit']);
        }
        if (entry.sourceIndex != i + 1) {
          return const OutcomeCrash('model-divergence', ['encode-index']);
        }
        modules.write(entry.pattern);
      }
      modules.write(ean13SideGuard);
      final moduleString = modules.toString();
      if (moduleString.length != 95) {
        return const OutcomeCrash('model-divergence', ['module-count']);
      }

      final runs = expandEan13Runs(text);
      final flattened = StringBuffer();
      for (var i = 0; i < runs.length; i++) {
        final run = runs[i];
        if (i > 0 && run.color == runs[i - 1].color) {
          return const OutcomeCrash('model-divergence', ['alternation']);
        }
        flattened.write((run.color == 'bar' ? '1' : '0') * run.modules);
      }
      if (flattened.toString() != moduleString) {
        return const OutcomeCrash('model-divergence', ['runs-vs-modules']);
      }

      final hostileLabel = texts[(t + 1) % texts.length];
      final options = Barcode1DSceneOptions(
        label: hostileLabel.length > 5000
            ? hostileLabel.substring(0, 5000)
            : hostileLabel,
        metadata: {'k': texts.last},
      );
      try {
        final scene = layoutEan13(text, options);
        if (scene.metadata['symbology'] != 'ean-13') {
          return const OutcomeCrash('model-divergence', ['scene-symbology']);
        }
        if (scene.metadata['humanReadableText'] != normalized) {
          return const OutcomeCrash('model-divergence', ['scene-text']);
        }
        if (scene.metadata['leadingDigit'] != normalized[0]) {
          return const OutcomeCrash('model-divergence', ['scene-leading']);
        }
        if (scene.metadata['leftParity'] != wantParity) {
          return const OutcomeCrash('model-divergence', ['scene-parity']);
        }
        if (scene.metadata['contentModules'] != '95') {
          return const OutcomeCrash('model-divergence', ['scene-modules']);
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

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('400638133393'.codeUnits),
      Uint8List.fromList('4006381333932'.codeUnits),
      Uint8List.fromList([0xff, 0xfe, 0x00, 0x41, 0x80, 0x81]),
      Uint8List.fromList(List<int>.generate(48, (i) => 0x30 + (i % 10))),
      Uint8List.fromList(List<int>.filled(5000, 0x41)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 331);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(331)),
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
