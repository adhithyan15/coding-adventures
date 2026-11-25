/// Hostile-input fuzz campaign for `coding_adventures_http1`, using the
/// in-lane `fuzz` package: raw byte strings drive both parsers, and the
/// model asserts the fail-closed vocabulary (only [Http1ParseError] ever
/// escapes), body-offset integrity, header shape, and framing consistency.
/// Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_http1/http1.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  cov.recordEdge(input.length % 64);
  for (final request in [true, false]) {
    cov.recordEdge(request ? 70 : 71);
    try {
      if (request) {
        final parsed = parseRequestHead(input);
        if (!_checkRequest(input, parsed, cov)) {
          return const OutcomeCrash('model-divergence', ['request-model']);
        }
      } else {
        final parsed = parseResponseHead(input);
        if (!_checkResponse(input, parsed, cov)) {
          return const OutcomeCrash('model-divergence', ['response-model']);
        }
      }
      cov.recordEdge(request ? 72 : 73);
    } on Http1ParseError {
      cov.recordEdge(request ? 74 : 75);
    } catch (e) {
      return OutcomeCrash('unexpected-escape', ['${e.runtimeType}']);
    }
  }
  cov.recordEdge(80 + (input.length % 8));
  return const OutcomePass();
}

bool _checkRequest(
    Uint8List input, ParsedRequestHead parsed, CoverageBitmap cov) {
  // The body offset splits head from body.
  if (parsed.bodyOffset < 0 || parsed.bodyOffset > input.length) return false;
  final headBytes = input.sublist(0, parsed.bodyOffset);
  // The head ends at a blank line.
  final text = String.fromCharCodes(headBytes);
  if (!text.endsWith('\n\n') &&
      !text.endsWith('\r\n\r\n') &&
      headBytes.isNotEmpty) {
    // LF/CRLF mixes still end at some newline pair; the offset must at
    // least sit on a line boundary.
    if (parsed.bodyOffset > 0 && input[parsed.bodyOffset - 1] != 0x0A) {
      return false;
    }
  }
  cov.recordEdge(90);
  // Headers are well-formed: non-empty names.
  for (final header in parsed.head.headers) {
    if (header.name.isEmpty) return false;
  }
  // The summary agrees with the parse.
  final summary = parsed.summary();
  if (summary.method != parsed.head.method) return false;
  if (summary.targetLen != utf8.encode(parsed.head.target).length) return false;
  if (summary.headerCount != parsed.head.headers.length) return false;
  if (summary.bodyOffset != parsed.bodyOffset) return false;
  if (summary.bodyKind != parsed.bodyKind) return false;
  cov.recordEdge(91);
  return true;
}

bool _checkResponse(
    Uint8List input, ParsedResponseHead parsed, CoverageBitmap cov) {
  if (parsed.bodyOffset < 0 || parsed.bodyOffset > input.length) return false;
  if (parsed.bodyOffset > 0 && input[parsed.bodyOffset - 1] != 0x0A) {
    return false;
  }
  for (final header in parsed.head.headers) {
    if (header.name.isEmpty) return false;
  }
  if (parsed.head.status < 0 || parsed.head.status > 65535) return false;
  final summary = parsed.summary();
  if (summary.status != parsed.head.status) return false;
  if (summary.reasonLen != utf8.encode(parsed.head.reason).length) return false;
  if (summary.headerCount != parsed.head.headers.length) return false;
  if (summary.bodyKind != parsed.bodyKind) return false;
  cov.recordEdge(92);
  return true;
}

void main() {
  test('hostile campaign finds no escapes or divergences', () {
    Uint8List b(String s) => Uint8List.fromList(utf8.encode(s));
    final seeds = <Uint8List>[
      Uint8List(0),
      Uint8List.fromList([0xFF, 0xFE, 0x0A, 0x0A]),
      b('GET / HTTP/1.1\r\nHost: example.com\r\n\r\n'),
      b('POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello'),
      b('HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody'),
      b('\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload'),
      b('HTTP/1.1 204 No Content\r\nTransfer-Encoding: chunked\r\n\r\n'),
      b('GET / HTTP/1.1\r\nHost example.com\r\n\r\n'),
      b('HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n'),
      b('GET / HTTP/1.1'),
      b('\r\n\r\n'),
      b('HTTP/1.1 99999\r\n\r\n'),
      b(':\r\n\r\n'),
      b('GET\x85/\x85HTTP/1.1\r\n\r\n'),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(1208)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
