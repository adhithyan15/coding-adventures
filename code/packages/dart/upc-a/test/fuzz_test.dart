/// Differential fuzz campaign for `coding_adventures_upc_a`, using the
/// in-lane `fuzz` package: hostile byte strings decode into hostile
/// inputs (Latin-1, malformed UTF-8, lone surrogates, over-limit
/// repeats) that drive every API entry point, and an independent oracle
/// rechecks every success — check digits, 95-module geometry, guard
/// placement, alternation, scene metadata. Only [InvalidUpcAInputError]
/// with a stable id, [ArgumentError] for ill-formed host strings, or
/// [Barcode1DV1Error] for hostile scene options may ever escape. Fixed
/// seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_upc_a/upc_a.dart';
import 'package:test/test.dart';

const Set<String> _stableIds = {
  'input-too-long',
  'invalid-length',
  'invalid-character',
  'invalid-check-digit',
};

String _oracleCheck(String payload11) {
  var oddSum = 0;
  var evenSum = 0;
  for (var i = 0; i < 11; i++) {
    final value = payload11.codeUnitAt(i) - 0x30;
    if (i.isEven) {
      oddSum += value;
    } else {
      evenSum += value;
    }
  }
  return (((10 - ((oddSum * 3 + evenSum) % 10)) % 10)).toString();
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    // Four hostile decodings of the same bytes.
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
    // An over-limit repeat form, like the corpus loader's.
    if (input.length > 4) {
      texts.add('A' * (4097 + (input[0] % 8)));
    }

    for (final text in texts) {
      cov.recordEdge(10 + (text.length % 16));
      String? normalized;
      try {
        normalized = normalizeUpcA(text);
      } on InvalidUpcAInputError catch (e) {
        if (!_stableIds.contains(e.errorId)) {
          return const OutcomeCrash('bad-error-id', ['normalize']);
        }
        continue;
      } on ArgumentError {
        continue; // Ill-formed host string: outside v1, legal escape.
      }

      // Success path: every downstream view must agree.
      if (normalized.length != 12) {
        return const OutcomeCrash('model-divergence', ['normalize-length']);
      }
      for (final unit in normalized.codeUnits) {
        if (unit < 0x30 || unit > 0x39) {
          return const OutcomeCrash('model-divergence', ['normalize-digits']);
        }
      }
      if (normalized[11] != _oracleCheck(normalized.substring(0, 11))) {
        return const OutcomeCrash('model-divergence', ['check-oracle']);
      }

      final encoded = encodeUpcA(text);
      if (encoded.length != 12) {
        return const OutcomeCrash('model-divergence', ['encode-length']);
      }
      final modules = StringBuffer(upcASideGuard);
      for (var i = 0; i < 12; i++) {
        if (i == 6) modules.write(upcACenterGuard);
        final entry = encoded[i];
        if (entry.digit != normalized[i]) {
          return const OutcomeCrash('model-divergence', ['encode-digit']);
        }
        modules.write(entry.pattern);
      }
      modules.write(upcASideGuard);
      final moduleString = modules.toString();
      if (moduleString.length != 95) {
        return const OutcomeCrash('model-divergence', ['module-count']);
      }

      final runs = expandUpcARuns(text);
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

      // Scene projection with hostile caller options.
      final hostileLabel = texts[(texts.indexOf(text) + 1) % texts.length];
      final options = Barcode1DSceneOptions(
        label: hostileLabel.length > 5000
            ? hostileLabel.substring(0, 5000)
            : hostileLabel,
        metadata: {'k': texts.last},
      );
      try {
        final scene = layoutUpcA(text, options);
        if (scene.metadata['symbology'] != 'upc-a') {
          return const OutcomeCrash('model-divergence', ['scene-symbology']);
        }
        if (scene.metadata['humanReadableText'] != normalized) {
          return const OutcomeCrash('model-divergence', ['scene-text']);
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
      Uint8List.fromList('03600029145'.codeUnits),
      Uint8List.fromList('036000291453'.codeUnits),
      Uint8List.fromList([0xff, 0xfe, 0x00, 0x41, 0x80, 0x81]),
      Uint8List.fromList(List<int>.generate(48, (i) => 0x30 + (i % 10))),
      Uint8List.fromList(List<int>.filled(5000, 0x41)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 311);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(311)),
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
