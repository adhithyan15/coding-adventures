/// Bounded fuzz campaign for `coding_adventures_url_parser`, using the
/// in-lane `fuzz` package: hostile strings (delimiters, escapes, lone
/// surrogates, astral characters) go through parse/resolve/canonicalize and
/// the percent codec. Only [UrlError] may ever escape; successful parses
/// must satisfy the serialize fixed point and canonicalization
/// idempotence. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_url_parser/coding_adventures_url_parser.dart';
import 'package:test/test.dart';

final _tokens = [
  'http',
  'https',
  'ftp',
  'mailto',
  'gopher',
  '1x',
  '://',
  ':',
  '/',
  '?',
  '#',
  '@',
  '[',
  ']',
  '%',
  '%2F',
  '%7e',
  '%GG',
  '%2',
  '%FF',
  '.',
  '..',
  '...',
  'a',
  'Z',
  '0',
  '9',
  ' ',
  '-',
  '_',
  '+',
  '~',
  String.fromCharCode(0xD800), // lone high surrogate
  String.fromCharCode(0xDC00), // lone low surrogate
  String.fromCharCode(0x1F600), // astral smile
  '日本',
  'café',
];

/// Total byte-string decoder: every read is bounds-checked (past-end reads
/// yield 0), so the decoder itself can never throw.
class _Decoder {
  final Uint8List input;
  int cursor = 0;

  _Decoder(this.input);

  int takeByte() {
    if (cursor >= input.length) return 0;
    return input[cursor++];
  }

  /// One hostile string: vocabulary tokens mixed with raw code units (the
  /// full 0-0xFFFF range, so lone surrogates stay reachable).
  String takeString() {
    final count = takeByte() % 12;
    final buffer = StringBuffer();
    for (var i = 0; i < count; i++) {
      if (takeByte().isEven) {
        buffer.write(_tokens[takeByte() % _tokens.length]);
      } else {
        final first = takeByte();
        if (first < 128) {
          buffer.writeCharCode(first);
        } else {
          buffer.writeCharCode((first << 8) | takeByte());
        }
      }
    }
    return buffer.toString();
  }
}

bool _sameComponents(Url a, Url b) =>
    a.scheme == b.scheme &&
    a.userinfo == b.userinfo &&
    a.host == b.host &&
    a.port == b.port &&
    a.path == b.path &&
    a.query == b.query &&
    a.fragment == b.fragment;

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final d = _Decoder(input);
    final text = d.takeString();
    final rel = d.takeString();
    cov.recordEdge(text.length % 16);

    // Percent codec pairing: encode and decode agree on lone surrogates,
    // and decode inverts encode whenever encoding succeeds.
    var encoded = '';
    var encodeThrew = false;
    try {
      encoded = percentEncode(text);
    } on UrlError {
      encodeThrew = true;
    }
    if (encodeThrew) {
      var decodeThrew = false;
      try {
        percentDecode(text);
      } on UrlError {
        decodeThrew = true;
      }
      if (!decodeThrew) {
        return const OutcomeCrash(
            'codec-pairing', ['encode-threw-decode-passed']);
      }
    } else if (percentDecode(encoded) != text) {
      return const OutcomeCrash('codec-round-trip', ['decode-encode-mismatch']);
    }
    cov.recordEdge(20 + (encodeThrew ? 1 : 0));

    Url? url;
    try {
      url = Url.parse(text);
    } on UrlError catch (e) {
      cov.recordEdge(30 + e.index);
      return const OutcomePass();
    }
    cov.recordEdge(40);

    // Serialize fixed point (except the known lossy no-host corner).
    final lossy =
        url.host == null && (url.userinfo != null || url.port != null);
    if (!lossy) {
      final reparsed = Url.parse(url.toUrlString());
      if (!_sameComponents(url, reparsed)) {
        return const OutcomeCrash('fixed-point', ['reparse-mismatch']);
      }
    }
    cov.recordEdge(50);

    // Canonicalization: total, and idempotent.
    try {
      final once = url.canonicalize();
      if (once.canonicalize() != once) {
        return const OutcomeCrash('idempotence', ['canon-not-idempotent']);
      }
    } on UrlError {
      cov.recordEdge(60);
    }

    // Resolution against any base with any reference: total.
    try {
      url.resolve(rel);
      cov.recordEdge(70);
    } on UrlError {
      cov.recordEdge(71);
    }
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList('http://h/a/../b?q#f'.codeUnits),
      Uint8List.fromList('mailto:a@b'.codeUnits),
      Uint8List.fromList([0x25, 0x25, 0x25, 0x25]),
      Uint8List.fromList([0x2e, 0x2e, 0x2f, 0x2e, 0x2e]),
      Uint8List.fromList(
          [0x5b, 0x3a, 0x3a, 0x31, 0x5d, 0x3a, 0x38, 0x30, 0x38, 0x30]),
      Uint8List.fromList([0xfe, 0xff, 0x00, 0x80, 0xd8, 0xdc]),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 68); longer
/// ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(68)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: total, fixed points hold, no crashes', () {
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
