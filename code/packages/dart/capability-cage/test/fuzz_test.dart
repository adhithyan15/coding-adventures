/// Bounded fuzz campaign for `coding_adventures_capability_cage`, using
/// the in-lane `fuzz` package: hostile JSON documents go through manifest
/// loading (only [ManifestError] may ever escape; successes re-verify
/// every invariant), and hostile glob pairs exercise [matchTarget]
/// totality. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_capability_cage/coding_adventures_capability_cage.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

/// JSON soup tokens: structure, literals, and cage vocabulary.
const _tokens = [
  '{',
  '}',
  '[',
  ']',
  ':',
  ',',
  '"',
  'version',
  '1',
  '2',
  'capabilities',
  'category',
  'action',
  'target',
  'justification',
  'flavor',
  'trust',
  'fs',
  'read',
  'write',
  'net',
  'true',
  'false',
  'null',
  'ingestion',
  'untrusted',
  '*',
  '/',
  'package:',
  ' ',
  'x',
];

/// Total byte-string decoder: every read is bounds-checked (past-end reads
/// yield 0), so the decoder itself can never throw. Documents stay short
/// (bounded nesting), matching the reference parser's untested but
/// recursion-based profile.
class _Decoder {
  final Uint8List input;
  int cursor = 0;

  _Decoder(this.input);

  int takeByte() {
    if (cursor >= input.length) return 0;
    return input[cursor++];
  }

  String takeDocument() {
    final count = takeByte() % 24;
    final buffer = StringBuffer();
    for (var i = 0; i < count; i++) {
      if (takeByte().isEven) {
        buffer.write(_tokens[takeByte() % _tokens.length]);
      } else {
        buffer.writeCharCode(takeByte() % 128);
      }
    }
    return buffer.toString();
  }

  String takeGlob() {
    final count = takeByte() % 10;
    final buffer = StringBuffer();
    for (var i = 0; i < count; i++) {
      switch (takeByte() % 8) {
        case 0:
          buffer.write('*');
        case 1:
          buffer.write('/');
        case 2:
          buffer.write(':');
        case 3:
          buffer.write('\\');
        case 4:
          buffer.write('.');
        default:
          buffer.writeCharCode(0x30 + (takeByte() % 74));
      }
    }
    return buffer.toString();
  }
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final d = _Decoder(input);
    final document = d.takeDocument();
    final pattern = d.takeGlob();
    final candidate = d.takeGlob();
    cov.recordEdge(document.length % 16);

    // Manifest loading: only ManifestError escapes; successes re-verify.
    try {
      final manifest = Manifest.loadFromString(document);
      cov.recordEdge(20);
      for (final cap in manifest.capabilities) {
        if (!isValidCombination(cap.category, cap.action)) {
          return const OutcomeCrash('invariant', ['invalid-pair-loaded']);
        }
        if (cap.target.isEmpty) {
          return const OutcomeCrash('invariant', ['empty-target-loaded']);
        }
      }
      // A loaded manifest re-validates cleanly (idempotent check).
      try {
        manifest.validateReadWriteSeparation();
      } on ManifestError {
        return const OutcomeCrash('invariant', ['loaded-manifest-rejected']);
      }
      final summary = manifest.capabilitySurfaceSummary();
      if (summary.totalCapabilities != manifest.capabilities.length) {
        return const OutcomeCrash('invariant', ['summary-count-mismatch']);
      }
      // Every declared triple is covered by has().
      for (final cap in manifest.capabilities) {
        if (!manifest.has(cap.category, cap.action, cap.target)) {
          return const OutcomeCrash('invariant', ['declared-triple-denied']);
        }
      }
    } on ManifestError {
      cov.recordEdge(21);
    }

    // Glob matching is total over hostile pairs.
    final matched = matchTarget(pattern, candidate);
    cov.recordEdge(30 + (matched ? 1 : 0));
    if (matchTarget(pattern, pattern) != true) {
      return const OutcomeCrash('invariant', ['glob-self-match-failed']);
    }
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('{"version": 1, "capabilities": []}'.codeUnits),
      Uint8List.fromList(
          '{"version": 1, "capabilities": [{"category": "fs", "action": "read", "target": "./x"}]}'
              .codeUnits),
      Uint8List.fromList([123, 34, 118, 255, 254, 125]),
      Uint8List.fromList('[[[[[[[[[[}}}}}}}}]]]]'.codeUnits),
      Uint8List.fromList('**/..\\*:%'.codeUnits),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 191);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(191)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: total, invariants hold, no crashes', () {
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
