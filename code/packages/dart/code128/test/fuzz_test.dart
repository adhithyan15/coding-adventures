/// Differential fuzz campaign for `coding_adventures_code_128`, using
/// the in-lane `fuzz` package: hostile byte strings decode into hostile
/// inputs (Latin-1, malformed UTF-8, lone surrogates, controls,
/// over-limit repeats) that drive every API entry point, and an
/// independent oracle rechecks every success — checksum, framing,
/// width patterns, run geometry, scene metadata. Only
/// [InvalidCode128InputError] with a stable id, [ArgumentError] for
/// ill-formed host strings, or [Barcode1DV1Error] for hostile scene
/// options may ever escape. Fixed seed, so the campaign reproduces
/// exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_code_128/code_128.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

const Set<String> _stableIds = {'input-too-long', 'invalid-character'};

int _oracleChecksum(List<int> values) {
  var sum = 104;
  for (var i = 0; i < values.length; i++) {
    sum += values[i] * (i + 1);
  }
  return sum % 103;
}

/// Independent oracle: printable-ASCII domain check per the contract.
/// Returns the normalized string, or null when unencodable.
String? _oracleNormalize(String text) {
  final runes = text.runes.toList();
  if (runes.length > 4096) return null;
  for (final rune in runes) {
    if (rune < 0x20 || rune > 0x7e) return null;
  }
  return text;
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final texts = [
      String.fromCharCodes(input),
      utf8.decode(input, allowMalformed: true),
      String.fromCharCodes(input.map((b) => 0x20 + (b % 95))),
      if (input.isNotEmpty)
        String.fromCharCodes([
          ...input.map((b) => 0x20 + (b % 95)),
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
        normalized = normalizeCode128B(text);
      } on InvalidCode128InputError catch (e) {
        if (!_stableIds.contains(e.errorId)) {
          return const OutcomeCrash('bad-error-id', ['normalize']);
        }
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

      final values =
          normalized.codeUnits.map((u) => u - 32).toList(growable: false);
      final wantChecksum = _oracleChecksum(values);
      if (computeCode128Checksum(values) != wantChecksum) {
        return const OutcomeCrash('model-divergence', ['check-oracle']);
      }

      final encoded = encodeCode128B(text);
      if (encoded.length != normalized.length + 3) {
        return const OutcomeCrash('model-divergence', ['encode-length']);
      }
      if (encoded[encoded.length - 2].value != wantChecksum) {
        return const OutcomeCrash('model-divergence', ['encode-checksum']);
      }

      final runs = expandCode128Runs(text);
      for (final run in runs) {
        if (run.role == 'inter-character-gap') {
          return const OutcomeCrash('model-divergence', ['spurious-gap']);
        }
      }
      for (var i = 1; i < runs.length; i++) {
        if (runs[i].color == runs[i - 1].color) {
          return const OutcomeCrash('model-divergence', ['alternation']);
        }
      }
      final total = runs.fold<int>(0, (sum, run) => sum + run.modules);
      if (total != encoded.length * 11 + 2) {
        return const OutcomeCrash('model-divergence', ['module-total']);
      }

      final hostileLabel = texts[(t + 1) % texts.length];
      final options = Barcode1DSceneOptions(
        label: hostileLabel.length > 5000
            ? hostileLabel.substring(0, 5000)
            : hostileLabel,
        metadata: {'k': texts.last},
      );
      try {
        final scene = layoutCode128(text, options);
        if (scene.metadata['symbology'] != 'code128') {
          return const OutcomeCrash('model-divergence', ['scene-symbology']);
        }
        if (scene.metadata['codeSet'] != 'B') {
          return const OutcomeCrash('model-divergence', ['scene-codeset']);
        }
        if (scene.metadata['checksum'] != wantChecksum.toString()) {
          return const OutcomeCrash('model-divergence', ['scene-checksum']);
        }
        if (scene.metadata['humanReadableText'] != normalized) {
          return const OutcomeCrash('model-divergence', ['scene-text']);
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
      Uint8List.fromList('HELLO'.codeUnits),
      Uint8List.fromList([0x00, 0x7f, 0x20, 0x7e]),
      Uint8List.fromList([0xff, 0xfe, 0x00, 0x41, 0x80, 0x81]),
      Uint8List.fromList(List<int>.generate(48, (i) => 0x20 + (i % 95))),
      Uint8List.fromList(List<int>.filled(5000, 0x41)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 391);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(391)),
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
