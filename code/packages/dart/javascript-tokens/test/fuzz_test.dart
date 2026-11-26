/// Hostile-input fuzz campaign for
/// `coding_adventures_javascript_tokens`, using the in-lane `fuzz`
/// package: raw bytes decode (malformed sequences become U+FFFD) and
/// drive version parsing, span construction, and kind wrapping.
/// [EsVersion.tryParse] must never throw; a parsed version must
/// re-emit its input; span laws must hold for derived ranges. Fixed
/// seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_javascript_tokens/javascript_tokens.dart';
import 'package:test/test.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  final parsed = EsVersion.tryParse(text);
  if (parsed != null) {
    cov.recordEdge(100 + (parsed.index % 14));
    if (parsed.asStr() != text) {
      return const OutcomeCrash('model-divergence', ['round-trip']);
    }
    try {
      final hard = EsVersion.parse(text);
      if (hard != parsed) {
        return const OutcomeCrash('model-divergence', ['twin-disagree']);
      }
    } on UnknownEsVersion {
      return const OutcomeCrash('model-divergence', ['twin-disagree']);
    }
  } else {
    try {
      EsVersion.parse(text);
      return const OutcomeCrash('model-divergence', ['twin-disagree']);
    } on UnknownEsVersion catch (e) {
      cov.recordEdge(120 + (e.message.length % 8));
      if (!e.message.contains('"$text"')) {
        return const OutcomeCrash('model-divergence', ['error-shape']);
      }
    }
  }
  // Derive a span range from the input bytes and check the laws.
  var a = 0;
  var b = 0;
  for (final byte in input) {
    a = (a + byte) & 0xFFFFFFFF;
    b = (b * 33 + byte) & 0xFFFFFFFF;
  }
  final lo = a < b ? a : b;
  final hi = a < b ? b : a;
  final span = Span(lo, hi);
  cov.recordEdge(200 + (span.length % 16));
  if (span.length != hi - lo || span.isEmpty != (lo == hi)) {
    return const OutcomeCrash('model-divergence', ['span-laws']);
  }
  final kind = TokenKindOther(text);
  if (kind != TokenKindOther(text) || kind.isEof || kind.isTrivia) {
    return const OutcomeCrash('model-divergence', ['other-laws']);
  }
  return const OutcomePass();
}

void main() {
  test('fuzz hostile bytes fail closed and stay consistent', () {
    final seeds = [
      Uint8List(0),
      Uint8List.fromList([0]),
      Uint8List.fromList(utf8.encode('es2025')),
      Uint8List.fromList(utf8.encode('es5.1\nlatest\nES2025\n')),
      Uint8List.fromList(utf8.encode('es1')),
      Uint8List.fromList(utf8.encode(' ')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(6310)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
