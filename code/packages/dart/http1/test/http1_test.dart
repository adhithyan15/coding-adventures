/// Unit tests for `coding_adventures_http1`: the 10 reference tests
/// (`code/packages/rust/http1/src/lib.rs`, tests module) plus the extras
/// this lane needs — the full error matrix, byte-vs-char lengths, the
/// exact-Unicode-whitespace behavior, BOM preservation, framing
/// precedence, and value semantics.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_http1/http1.dart';
import 'package:coding_adventures_http_core/http_core.dart';
import 'package:test/test.dart';

Uint8List bytes(String text) => Uint8List.fromList(utf8.encode(text));

void main() {
  group('requests', () {
    test('parses a simple request', () {
      final parsed = parseRequestHead(
          bytes('GET / HTTP/1.0\r\nHost: example.com\r\n\r\n'));
      expect(parsed.head.method, 'GET');
      expect(parsed.head.target, '/');
      expect(parsed.head.version, const HttpVersion(major: 1, minor: 0));
      expect(parsed.head.headers.length, 1);
      expect(parsed.bodyKind, const BodyNone());
    });

    test('parses a content-length request', () {
      final parsed = parseRequestHead(
          bytes('POST /submit HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello'));
      expect(parsed.bodyOffset, 44);
      expect(parsed.bodyKind, const BodyContentLength(5));
    });

    test('empty input is an incomplete head', () {
      expect(() => parseRequestHead(Uint8List(0)),
          throwsA(const IncompleteHead()));
    });

    test('blank lines alone are an incomplete head', () {
      expect(() => parseRequestHead(bytes('\r\n')),
          throwsA(const IncompleteHead()));
    });

    test('unterminated head is an incomplete head', () {
      expect(() => parseRequestHead(bytes('GET / HTTP/1.1\r\nHost: x')),
          throwsA(const IncompleteHead()));
    });

    test('all-blank input is an incomplete head', () {
      // The leading-blank skip consumes the whole input, so the head
      // never starts. (The empty-lines arm this guards is unreachable:
      // post-skip, the first content line always holds a byte.)
      expect(() => parseRequestHead(bytes('\r\n\r\n')),
          throwsA(const IncompleteHead()));
    });

    test('start line must have exactly three parts', () {
      expect(() => parseRequestHead(bytes('GET / HTTP/1.1 extra\r\n\r\n')),
          throwsA(isA<InvalidStartLine>()));
      expect(() => parseRequestHead(bytes('GET /\r\n\r\n')),
          throwsA(isA<InvalidStartLine>()));
      expect(() => parseRequestHead(bytes('GET\r\n\r\n')),
          throwsA(isA<InvalidStartLine>()));
    });

    test('bad version names the version text', () {
      expect(() => parseRequestHead(bytes('GET / FOO/1.1\r\n\r\n')),
          throwsA(const InvalidVersion('FOO/1.1')));
    });

    test('leading CRLF blanks are skipped', () {
      final parsed =
          parseRequestHead(bytes('\r\n\r\nGET / HTTP/1.0\r\n\r\nbody-bytes'));
      expect(parsed.head.method, 'GET');
      expect(parsed.bodyOffset, greaterThan(0));
      expect(
          utf8.decode((Uint8List.fromList(
                  utf8.encode('\r\n\r\nGET / HTTP/1.0\r\n\r\nbody-bytes')))
              .sublist(parsed.bodyOffset)),
          'body-bytes');
    });

    test('lone carriage return does not start a blank line', () {
      // `\r` without `\n` breaks the blank-line skip, so it lands
      // mid-line — where `\r` is ordinary whitespace and the request
      // still parses.
      final parsed = parseRequestHead(bytes('\rGET / HTTP/1.1\r\n\r\n'));
      expect(parsed.head.method, 'GET');
      expect(parsed.head.target, '/');
    });

    test('padded start line parses', () {
      final parsed = parseRequestHead(bytes('  GET  /  HTTP/1.1  \r\n\r\n'));
      expect(parsed.head.method, 'GET');
      expect(parsed.head.target, '/');
    });

    test('tab-separated start line parses', () {
      final parsed = parseRequestHead(bytes('GET\t/\tHTTP/1.1\r\n\r\n'));
      expect(parsed.head.method, 'GET');
    });

    test('every Unicode whitespace separates the start line', () {
      // The exact White_Space set: NEL, NBSP, OGHAM, EN QUAD..HAIR
      // SPACE, LS, PS, NNBSP, MMSP, IDEOGRAPHIC SPACE.
      const spaces = [
        0x85,
        0xA0,
        0x1680,
        0x2000,
        0x2001,
        0x2002,
        0x2003,
        0x2004,
        0x2005,
        0x2006,
        0x2007,
        0x2008,
        0x2009,
        0x200A,
        0x2028,
        0x2029,
        0x202F,
        0x205F,
        0x3000,
      ];
      for (final space in spaces) {
        final sep = String.fromCharCode(space);
        final parsed =
            parseRequestHead(bytes('GET${sep}/${sep}HTTP/1.1\r\n\r\n'));
        expect(parsed.head.method, 'GET',
            reason: 'U+${space.toRadixString(16)}');
        expect(parsed.head.target, '/', reason: 'U+${space.toRadixString(16)}');
      }
    });

    test('BOM is preserved like from_utf8, not stripped', () {
      // `utf8.decode` strips a leading BOM; the reference keeps it as
      // U+FEFF inside the first token — and U+FEFF is not whitespace,
      // so it must not separate either.
      final raw = Uint8List.fromList(
          [0xEF, 0xBB, 0xBF, ...utf8.encode('GET / HTTP/1.1\r\n\r\n')]);
      final parsed = parseRequestHead(raw);
      expect(parsed.head.method, '\uFEFFGET');
      expect(parsed.head.target, '/');
    });

    test('BOM plus malformed bytes is an encoding error', () {
      final raw = Uint8List.fromList([0xEF, 0xBB, 0xBF, 0xFF, 0x0A, 0x0A]);
      expect(() => parseRequestHead(raw), throwsA(const InvalidHeadEncoding()));
    });

    test('invalid start-line bytes are an encoding error', () {
      final raw = Uint8List.fromList([0xFF, 0x0A, 0x0A]);
      expect(() => parseRequestHead(raw), throwsA(const InvalidHeadEncoding()));
    });

    test('invalid header bytes are an encoding error', () {
      final raw = Uint8List.fromList([
        ...utf8.encode('GET / HTTP/1.1\r\nX: '),
        0xFF,
        ...utf8.encode('\r\n\r\n')
      ]);
      expect(() => parseRequestHead(raw), throwsA(const InvalidHeadEncoding()));
    });
  });

  group('responses', () {
    test('parses a response and its reason', () {
      final parsed = parseResponseHead(
          bytes('HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nbody'));
      expect(parsed.head.status, 200);
      expect(parsed.head.reason, 'OK');
      expect(parsed.bodyKind, const BodyContentLength(4));
    });

    test('response without length reads until EOF', () {
      final parsed = parseResponseHead(
          bytes('HTTP/1.0 200 OK\r\nServer: Venture\r\n\r\n'));
      expect(parsed.bodyKind, const BodyUntilEof());
    });

    test('bodyless status codes override headers', () {
      final parsed = parseResponseHead(
          bytes('HTTP/1.1 204 No Content\r\nContent-Length: 12\r\n\r\n'));
      expect(parsed.bodyKind, const BodyNone());
    });

    test('1xx and 304 are bodyless too', () {
      for (final status in [100, 199]) {
        final parsed = parseResponseHead(
            bytes('HTTP/1.1 $status Maybe\r\nContent-Length: 9\r\n\r\n'));
        expect(parsed.bodyKind, const BodyNone(), reason: '$status');
      }
      final notModified = parseResponseHead(
          bytes('HTTP/1.1 304 Not Modified\r\nContent-Length: 9\r\n\r\n'));
      expect(notModified.bodyKind, const BodyNone());
    });

    test('bodyless wins over chunked', () {
      final parsed = parseResponseHead(bytes(
          'HTTP/1.1 204 No Content\r\nTransfer-Encoding: chunked\r\n\r\n'));
      expect(parsed.bodyKind, const BodyNone());
    });

    test('accepts LF-only lines and duplicate headers', () {
      final parsed = parseResponseHead(bytes(
          '\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload'));
      expect(parsed.head.headers.length, 2);
      expect(parsed.head.headers[0].value, 'a=1');
      expect(parsed.head.headers[1].value, 'b=2');
    });

    test('body offset points at the payload', () {
      final raw =
          '\nHTTP/1.1 200 OK\nSet-Cookie: a=1\nSet-Cookie: b=2\n\npayload';
      final parsed = parseResponseHead(bytes(raw));
      expect(utf8.decode(bytes(raw).sublist(parsed.bodyOffset)), 'payload');
    });

    test('status line needs version and status', () {
      expect(() => parseResponseHead(bytes('HTTP/1.1\r\n\r\n')),
          throwsA(isA<InvalidStartLine>()));
    });

    test('bad version names the version text', () {
      expect(() => parseResponseHead(bytes('HTTP/9.9.9 200 OK\r\n\r\n')),
          throwsA(const InvalidVersion('HTTP/9.9.9')));
    });

    test('bad statuses name the status text', () {
      for (final status in ['OK', '99999', '1000000', '+200', '200x']) {
        expect(() => parseResponseHead(bytes('HTTP/1.1 $status\r\n\r\n')),
            throwsA(InvalidStatus(status)),
            reason: status);
      }
    });

    test('status accepts leading zeros', () {
      final parsed = parseResponseHead(bytes('HTTP/1.1 0200 OK\r\n\r\n'));
      expect(parsed.head.status, 200);
    });

    test('missing reason reads empty', () {
      final parsed = parseResponseHead(bytes('HTTP/1.1 200\r\n\r\n'));
      expect(parsed.head.reason, '');
    });

    test('reason whitespace runs normalize to single spaces', () {
      final parsed = parseResponseHead(bytes('HTTP/1.1 200  A   B\tC\r\n\r\n'));
      expect(parsed.head.reason, 'A B C');
    });

    test('empty input is an incomplete head', () {
      expect(() => parseResponseHead(Uint8List(0)),
          throwsA(const IncompleteHead()));
    });

    test('all-blank input is an incomplete head', () {
      expect(() => parseResponseHead(bytes('\n\n')),
          throwsA(const IncompleteHead()));
    });
  });

  group('headers', () {
    test('rejects headers without a colon', () {
      expect(
          () => parseRequestHead(
              bytes('GET / HTTP/1.1\r\nHost example.com\r\n\r\n')),
          throwsA(const InvalidHeader('Host example.com')));
    });

    test('rejects empty header names', () {
      expect(() => parseRequestHead(bytes('GET / HTTP/1.1\r\n: value\r\n\r\n')),
          throwsA(const InvalidHeader(': value')));
      expect(
          () => parseRequestHead(bytes('GET / HTTP/1.1\r\n   : value\r\n\r\n')),
          throwsA(const InvalidHeader('   : value')));
    });

    test('header names trim Unicode whitespace', () {
      final parsed = parseRequestHead(
          bytes('GET / HTTP/1.1\r\n\u00A0Host\u2003: x\r\n\r\n'));
      expect(parsed.head.headers.single.name, 'Host');
    });

    test('header values trim spaces and tabs only', () {
      final parsed = parseRequestHead(
          bytes('GET / HTTP/1.1\r\nX: \t a b \t \r\nY:\r\nZ:v\r\n\r\n'));
      expect(parsed.head.headers[0].value, 'a b');
      expect(parsed.head.headers[1].value, '');
      expect(parsed.head.headers[2].value, 'v');
    });

    test('all-whitespace value reads empty', () {
      final parsed =
          parseRequestHead(bytes('GET / HTTP/1.1\r\nX: \t \r\n\r\n'));
      expect(parsed.head.headers.single.value, '');
    });

    test('value splits at the first colon', () {
      final parsed =
          parseRequestHead(bytes('GET / HTTP/1.1\r\nX: a:b:c\r\n\r\n'));
      expect(parsed.head.headers.single.value, 'a:b:c');
    });

    test('BOM in a header name is preserved', () {
      final raw = Uint8List.fromList([
        ...utf8.encode('GET / HTTP/1.1\r\n'),
        0xEF,
        0xBB,
        0xBF,
        ...utf8.encode('Host: x\r\n\r\n'),
      ]);
      final parsed = parseRequestHead(raw);
      expect(parsed.head.headers.single.name, '\uFEFFHost');
    });
  });

  group('framing', () {
    test('rejects invalid content lengths', () {
      expect(
          () => parseResponseHead(
              bytes('HTTP/1.1 200 OK\r\nContent-Length: nope\r\n\r\n')),
          throwsA(const InvalidContentLength('nope')));
    });

    test('content length rejects signs, empties, and overflow', () {
      for (final value in ['+5', '', '99999999999999999999999']) {
        expect(
            () => parseResponseHead(
                bytes('HTTP/1.1 200 OK\r\nContent-Length: $value\r\n\r\n')),
            throwsA(InvalidContentLength(value)),
            reason: '[$value]');
      }
    });

    test('content length accepts leading zeros', () {
      final parsed = parseResponseHead(
          bytes('HTTP/1.1 200 OK\r\nContent-Length: 007\r\n\r\n'));
      expect(parsed.bodyKind, const BodyContentLength(7));
    });

    test('first content length wins', () {
      final parsed = parseResponseHead(bytes(
          'HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Length: 9\r\n\r\n'));
      expect(parsed.bodyKind, const BodyContentLength(5));
    });

    test('content length matches ASCII case-insensitively', () {
      final parsed = parseResponseHead(
          bytes('HTTP/1.1 200 OK\r\ncOnTeNt-LeNgTh: 3\r\n\r\n'));
      expect(parsed.bodyKind, const BodyContentLength(3));
    });

    test('Kelvin sign does not fold to k', () {
      // `toLowerCase` would equate U+212A with `k`; the reference's
      // ASCII-only comparison must not.
      final parsed = parseResponseHead(
          bytes('HTTP/1.1 200 OK\r\n\u212Aontent-Length: 3\r\n\r\n'));
      expect(parsed.bodyKind, const BodyUntilEof());
    });

    test('zero content length means no body', () {
      final request = parseRequestHead(
          bytes('POST /x HTTP/1.1\r\nContent-Length: 0\r\n\r\n'));
      expect(request.bodyKind, const BodyNone());
      final response = parseResponseHead(
          bytes('HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n'));
      expect(response.bodyKind, const BodyNone());
    });

    test('chunked wins over content length', () {
      final parsed = parseRequestHead(bytes(
          'POST /x HTTP/1.1\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\n\r\n'));
      expect(parsed.bodyKind, const BodyChunked());
    });

    test('chunked matches in lists and mixed case', () {
      final parsed = parseResponseHead(
          bytes('HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip, Chunked\r\n\r\n'));
      expect(parsed.bodyKind, const BodyChunked());
    });

    test('chunked prefix does not match', () {
      final parsed = parseResponseHead(
          bytes('HTTP/1.1 200 OK\r\nTransfer-Encoding: chunkedx\r\n\r\n'));
      expect(parsed.bodyKind, const BodyUntilEof());
    });
  });

  group('summaries', () {
    test('request summary omits target text and headers', () {
      final parsed = parseRequestHead(bytes(
          'POST /pair?token=secret-token HTTP/1.1\r\nHost: bridge.local\r\nContent-Length: 4\r\n\r\nbody'));

      final summary = parsed.summary();
      expect(
          summary,
          Http1RequestHeadSummary(
            method: 'POST',
            targetLen: '/pair?token=secret-token'.length,
            version: const HttpVersion(major: 1, minor: 1),
            headerCount: 2,
            bodyOffset: parsed.bodyOffset,
            bodyKind: const BodyContentLength(4),
          ));

      final debug = summary.toString();
      expect(debug, isNot(contains('secret-token')));
      expect(debug, isNot(contains('bridge.local')));
    });

    test('response summary omits reason, headers, and body', () {
      final parsed = parseResponseHead(bytes(
          'HTTP/1.1 503 Secret Backend Down\r\nRetry-After: 10\r\nContent-Length: 11\r\n\r\nhidden-body'));

      final summary = parsed.summary();
      expect(
          summary,
          Http1ResponseHeadSummary(
            version: const HttpVersion(major: 1, minor: 1),
            status: 503,
            reasonLen: 'Secret Backend Down'.length,
            headerCount: 2,
            bodyOffset: parsed.bodyOffset,
            bodyKind: const BodyContentLength(11),
          ));

      final debug = summary.toString();
      expect(debug, isNot(contains('Secret Backend Down')));
      expect(debug, isNot(contains('Retry-After')));
      expect(debug, isNot(contains('hidden-body')));
    });

    test('summary lengths count bytes, not characters', () {
      final request =
          parseRequestHead(bytes('GET /caf\u00E9 HTTP/1.1\r\n\r\n'));
      expect(request.summary().targetLen, 6);
      final response = parseResponseHead(bytes('HTTP/1.1 200 O\u00E9\r\n\r\n'));
      expect(response.summary().reasonLen, 3);
    });
  });

  group('errors', () {
    test('every variant carries the reference message', () {
      expect(const IncompleteHead().toString(),
          'Http1ParseError: incomplete HTTP/1 head');
      expect(const InvalidHeadEncoding().toString(),
          'Http1ParseError: invalid HTTP/1 head encoding');
      expect(const InvalidStartLine('GET').toString(),
          'Http1ParseError: invalid HTTP/1 start line: GET');
      expect(const InvalidHeader('H').toString(),
          'Http1ParseError: invalid HTTP/1 header: H');
      expect(const InvalidVersion('V').toString(),
          'Http1ParseError: invalid HTTP version: V');
      expect(const InvalidStatus('S').toString(),
          'Http1ParseError: invalid HTTP status: S');
      expect(const InvalidContentLength('C').toString(),
          'Http1ParseError: invalid Content-Length: C');
    });

    test('errors are Exceptions with value equality', () {
      expect(const IncompleteHead(), isA<Exception>());
      expect(const IncompleteHead(), const IncompleteHead());
      expect(const IncompleteHead().hashCode, const IncompleteHead().hashCode);
      expect(const InvalidHeadEncoding(), const InvalidHeadEncoding());
      expect(const InvalidHeadEncoding().hashCode,
          const InvalidHeadEncoding().hashCode);
      expect(const InvalidStartLine('a'), const InvalidStartLine('a'));
      expect(const InvalidStartLine('a'), isNot(const InvalidStartLine('b')));
      expect(const InvalidStartLine('a'), isNot(const IncompleteHead()));
      expect(const InvalidStartLine('a').hashCode,
          const InvalidStartLine('a').hashCode);
      expect(const InvalidHeader('a'), const InvalidHeader('a'));
      expect(const InvalidHeader('a'), isNot(const InvalidHeader('b')));
      expect(
          const InvalidHeader('a').hashCode, const InvalidHeader('a').hashCode);
      expect(const InvalidVersion('a'), const InvalidVersion('a'));
      expect(const InvalidVersion('a'), isNot(const InvalidVersion('b')));
      expect(const InvalidVersion('a').hashCode,
          const InvalidVersion('a').hashCode);
      expect(const InvalidStatus('a'), const InvalidStatus('a'));
      expect(const InvalidStatus('a'), isNot(const InvalidStatus('b')));
      expect(
          const InvalidStatus('a').hashCode, const InvalidStatus('a').hashCode);
      expect(const InvalidContentLength('a'), const InvalidContentLength('a'));
      expect(const InvalidContentLength('a'),
          isNot(const InvalidContentLength('b')));
      expect(const InvalidContentLength('a').hashCode,
          const InvalidContentLength('a').hashCode);
    });
  });

  group('value semantics', () {
    test('parsed heads compare by value', () {
      final a = parseRequestHead(bytes('GET / HTTP/1.0\r\n\r\n'));
      final b = parseRequestHead(bytes('GET / HTTP/1.0\r\n\r\n'));
      expect(a, b);
      expect(a.hashCode, b.hashCode);
      expect(a.toString(), contains('GET'));
      expect(a, isNot(parseRequestHead(bytes('GET /x HTTP/1.0\r\n\r\n'))));
      expect(a, isNot('not a head'));

      final c = parseResponseHead(bytes('HTTP/1.1 200 OK\r\n\r\n'));
      expect(c, parseResponseHead(bytes('HTTP/1.1 200 OK\r\n\r\n')));
      expect(c.hashCode,
          parseResponseHead(bytes('HTTP/1.1 200 OK\r\n\r\n')).hashCode);
      expect(c.toString(), contains('200'));
      expect(c, isNot(parseResponseHead(bytes('HTTP/1.1 404 OK\r\n\r\n'))));
      expect(c, isNot('not a head'));
    });

    test('summaries compare by value', () {
      final request = parseRequestHead(bytes('GET / HTTP/1.0\r\n\r\n'));
      expect(request.summary(), request.summary());
      expect(request.summary().hashCode, request.summary().hashCode);
      expect(request.summary().toString(), contains('GET'));
      expect(request.summary(),
          isNot(parseRequestHead(bytes('POST / HTTP/1.0\r\n\r\n')).summary()));
      expect(request.summary(), isNot('not a summary'));

      final response = parseResponseHead(bytes('HTTP/1.1 200 OK\r\n\r\n'));
      expect(response.summary(), response.summary());
      expect(response.summary().hashCode, response.summary().hashCode);
      expect(response.summary().toString(), contains('200'));
      expect(response.summary(), isNot('not a summary'));
    });
  });
}
