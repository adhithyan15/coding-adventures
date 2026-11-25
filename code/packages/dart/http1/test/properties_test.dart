/// Property tests for `coding_adventures_http1`, using the in-lane `pbt`
/// package: the fail-closed vocabulary over hostile bytes, request and
/// response round-trips over generated messages, the framing decision
/// table, body-offset integrity, summary redaction, and query purity —
/// all with fixed seeds.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_http1/http1.dart';
import 'package:coding_adventures_http_core/http_core.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

/// Visible ASCII without whitespace, colon, or comma (safe in tokens,
/// targets, header names, and single reason words).
Gen<String> tokenGen(int minLen, int maxLen) {
  const alphabet =
      'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~!#\$%&\'*+^`|/';
  return Gen.list(minLen, maxLen,
          Gen.integer(BigInt.zero, BigInt.from(alphabet.length - 1)))
      .map((idx) => idx.map((i) => alphabet[i.toInt()]).join());
}

Gen<String> methodGen() => Gen.oneOf([
      Gen.constant('GET'),
      Gen.constant('POST'),
      Gen.constant('PUT'),
      Gen.constant('DELETE'),
      Gen.constant('PATCH'),
      Gen.constant('HEAD'),
      Gen.constant('OPTIONS'),
      tokenGen(1, 8).map((t) => t.toUpperCase()),
    ]);

Gen<String> versionGen() => Gen.tuple2(
      Gen.integer(BigInt.zero, BigInt.from(9)),
      Gen.integer(BigInt.zero, BigInt.from(9)),
    ).map((v) => 'HTTP/${v.$1}.${v.$2}');

Gen<(String, String)> headerGen() =>
    Gen.tuple2(tokenGen(1, 12), tokenGen(0, 20));

Uint8List requestBytes(String method, String target, String version,
    List<(String, String)> headers, String body) {
  final buf = StringBuffer('$method $target $version\r\n');
  for (final (name, value) in headers) {
    buf.write('$name: $value\r\n');
  }
  buf.write('\r\n$body');
  return Uint8List.fromList(utf8.encode(buf.toString()));
}

void main() {
  group('properties', () {
    test('parsers fail closed on hostile bytes', () {
      check(
        'vocabulary',
        checkProperty<Uint8List>(
          BigInt.from(1201),
          PbtOptions(numTests: 300, seed: BigInt.from(1201)),
          (rng) => Gen.bytes(0, 200).sample(rng),
          (input) {
            final List<Object Function(List<int>)> parses = [
              parseRequestHead,
              parseResponseHead,
            ];
            for (final parse in parses) {
              try {
                final parsed = parse(input);
                // Summaries never throw on parsed output.
                if (parsed is ParsedRequestHead) {
                  parsed.summary();
                } else {
                  (parsed as ParsedResponseHead).summary();
                }
              } on Http1ParseError {
                // Expected vocabulary.
              } catch (_) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('requests round-trip', () {
      final gen = Gen.tuple2(
        Gen.tuple3(
            methodGen(), tokenGen(1, 24).map((t) => '/$t'), versionGen()),
        Gen.tuple2(Gen.list(0, 6, headerGen()), tokenGen(0, 32)),
      );
      check(
        'request round-trip',
        checkProperty<
            ((String, String, String), (List<(String, String)>, String))>(
          BigInt.from(1202),
          PbtOptions(numTests: 150, seed: BigInt.from(1202)),
          (rng) => gen.sample(rng),
          (input) {
            final (method, target, version) = input.$1;
            final (headers, body) = input.$2;
            final raw = requestBytes(method, target, version, headers, body);
            final parsed = parseRequestHead(raw);
            if (parsed.head.method != method) return false;
            if (parsed.head.target != target) return false;
            if (parsed.head.version.toString() != version) return false;
            if (parsed.head.headers.length != headers.length) return false;
            for (var i = 0; i < headers.length; i++) {
              if (parsed.head.headers[i].name != headers[i].$1) return false;
              if (parsed.head.headers[i].value != headers[i].$2) return false;
            }
            // The body offset finds the body bytes exactly.
            final rest = utf8.decode(raw.sublist(parsed.bodyOffset));
            if (rest != body) return false;
            return true;
          },
        ),
      );
    });

    test('responses round-trip', () {
      final gen = Gen.tuple2(
        Gen.tuple3(
          versionGen(),
          Gen.integer(BigInt.zero, BigInt.from(599)),
          Gen.list(0, 4, tokenGen(1, 8)),
        ),
        Gen.tuple2(Gen.list(0, 6, headerGen()), tokenGen(0, 32)),
      );
      check(
        'response round-trip',
        checkProperty<
            ((String, BigInt, List<String>), (List<(String, String)>, String))>(
          BigInt.from(1203),
          PbtOptions(numTests: 150, seed: BigInt.from(1203)),
          (rng) => gen.sample(rng),
          (input) {
            final (version, status, reasonWords) = input.$1;
            final (headers, body) = input.$2;
            final reason = reasonWords.join(' ');
            final buf = StringBuffer(
                '$version ${status.toInt()}${reason.isEmpty ? '' : ' $reason'}\r\n');
            for (final (name, value) in headers) {
              buf.write('$name: $value\r\n');
            }
            buf.write('\r\n$body');
            final raw = Uint8List.fromList(utf8.encode(buf.toString()));
            final parsed = parseResponseHead(raw);
            if (parsed.head.version.toString() != version) return false;
            if (parsed.head.status != status.toInt()) return false;
            if (parsed.head.reason != reason) return false;
            if (parsed.head.headers.length != headers.length) return false;
            final rest = utf8.decode(raw.sublist(parsed.bodyOffset));
            if (rest != body) return false;
            return true;
          },
        ),
      );
    });

    test('framing follows the decision table', () {
      // Random statuses against random framing headers: bodyless always
      // None, chunked always wins, zero always None, and the request /
      // response defaults differ exactly as specified.
      final framingGen = Gen.oneOf([
        Gen.constant(<(String, String)>[]),
        Gen.constant(const [('Content-Length', '0')]),
        Gen.integer(BigInt.from(1), BigInt.from(9999))
            .map((n) => [('Content-Length', n.toString())]),
        Gen.constant(const [('Transfer-Encoding', 'chunked')]),
        Gen.constant(const [
          ('Content-Length', '12'),
          ('Transfer-Encoding', 'gzip, chunked'),
        ]),
      ]);
      check(
        'framing table',
        checkProperty<(BigInt, List<(String, String)>)>(
          BigInt.from(1204),
          PbtOptions(numTests: 200, seed: BigInt.from(1204)),
          (rng) => Gen.tuple2(
            Gen.integer(BigInt.zero, BigInt.from(599)),
            framingGen,
          ).sample(rng),
          (input) {
            final status = input.$1.toInt();
            final headers = input.$2;
            final buf = StringBuffer('HTTP/1.1 $status R\r\n');
            for (final (name, value) in headers) {
              buf.write('$name: $value\r\n');
            }
            buf.write('\r\n');
            final kind = parseResponseHead(
                    Uint8List.fromList(utf8.encode(buf.toString())))
                .bodyKind;
            final bodyless = (status >= 100 && status < 200) ||
                status == 204 ||
                status == 304;
            final chunked = headers.any(
                (h) => h.$1 == 'Transfer-Encoding' && h.$2.contains('chunked'));
            final lengths = [
              for (final h in headers)
                if (h.$1 == 'Content-Length') int.parse(h.$2)
            ];
            if (bodyless) return kind == const BodyNone();
            if (chunked) return kind == const BodyChunked();
            if (lengths.isEmpty) return kind == const BodyUntilEof();
            if (lengths.first == 0) return kind == const BodyNone();
            return kind == BodyContentLength(lengths.first);
          },
        ),
      );
    });

    test('requests without framing headers have no body', () {
      check(
        'request default',
        checkProperty<List<(String, String)>>(
          BigInt.from(1205),
          PbtOptions(numTests: 120, seed: BigInt.from(1205)),
          (rng) => Gen.list(0, 6, headerGen()).sample(rng),
          (headers) {
            // Generated names never collide with the framing headers
            // (they lack the hyphen... no — they CAN contain hyphens,
            // so filter to names that cannot match).
            final safe = headers
                .where((h) =>
                    h.$1.toLowerCase() != 'content-length' &&
                    h.$1.toLowerCase() != 'transfer-encoding')
                .toList();
            final parsed = parseRequestHead(
                requestBytes('GET', '/x', 'HTTP/1.1', safe, ''));
            return parsed.bodyKind == const BodyNone();
          },
        ),
      );
    });

    test('summaries never leak secrets', () {
      check(
        'redaction',
        checkProperty<(String, List<(String, String)>, String)>(
          BigInt.from(1206),
          PbtOptions(numTests: 120, seed: BigInt.from(1206)),
          (rng) => Gen.tuple3(
            tokenGen(16, 32),
            Gen.list(1, 4, headerGen()),
            tokenGen(16, 32),
          ).sample(rng),
          (input) {
            final (secret, headers, secret2) = input;
            final target = '/pair?token=$secret';
            final request = parseRequestHead(requestBytes(
                'POST', target, 'HTTP/1.1', headers, 'body-$secret2'));
            final text = request.summary().toString();
            if (text.contains(secret)) return false;
            // Long values cannot collide coincidentally with the
            // summary's fixed shape; short ones could, so only long
            // ones carry the redaction check. (Unit tests pin names.)
            for (final (_, value) in headers) {
              if (value.length >= 12 && text.contains(value)) return false;
            }
            if (text.contains(secret2)) return false;
            return true;
          },
        ),
      );
    });

    test('queries are pure', () {
      check(
        'purity',
        checkProperty<Uint8List>(
          BigInt.from(1207),
          PbtOptions(numTests: 120, seed: BigInt.from(1207)),
          (rng) => Gen.bytes(0, 200).sample(rng),
          (input) {
            for (var round = 0; round < 2; round++) {
              try {
                final a = parseRequestHead(input);
                final b = parseRequestHead(input);
                if (a != b || a.summary() != b.summary()) return false;
              } on Http1ParseError catch (e1) {
                try {
                  parseRequestHead(input);
                  return false;
                } on Http1ParseError catch (e2) {
                  if (e1 != e2) return false;
                }
              }
            }
            return true;
          },
        ),
      );
    });
  });
}
