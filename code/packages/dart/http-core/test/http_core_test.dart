/// Unit tests for `coding_adventures_http_core`: all 9 reference
/// tests mirrored case-for-case, plus edge matrices over versions,
/// headers, content helpers, targets, routes, query pairs, body
/// kinds, and value semantics.
library;

import 'package:coding_adventures_http_core/http_core.dart';
import 'package:test/test.dart';

void main() {
  group('versions', () {
    test('parses versions', () {
      final version = HttpVersion.parse('HTTP/1.1');
      expect(version.major, equals(1));
      expect(version.minor, equals(1));
      expect(version.toString(), equals('HTTP/1.1'));
    });

    test('version matrix', () {
      expect(HttpVersion.parse('HTTP/2.0').toString(), equals('HTTP/2.0'));
      expect(HttpVersion.parse('HTTP/0.9').toString(), equals('HTTP/0.9'));
      expect(HttpVersion.parse('HTTP/10.20').toString(), equals('HTTP/10.20'));
      expect(HttpVersion.parse('HTTP/65535.65535').toString(),
          equals('HTTP/65535.65535'));
      expect(HttpVersion.parse('HTTP/01.02').toString(), equals('HTTP/1.2'));
      for (final bad in [
        '',
        'HTTP',
        'HTTP/',
        'HTTP/1',
        'HTTP/1.',
        'HTTP/.1',
        'HTTP/a.b',
        'HTTP/1.b',
        'HTTP/-1.0',
        'HTTP/+1.0',
        'HTTP/1.0 ',
        ' HTTP/1.0',
        'HTTP/65536.0',
        'HTTP/0.65536',
        'HTTP/1.2.3',
        'http/1.1',
        'HTTP//1.1',
      ]) {
        expect(() => HttpVersion.parse(bad),
            throwsA(equals(InvalidHttpVersion(bad))),
            reason: 'input $bad');
      }
      expect(const InvalidHttpVersion('x').toString(),
          equals('invalid HTTP version: x'));
      expect(
          const InvalidHttpVersion('x'), equals(const InvalidHttpVersion('x')));
      expect(const InvalidHttpVersion('x'),
          isNot(equals(const InvalidHttpVersion('y'))));
      expect(const InvalidHttpVersion('x').hashCode,
          equals(const InvalidHttpVersion('x').hashCode));
    });

    test('version values', () {
      expect(const HttpVersion(major: 1, minor: 1),
          equals(const HttpVersion(major: 1, minor: 1)));
      expect(const HttpVersion(major: 1, minor: 1),
          isNot(equals(const HttpVersion(major: 2, minor: 0))));
      expect(const HttpVersion(major: 1, minor: 1).hashCode,
          equals(const HttpVersion(major: 1, minor: 1).hashCode));
    });
  });

  group('headers', () {
    test('finds headers case insensitively', () {
      final headers = [Header('Content-Type', 'text/plain')];
      expect(findHeader(headers, 'content-type'), equals('text/plain'));
    });

    test('header lookup matrix', () {
      final headers = [
        Header('X-First', 'one'),
        Header('CONTENT-LENGTH', '5'),
        Header('X-First', 'two'),
      ];
      // First match wins, ASCII case-insensitive.
      expect(findHeader(headers, 'x-first'), equals('one'));
      expect(findHeader(headers, 'Content-Length'), equals('5'));
      expect(findHeader([], 'x'), isNull);
      expect(findHeader(headers, 'missing'), isNull);
      // Non-ASCII never folds: 'İ' (U+0130) must not match 'i'.
      expect(findHeader([Header('İ', 'v')], 'i'), isNull);
      expect(findHeader([Header('i', 'v')], 'İ'), isNull);
      // Length mismatch short-circuits.
      expect(findHeader([Header('ab', 'v')], 'abc'), isNull);
    });

    test('header values', () {
      expect(Header('a', 'b'), equals(Header('a', 'b')));
      expect(Header('a', 'b'), isNot(equals(Header('a', 'c'))));
      expect(Header('a', 'b').hashCode, equals(Header('a', 'b').hashCode));
      expect(Header('a', 'b').toString(), equals('Header(a, b)'));
    });
  });

  group('content helpers', () {
    test('parses content helpers', () {
      final headers = [
        Header('Content-Length', '42'),
        Header('Content-Type', 'text/html; charset=utf-8'),
      ];
      expect(parseContentLength(headers), equals(42));
      expect(parseContentType(headers), equals(('text/html', 'utf-8')));
    });

    test('content length matrix', () {
      int? len(String v) => parseContentLength([Header('Content-Length', v)]);
      expect(len('0'), equals(0));
      expect(len('007'), equals(7));
      expect(len('18446744073709551615'), isNull); // u64 max, over int64
      expect(len('99999999999999999999999999'), isNull); // overflow
      expect(len(''), isNull);
      expect(len(' 42'), isNull);
      expect(len('42 '), isNull);
      expect(len('+42'), isNull);
      expect(len('-1'), isNull);
      expect(len('4a'), isNull);
      expect(len('0x10'), isNull);
      expect(parseContentLength([]), isNull);
      expect(parseContentLength([Header('Content-Length', '3')]), equals(3));
    });

    test('content type matrix', () {
      (String, String?)? type(String v) =>
          parseContentType([Header('Content-Type', v)]);
      expect(type('application/json'), equals(('application/json', null)));
      expect(
          type('text/html; charset="utf-8"'), equals(('text/html', 'utf-8')));
      expect(type('text/html; Charset=UTF-8'), equals(('text/html', 'UTF-8')));
      expect(type('text/html; level=1; charset=iso-8859-1'),
          equals(('text/html', 'iso-8859-1')));
      expect(type('text/html; charset='), equals(('text/html', '')));
      expect(type('text/html; note'), equals(('text/html', null)));
      expect(type('text/html; x=1'), equals(('text/html', null)));
      expect(type('  text/plain  ;  charset = latin1  '),
          equals(('text/plain', 'latin1')));
      expect(type(''), isNull);
      expect(type('; charset=utf-8'), isNull);
      expect(type('   '), isNull);
      expect(parseContentType([]), isNull);
    });
  });

  group('request targets', () {
    test('parses without decoding query values', () {
      final target = parseRequestTarget(
          '/clip/v2/resource/light?id=abc%20123&limit=10#ignored');
      expect(target.path, equals('/clip/v2/resource/light'));
      expect(target.query, equals('id=abc%20123&limit=10'));
      expect(target.fragment, equals('ignored'));
      expect(target.queryPairs().toList(),
          equals([('id', 'abc%20123'), ('limit', '10')]));
      expect(target.queryValue('limit'), equals('10'));
      expect(target.queryValue('missing'), isNull);
    });

    test('target matrix', () {
      RequestTarget t(String s) => parseRequestTarget(s);
      expect(t('/a/b'), equals(RequestTarget(path: '/a/b')));
      expect(t('/a?x=1').query, equals('x=1'));
      expect(t('/a#f').fragment, equals('f'));
      expect(t('/a?x=1#f').fragment, equals('f'));
      // '?' inside the fragment stays in the fragment.
      expect(t('/a#f?x=1').query, isNull);
      expect(t('/a#f?x=1').fragment, equals('f?x=1'));
      // First '#' wins; first '?' wins.
      expect(t('/a?x=1?y=2').query, equals('x=1?y=2'));
      expect(t('?x=1').path, equals('/'));
      expect(t('?x=1').query, equals('x=1'));
      expect(t('#f').path, equals('/'));
      expect(t('').path, equals('/'));
      expect(t('/').path, equals('/'));
      expect(t('rel/path'),
          equals(RequestTarget(path: 'rel/path', query: null, fragment: null)));
    });

    test('query pairs matrix', () {
      List<(String, String)> pairs(String q) =>
          RequestTarget(path: '/', query: q).queryPairs().toList();
      expect(pairs(''), equals([]));
      expect(pairs('&&&'), equals([]));
      expect(pairs('a'), equals([('a', '')]));
      expect(pairs('a='), equals([('a', '')]));
      expect(pairs('=b'), equals([('', 'b')]));
      expect(pairs('a=b=c'), equals([('a', 'b=c')]));
      expect(pairs('&a=1&&b=2&'), equals([('a', '1'), ('b', '2')]));
      expect(RequestTarget(path: '/').queryPairs().toList(), equals([]));
      // First value wins.
      final t = RequestTarget(path: '/', query: 'k=1&k=2');
      expect(t.queryValue('k'), equals('1'));
    });

    test('target values', () {
      const a = RequestTarget(path: '/a', query: 'x', fragment: 'f');
      expect(a, equals(RequestTarget(path: '/a', query: 'x', fragment: 'f')));
      expect(a, isNot(equals(RequestTarget(path: '/b'))));
      expect(a.hashCode, equals(a.hashCode));
      expect(a.toString(), equals('RequestTarget(/a, x, f)'));
    });
  });

  group('heads', () {
    test('request heads expose path and query helpers', () {
      final request = RequestHead(
        method: 'GET',
        target: '/api/devices?room=kitchen&verbose',
        version: HttpVersion(major: 1, minor: 1),
        headers: [],
      );
      expect(request.path(), equals('/api/devices'));
      expect(request.queryValue('room'), equals('kitchen'));
      expect(request.queryValue('verbose'), equals(''));
    });

    test('heads delegate to helpers', () {
      final request = RequestHead(
        method: 'POST',
        target: '/submit',
        version: HttpVersion(major: 1, minor: 1),
        headers: [Header('Content-Length', '5')],
      );
      final response = ResponseHead(
        version: HttpVersion(major: 1, minor: 0),
        status: 200,
        reason: 'OK',
        headers: [Header('Content-Type', 'application/json')],
      );
      expect(request.contentLength(), equals(5));
      expect(request.header('content-length'), equals('5'));
      expect(request.contentType(), isNull);
      expect(response.contentType(), equals(('application/json', null)));
      expect(response.header('content-type'), equals('application/json'));
      expect(response.contentLength(), isNull);
    });

    test('head values', () {
      final req = RequestHead(
          method: 'GET',
          target: '/',
          version: HttpVersion(major: 1, minor: 1),
          headers: [Header('a', 'b')]);
      expect(
          req,
          equals(RequestHead(
              method: 'GET',
              target: '/',
              version: HttpVersion(major: 1, minor: 1),
              headers: [Header('a', 'b')])));
      expect(
          req,
          isNot(equals(RequestHead(
              method: 'POST',
              target: '/',
              version: HttpVersion(major: 1, minor: 1),
              headers: []))));
      expect(req.hashCode, equals(req.hashCode));
      expect(req.toString(),
          equals('RequestHead(GET, /, HTTP/1.1, [Header(a, b)])'));
      final res = ResponseHead(
          version: HttpVersion(major: 1, minor: 1),
          status: 404,
          reason: 'Nope',
          headers: []);
      expect(
          res,
          equals(ResponseHead(
              version: HttpVersion(major: 1, minor: 1),
              status: 404,
              reason: 'Nope',
              headers: [])));
      expect(res.hashCode, equals(res.hashCode));
      expect(res.toString(), equals('ResponseHead(HTTP/1.1, 404, Nope, [])'));
    });
  });

  group('routes', () {
    test('patterns match targets by path only', () {
      final pattern = RoutePattern.parse('/clip/v2/resource/:kind/:id');
      expect(
        pattern.matchTarget('/clip/v2/resource/light/abc?limit=10'),
        equals([
          ('kind', 'light'),
          ('id', 'abc'),
        ]),
      );
      expect(pattern.matchTarget('/clip/v2/resource/light'), isNull);
    });

    test('matches named params', () {
      final pattern = RoutePattern.parse('/hello/:name');
      expect(
          pattern.matchPath('/hello/Adhithya'), equals([('name', 'Adhithya')]));
      expect(pattern.matchPath('/hello'), isNull);
      expect(pattern.matchPath('/goodbye/Adhithya'), isNull);
    });

    test('handles root paths', () {
      final pattern = RoutePattern.parse('/');
      expect(pattern.matchPath('/'), equals([]));
      expect(pattern.matchPath('/extra'), isNull);
    });

    test('route matrix', () {
      // Empty pattern segments and double slashes collapse.
      expect(RoutePattern.parse('//a//b').segments,
          equals([RouteLiteral('a'), RouteLiteral('b')]));
      // A bare ':' is a param with an empty name (mirrors strip_prefix).
      expect(RoutePattern.parse('/x/:').segments,
          equals([RouteLiteral('x'), RouteParam('')]));
      // Params match empty-looking but present segments only via count.
      final p = RoutePattern.parse('/:a/:b');
      expect(p.matchPath('/1/2'), equals([('a', '1'), ('b', '2')]));
      expect(p.matchPath('/1'), isNull);
      expect(p.matchPath('/1/2/3'), isNull);
      // Literal mismatch anywhere fails.
      final q = RoutePattern.parse('/a/b/c');
      expect(q.matchPath('/a/X/c'), isNull);
      expect(q.matchPath('/a/b/c'), equals([]));
    });

    test('split segments matrix', () {
      expect(splitPathSegments('/'), equals([]));
      expect(splitPathSegments(''), equals([]));
      expect(splitPathSegments('///'), equals([]));
      expect(splitPathSegments('/a/b'), equals(['a', 'b']));
      expect(splitPathSegments('a/b'), equals(['a', 'b']));
      expect(splitPathSegments('/a//b/'), equals(['a', 'b']));
    });

    test('route values', () {
      expect(RouteLiteral('a'), equals(RouteLiteral('a')));
      expect(RouteLiteral('a'), isNot(equals(RouteParam('a'))));
      expect(RouteParam('n'), equals(RouteParam('n')));
      expect(RouteLiteral('a').hashCode, equals(RouteLiteral('a').hashCode));
      expect(RouteLiteral('a').toString(), equals('RouteLiteral(a)'));
      expect(RouteParam('n').toString(), equals('RouteParam(n)'));
      final pattern = RoutePattern.parse('/a/:b');
      expect(pattern, equals(RoutePattern.parse('/a/:b')));
      expect(pattern, isNot(equals(RoutePattern.parse('/a/:c'))));
      expect(pattern, isNot(equals(RoutePattern.parse('/a'))));
      expect(pattern.hashCode, equals(pattern.hashCode));
      expect(pattern.toString(),
          equals('RoutePattern([RouteLiteral(a), RouteParam(b)])'));
    });
  });

  group('body kinds and version', () {
    test('body kinds compare by type and payload', () {
      final kinds = [
        const BodyNone(),
        const BodyContentLength(42),
        const BodyUntilEof(),
        const BodyChunked(),
      ];
      for (final kind in kinds) {
        expect(kind == kind, isTrue);
        expect(kind.hashCode, equals(kind.hashCode));
        expect(kind.toString(), isNotEmpty);
      }
      for (var i = 0; i < kinds.length; i++) {
        for (var j = 0; j < kinds.length; j++) {
          if (i != j) {
            expect(kinds[i] == kinds[j], isFalse, reason: '$i vs $j');
          }
        }
      }
      expect(const BodyContentLength(42),
          isNot(equals(const BodyContentLength(43))));
      expect(const BodyNone().toString(), equals('BodyNone()'));
      expect(const BodyContentLength(7).toString(),
          equals('BodyContentLength(7)'));
      expect(const BodyUntilEof().toString(), equals('BodyUntilEof()'));
      expect(const BodyChunked().toString(), equals('BodyChunked()'));
    });

    test('version constant', () {
      expect(httpCoreVersion, equals('0.1.0'));
    });
  });
}
