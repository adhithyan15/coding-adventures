/// Differential fuzz campaign for `coding_adventures_http_core`,
/// using the in-lane `fuzz` package: hostile byte strings decode
/// into targets, patterns, headers, and versions driving every
/// parser, and the model asserts structural invariants (non-empty
/// paths, exact version round-trips, first-match lookup, padding
/// shape) — with only [InvalidHttpVersion] ever escaping. Fixed
/// seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_http_core/http_core.dart';
import 'package:test/test.dart';

/// Lossy hostile text: raw bytes decoded as Latin-1 so every byte
/// value (including NUL and non-UTF8) reaches the parsers.
String _text(Uint8List bytes, int offset, int length) {
  final end = offset + length > bytes.length ? bytes.length : offset + length;
  return latin1.decode(bytes.sublist(offset, end));
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    if (input.length < 8) {
      return const OutcomePass();
    }
    final target = _text(input, 0, input.length ~/ 2);
    final patternText = _text(input, input.length ~/ 2, input.length);
    cov.recordEdge(target.length % 16);
    cov.recordEdge(20 + (patternText.length % 16));

    // Target decomposition: path never empty; query/fragment split
    // at the first '#' then '?'.
    final parts = parseRequestTarget(target);
    if (parts.path.isEmpty) {
      return const OutcomeCrash('model-divergence', ['empty-path']);
    }
    final hash = target.indexOf('#');
    final head = hash == -1 ? target : target.substring(0, hash);
    final q = head.indexOf('?');
    final wantPath = (q == -1 ? head : head.substring(0, q));
    final wantPathOrRoot = wantPath.isEmpty ? '/' : wantPath;
    if (parts.path != wantPathOrRoot) {
      return const OutcomeCrash('model-divergence', ['path-split']);
    }
    final wantQuery = q == -1 ? null : head.substring(q + 1);
    if (parts.query != wantQuery) {
      return const OutcomeCrash('model-divergence', ['query-split']);
    }
    final wantFragment = hash == -1 ? null : target.substring(hash + 1);
    if (parts.fragment != wantFragment) {
      return const OutcomeCrash('model-divergence', ['fragment-split']);
    }

    // Query pairs: count matches the non-empty &-pieces; values raw.
    final pairs = parts.queryPairs().toList();
    if (wantQuery != null) {
      final pieces = wantQuery.split('&').where((p) => p.isNotEmpty).toList();
      if (pairs.length != pieces.length) {
        return const OutcomeCrash('model-divergence', ['pair-count']);
      }
      for (var i = 0; i < pieces.length; i++) {
        final eq = pieces[i].indexOf('=');
        final wantName = eq == -1 ? pieces[i] : pieces[i].substring(0, eq);
        final wantValue = eq == -1 ? '' : pieces[i].substring(eq + 1);
        if (pairs[i].$1 != wantName || pairs[i].$2 != wantValue) {
          return const OutcomeCrash('model-divergence', ['pair-shape']);
        }
      }
      // First value wins.
      if (pairs.isNotEmpty && parts.queryValue(pairs[0].$1) == null) {
        return const OutcomeCrash('model-divergence', ['query-first']);
      }
    }

    // Route matching over the hostile pattern: a match implies
    // arity agreement; matching the joined path reproduces params.
    final pattern = RoutePattern.parse(patternText);
    final segs = splitPathSegments(target);
    final matched = pattern.matchPath(target);
    if (matched != null) {
      if (pattern.segments.length != segs.length) {
        return const OutcomeCrash('model-divergence', ['match-arity']);
      }
      for (var i = 0; i < segs.length; i++) {
        final segment = pattern.segments[i];
        if (segment is RouteLiteral && segment.value != segs[i]) {
          return const OutcomeCrash('model-divergence', ['match-literal']);
        }
      }
      if (matched.length != pattern.segments.whereType<RouteParam>().length) {
        return const OutcomeCrash('model-divergence', ['match-params']);
      }
    }

    // Headers: first-match ASCII-insensitive lookup.
    final headers = [
      Header('X-A-${input[0]}', 'v0'),
      Header('x-b-${input[1]}', 'v1'),
    ];
    if (findHeader(headers, 'X-A-${input[0]}') != 'v0') {
      return const OutcomeCrash('model-divergence', ['header-first']);
    }
    if (findHeader(headers, 'X-B-${input[1]}') != 'v1') {
      return const OutcomeCrash('model-divergence', ['header-fold']);
    }
    final cl = parseContentLength([Header('Content-Length', '${input[2]}')]);
    if (cl != input[2]) {
      return const OutcomeCrash('model-divergence', ['content-length']);
    }

    // Versions: valid shapes round-trip; anything else raises
    // exactly InvalidHttpVersion.
    final versionText = _text(input, 0, input.length < 12 ? input.length : 12);
    try {
      final version = HttpVersion.parse(versionText);
      if (version.toString() != 'HTTP/${version.major}.${version.minor}') {
        return const OutcomeCrash('model-divergence', ['version-render']);
      }
      // A rendering re-parses to itself.
      final again = HttpVersion.parse(version.toString());
      if (again != version) {
        return const OutcomeCrash('model-divergence', ['version-fixpoint']);
      }
    } on InvalidHttpVersion {
      // Expected vocabulary for hostile version text.
    }

    cov.recordEdge(40 + (pairs.length % 8));
    return const OutcomePass();
  } on InvalidHttpVersion catch (e) {
    return OutcomeCrash('model-divergence', ['escaped-${e.runtimeType}']);
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('/a/b?x=1&y=2#f'.codeUnits),
      Uint8List.fromList('/hello/:name'.codeUnits),
      Uint8List.fromList('HTTP/1.1'.codeUnits),
      Uint8List.fromList(
          [0x2f, 0x00, 0x3f, 0x3d, 0x26, 0x23, 0xff, 0x3a, 0x2e, 0x3b]),
      Uint8List.fromList(List<int>.generate(64, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed
/// 691); longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 128,
          seed: seed ?? BigInt.from(691)),
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
