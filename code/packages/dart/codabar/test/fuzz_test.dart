/// Differential fuzz campaign for `coding_adventures_codabar`, using
/// the in-lane `fuzz` package: hostile byte strings decode into hostile
/// inputs and guard options (Latin-1, malformed UTF-8, lone surrogates,
/// `*` placements, over-limit repeats) that drive every API entry
/// point, and an independent oracle rechecks every success — folding,
/// framing, width patterns, run geometry, scene metadata. Only
/// [InvalidCodabarInputError] with a stable id, [ArgumentError] for
/// ill-formed host strings, or [Barcode1DV1Error] for hostile scene
/// options may ever escape. Fixed seed, so the campaign reproduces
/// exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_codabar/codabar.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

const Set<String> _stableIds = {
  'input-too-long',
  'invalid-character',
  'invalid-guard',
  'guard-conflict',
};

bool _isGuard(String ch) => ch == 'A' || ch == 'B' || ch == 'C' || ch == 'D';

/// Independent oracle: fold, frame, and scan per the v1 contract.
/// Returns the normalized string, or null when the input is unencodable
/// (the exact id is the implementation's job; agreement on
/// encodable/unencodable is the oracle's).
String? _oracleNormalize(String text, String? start, String? stop) {
  final runes = text.runes.toList();
  if (runes.length > 4096) return null;
  final folded = String.fromCharCodes(
    runes.map((r) => r >= 0x61 && r <= 0x7a ? r - 0x20 : r),
  );

  String? foldOpt(String? opt) {
    if (opt == null) return null;
    final chars = opt.runes.toList();
    if (chars.length != 1) return '';
    final foldedOpt = chars.single >= 0x61 && chars.single <= 0x7a
        ? String.fromCharCode(chars.single - 0x20)
        : opt;
    return _isGuard(foldedOpt) ? foldedOpt : '';
  }

  final wantStart = foldOpt(start);
  final wantStop = foldOpt(stop);
  final firstGuard = folded.isNotEmpty && _isGuard(folded[0]);
  final lastGuard = folded.isNotEmpty && _isGuard(folded[folded.length - 1]);

  if ((firstGuard || lastGuard) &&
      !(folded.length >= 2 && firstGuard && lastGuard)) {
    return null;
  }
  if (folded.length >= 2 && firstGuard && lastGuard) {
    if ((wantStart != null && wantStart != folded[0]) ||
        (wantStop != null && wantStop != folded[folded.length - 1])) {
      return null;
    }
    for (var i = 1; i < folded.length - 1; i++) {
      final ch = folded[i];
      if (_isGuard(ch) || !codabarPatterns.containsKey(ch)) return null;
    }
    return folded;
  }
  if (wantStart == '' || wantStop == '') return null;
  for (var i = 0; i < folded.length; i++) {
    final ch = folded[i];
    if (_isGuard(ch) || !codabarPatterns.containsKey(ch)) return null;
  }
  return '${wantStart ?? 'A'}$folded${wantStop ?? 'A'}';
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final texts = [
      String.fromCharCodes(input),
      utf8.decode(input, allowMalformed: true),
      if (input.isNotEmpty) 'A${String.fromCharCodes(input)}B',
    ];
    if (input.length > 4) {
      texts.add('a' * (4097 + (input[0] % 8)));
    }
    final options = <String?>[null, 'A', 'b', 'X', '', 'AB'];

    for (final text in texts) {
      for (final start in options) {
        for (final stop in options) {
          cov.recordEdge(
            10 + (text.length + (start?.length ?? 0)) % 16,
          );
          String? normalized;
          try {
            normalized = normalizeCodabar(text, start: start, stop: stop);
          } on InvalidCodabarInputError catch (e) {
            if (!_stableIds.contains(e.errorId)) {
              return const OutcomeCrash('bad-error-id', ['normalize']);
            }
            if (_oracleNormalize(text, start, stop) != null &&
                !_hasLoneSurrogate(text)) {
              return const OutcomeCrash('model-divergence', ['oracle-accept']);
            }
            continue;
          } on ArgumentError {
            if (!_hasLoneSurrogate(text) &&
                (start == null || !_hasLoneSurrogate(start)) &&
                (stop == null || !_hasLoneSurrogate(stop))) {
              return const OutcomeCrash(
                'model-divergence',
                ['spurious-arg-error'],
              );
            }
            continue;
          }

          if (_oracleNormalize(text, start, stop) != normalized) {
            return const OutcomeCrash('model-divergence', ['oracle-normalize']);
          }

          final encoded = encodeCodabar(text, start: start, stop: stop);
          if (encoded.length != normalized.length) {
            return const OutcomeCrash('model-divergence', ['encode-length']);
          }
          for (var i = 0; i < encoded.length; i++) {
            if (encoded[i].ch != normalized[i]) {
              return const OutcomeCrash('model-divergence', ['encode-char']);
            }
          }

          final runs = expandCodabarRuns(text, start: start, stop: stop);
          final gaps =
              runs.where((r) => r.role == 'inter-character-gap').toList();
          if (gaps.length != encoded.length - 1) {
            return const OutcomeCrash('model-divergence', ['gap-count']);
          }
          for (var i = 1; i < runs.length; i++) {
            if (runs[i].color == runs[i - 1].color) {
              return const OutcomeCrash('model-divergence', ['alternation']);
            }
          }

          final scene = layoutCodabar(
            text,
            start: start,
            stop: stop,
            options: const Barcode1DSceneOptions(label: 'fuzz'),
          );
          if (scene.metadata['symbology'] != 'codabar') {
            return const OutcomeCrash('model-divergence', ['scene-symbology']);
          }
          if (scene.metadata['start'] != normalized[0] ||
              scene.metadata['stop'] != normalized[normalized.length - 1]) {
            return const OutcomeCrash('model-divergence', ['scene-guards']);
          }
          if (scene.instructions.isEmpty) {
            return const OutcomeCrash('model-divergence', ['scene-empty']);
          }
        }
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
      Uint8List.fromList('40156'.codeUnits),
      Uint8List.fromList('A12B'.codeUnits),
      Uint8List.fromList('A'.codeUnits),
      Uint8List.fromList([0xff, 0xfe, 0x00, 0x41, 0x80, 0x81]),
      Uint8List.fromList(List<int>.filled(5000, 0x41)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 371);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(371)),
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
