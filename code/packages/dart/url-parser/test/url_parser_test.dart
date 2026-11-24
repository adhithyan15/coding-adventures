/// Unit tests for `coding_adventures_url_parser`.
///
/// Mirrors the Rust reference's inline suite case-for-case, then covers the
/// branches it leaves implicit: scheme-form edges, userinfo/port quirks,
/// resolve fall-throughs, canonicalization corners, error-message texts,
/// value semantics, and lone-surrogate totality.
library;

import 'package:coding_adventures_url_parser/coding_adventures_url_parser.dart';
import 'package:test/test.dart';

/// Parses [input], expecting [UrlError] [want], and returns nothing.
/// Fails the test when parsing succeeds or throws a different error.
void expectParseError(String input, UrlError want) {
  expect(() => Url.parse(input), throwsA(want));
}

void main() {
  group('mirror: basic parsing', () {
    test('parse simple http url', () {
      final url = Url.parse('http://www.example.com');
      expect(url.scheme, 'http');
      expect(url.host, 'www.example.com');
      expect(url.port, isNull);
      expect(url.path, '/');
      expect(url.query, isNull);
      expect(url.fragment, isNull);
    });

    test('parse http with path', () {
      final url = Url.parse('http://www.example.com/docs/page.html');
      expect(url.scheme, 'http');
      expect(url.host, 'www.example.com');
      expect(url.path, '/docs/page.html');
    });

    test('parse all components', () {
      final url = Url.parse(
          'http://alice:secret@www.example.com:8080/docs/page.html?q=hello#section2');
      expect(url.scheme, 'http');
      expect(url.userinfo, 'alice:secret');
      expect(url.host, 'www.example.com');
      expect(url.port, 8080);
      expect(url.path, '/docs/page.html');
      expect(url.query, 'q=hello');
      expect(url.fragment, 'section2');
    });

    test('parse https url', () {
      final url = Url.parse('https://secure.example.com/login');
      expect(url.scheme, 'https');
      expect(url.host, 'secure.example.com');
      expect(url.effectivePort(), 443);
    });

    test('parse ftp url', () {
      final url = Url.parse('ftp://files.example.com/pub/readme.txt');
      expect(url.scheme, 'ftp');
      expect(url.effectivePort(), 21);
    });

    test('parse mailto url', () {
      final url = Url.parse('mailto:alice@example.com');
      expect(url.scheme, 'mailto');
      expect(url.host, isNull);
      expect(url.path, 'alice@example.com');
      expect(url.toUrlString(), 'mailto:alice@example.com');
    });
  });

  group('mirror: case, ports, authority', () {
    test('scheme is lowercased', () {
      final url = Url.parse('HTTP://WWW.EXAMPLE.COM/PATH');
      expect(url.scheme, 'http');
      expect(url.host, 'www.example.com');
      expect(url.path, '/PATH');
    });

    test('effective port http default', () {
      final url = Url.parse('http://example.com');
      expect(url.port, isNull);
      expect(url.effectivePort(), 80);
    });

    test('effective port explicit', () {
      final url = Url.parse('http://example.com:9090');
      expect(url.port, 9090);
      expect(url.effectivePort(), 9090);
    });

    test('authority with all parts', () {
      final url = Url.parse('http://user:pass@host.com:8080/path');
      expect(url.authority(), 'user:pass@host.com:8080');
    });

    test('authority host only', () {
      final url = Url.parse('http://host.com/path');
      expect(url.authority(), 'host.com');
    });
  });

  group('mirror: invalid urls', () {
    test('missing scheme', () {
      expectParseError('www.example.com', UrlError.missingScheme);
    });

    test('invalid scheme starts with digit', () {
      expectParseError('1http://x.com', UrlError.invalidScheme);
    });

    test('invalid port too large', () {
      expectParseError('http://host:99999', UrlError.invalidPort);
    });
  });

  group('mirror: percent-encoding', () {
    test('encode space', () {
      expect(percentEncode('hello world'), 'hello%20world');
    });

    test('encode preserves unreserved', () {
      expect(percentEncode('abc-def_ghi.jkl~mno'), 'abc-def_ghi.jkl~mno');
    });

    test('encode preserves slashes', () {
      expect(percentEncode('/path/to/file'), '/path/to/file');
    });

    test('decode space', () {
      expect(percentDecode('hello%20world'), 'hello world');
    });

    test('decode utf8', () {
      expect(percentDecode('%E6%97%A5'), '日');
    });

    test('decode roundtrip', () {
      const original = 'hello world/日本語';
      expect(percentDecode(percentEncode(original)), original);
    });

    test('decode malformed truncated', () {
      expect(
          () => percentDecode('%2'), throwsA(UrlError.invalidPercentEncoding));
    });

    test('decode malformed bad hex', () {
      expect(
          () => percentDecode('%GG'), throwsA(UrlError.invalidPercentEncoding));
    });
  });

  group('mirror: relative resolution', () {
    test('resolve same directory', () {
      final base = Url.parse('http://host/a/b/c.html');
      final resolved = base.resolve('d.html');
      expect(resolved.scheme, 'http');
      expect(resolved.host, 'host');
      expect(resolved.path, '/a/b/d.html');
    });

    test('resolve parent directory', () {
      final base = Url.parse('http://host/a/b/c.html');
      expect(base.resolve('../d.html').path, '/a/d.html');
    });

    test('resolve grandparent directory', () {
      final base = Url.parse('http://host/a/b/c.html');
      expect(base.resolve('../../d.html').path, '/d.html');
    });

    test('resolve absolute path', () {
      final base = Url.parse('http://host/a/b/c.html');
      final resolved = base.resolve('/x/y.html');
      expect(resolved.path, '/x/y.html');
      expect(resolved.host, 'host');
    });

    test('resolve scheme relative', () {
      final base = Url.parse('http://host/a/b');
      final resolved = base.resolve('//other.com/path');
      expect(resolved.scheme, 'http');
      expect(resolved.host, 'other.com');
      expect(resolved.path, '/path');
    });

    test('resolve already absolute', () {
      final base = Url.parse('http://host/a/b');
      final resolved = base.resolve('https://other.com/x');
      expect(resolved.scheme, 'https');
      expect(resolved.host, 'other.com');
      expect(resolved.path, '/x');
    });

    test('resolve dot segments', () {
      final base = Url.parse('http://host/a/b/c');
      expect(base.resolve('./d').path, '/a/b/d');
    });

    test('resolve empty returns base', () {
      final base = Url.parse('http://host/a/b?q=1#frag');
      final resolved = base.resolve('');
      expect(resolved.path, '/a/b');
      expect(resolved.query, 'q=1');
      expect(resolved.fragment, isNull);
    });

    test('resolve fragment only', () {
      final base = Url.parse('http://host/a/b');
      final resolved = base.resolve('#sec');
      expect(resolved.path, '/a/b');
      expect(resolved.fragment, 'sec');
    });

    test('resolve with query', () {
      final base = Url.parse('http://host/a/b');
      final resolved = base.resolve('c?key=val');
      expect(resolved.path, '/a/c');
      expect(resolved.query, 'key=val');
    });
  });

  group('mirror: dot segments via resolve', () {
    // The reference tests its private remover directly; here the same
    // vectors run through resolve, which is the only caller.
    test('remove single dot', () {
      final base = Url.parse('http://host/');
      expect(base.resolve('/a/./b').path, '/a/b');
    });

    test('remove double dot', () {
      final base = Url.parse('http://host/');
      expect(base.resolve('/a/b/../c').path, '/a/c');
    });

    test('remove multiple double dots', () {
      final base = Url.parse('http://host/');
      expect(base.resolve('/a/b/../../c').path, '/c');
    });

    test('double dot above root', () {
      final base = Url.parse('http://host/');
      expect(base.resolve('/a/../../../c').path, '/c');
    });
  });

  group('mirror: serialization', () {
    test('roundtrip full url', () {
      const input = 'http://user:pass@host.com:8080/path?q=1#frag';
      expect(Url.parse(input).toUrlString(), input);
    });

    test('roundtrip simple url', () {
      const input = 'http://example.com/path';
      expect(Url.parse(input).toUrlString(), input);
    });

    test('canonicalize normalizes identity without losing fragment', () {
      final url = Url.parse('HTTP://Example.COM:80/a/../%7euser?q=%7b#part')
          .canonicalize();
      expect(url.toUrlString(), 'http://example.com/~user?q=%7B#part');
    });

    test('canonicalize preserves unicode, rejects bad escapes', () {
      final url = Url.parse('http://example.com/café/%E2%98%83').canonicalize();
      expect(url.toUrlString(), 'http://example.com/café/%E2%98%83');
      expect(() => Url.parse('http://example.com/%GG').canonicalize(),
          throwsA(UrlError.invalidPercentEncoding));
    });
  });

  group('mirror: historical and ipv6', () {
    test('parse cern original url', () {
      final url =
          Url.parse('http://info.cern.ch/hypertext/WWW/TheProject.html');
      expect(url.scheme, 'http');
      expect(url.host, 'info.cern.ch');
      expect(url.path, '/hypertext/WWW/TheProject.html');
      expect(url.effectivePort(), 80);
    });

    test('parse ncsa mosaic url', () {
      final url = Url.parse('http://www.ncsa.uiuc.edu/SDG/Software/Mosaic/');
      expect(url.host, 'www.ncsa.uiuc.edu');
      expect(url.path, '/SDG/Software/Mosaic/');
    });

    test('parse ipv6 localhost', () {
      final url = Url.parse('http://[::1]:8080/path');
      expect(url.host, '[::1]');
      expect(url.port, 8080);
      expect(url.path, '/path');
    });
  });

  group('mirror: edge cases', () {
    test('parse trailing slash', () {
      expect(Url.parse('http://host/').path, '/');
    });

    test('parse query without path', () {
      final url = Url.parse('http://host?q=1');
      expect(url.host, 'host');
      expect(url.path, '/');
      expect(url.query, 'q=1');
    });

    test('parse fragment without path', () {
      final url = Url.parse('http://host#frag');
      expect(url.host, 'host');
      expect(url.path, '/');
      expect(url.fragment, 'frag');
    });
  });

  group('EXTRA: scheme forms', () {
    test('surrounding whitespace is trimmed for parsing', () {
      final url = Url.parse('  http://host/path  ');
      expect(url.scheme, 'http');
      expect(url.host, 'host');
      expect(url.path, '/path');
      // ...but the preserved input keeps it, so equality notices.
      expect(url == Url.parse('http://host/path'), isFalse);
    });

    test('scheme:path form splits query and fragment', () {
      final url = Url.parse('mailto:alice@example.com?subject=x#y');
      expect(url.scheme, 'mailto');
      expect(url.host, isNull);
      expect(url.path, 'alice@example.com');
      expect(url.query, 'subject=x');
      expect(url.fragment, 'y');
    });

    test('scheme:path accepts full scheme alphabet', () {
      final url = Url.parse('x-y+z.w:p');
      expect(url.scheme, 'x-y+z.w');
      expect(url.path, 'p');
    });

    test('colon at start or after slash misses the scheme', () {
      expectParseError(':foo', UrlError.missingScheme);
      expectParseError('a/b:c', UrlError.missingScheme);
      expectParseError('', UrlError.missingScheme);
    });

    test('empty and digit-led schemes are invalid', () {
      expectParseError('://host', UrlError.invalidScheme);
      expectParseError('1x:y', UrlError.invalidScheme);
      expectParseError('ht tp://h', UrlError.invalidScheme);
    });
  });

  group('EXTRA: authority quirks', () {
    test('userinfo splits at the last @', () {
      final url = Url.parse('http://a@b@c/');
      expect(url.userinfo, 'a@b');
      expect(url.host, 'c');
    });

    test('empty host parses to null without error', () {
      // UrlError.emptyHost exists but is never thrown, in either lane.
      final url = Url.parse('http:///path');
      expect(url.host, isNull);
      expect(url.path, '/path');
    });

    test('userinfo without host', () {
      final url = Url.parse('http://user@/p');
      expect(url.userinfo, 'user');
      expect(url.host, isNull);
      expect(url.authority(), 'user@');
    });

    test('trailing colon stays in the host', () {
      final url = Url.parse('http://host:/p');
      expect(url.host, 'host:');
      expect(url.port, isNull);
    });

    test('non-digit port stays in the host', () {
      final url = Url.parse('http://host:abc/p');
      expect(url.host, 'host:abc');
      expect(url.port, isNull);
    });

    test('port bounds and leading zeros', () {
      expect(Url.parse('http://h:0/p').port, 0);
      expect(Url.parse('http://h:65535/p').port, 65535);
      expect(Url.parse('http://h:0080/p').port, 80);
      expectParseError('http://h:65536', UrlError.invalidPort);
      expect(Url.parse('http://h:0').effectivePort(), 0);
    });

    test('ipv6 without port and malformed bracket', () {
      final bare = Url.parse('http://[::1]/');
      expect(bare.host, '[::1]');
      expect(bare.port, isNull);
      final malformed = Url.parse('http://[::1/path');
      expect(malformed.host, '[::1');
      expect(malformed.port, isNull);
    });

    test('ipv6 empty and non-digit ports are errors', () {
      expectParseError('http://[::1]:/p', UrlError.invalidPort);
      expectParseError('http://[::1]:abc/p', UrlError.invalidPort);
    });

    test('port without host is dropped by serialization', () {
      // "http://:8080/p" parses (host None, port 8080) but toUrlString
      // emits the no-host form, losing the port — as in the reference.
      final url = Url.parse('http://:8080/p');
      expect(url.host, isNull);
      expect(url.port, 8080);
      expect(url.toUrlString(), 'http:/p');
    });
  });

  group('EXTRA: resolve corners', () {
    test('garbage scheme shape falls through to path merge', () {
      final base = Url.parse('http://h/a/b');
      // "://foo" has an empty scheme candidate, so it merges as a path.
      expect(base.resolve('://foo').path, '/a/://foo');
    });

    test('scheme:path references parse as absolute', () {
      final base = Url.parse('http://h/a/b');
      final resolved = base.resolve('http:foo');
      expect(resolved.scheme, 'http');
      expect(resolved.host, isNull);
      expect(resolved.path, 'foo');
    });

    test('scheme-shape probes each allowed character', () {
      final base = Url.parse('http://h/a/b');
      for (final ref in ['h1:y', 'h+x:y', 'h-x:y', 'h.x:y']) {
        final resolved = base.resolve(ref);
        expect(resolved.path, 'y', reason: ref);
        expect(resolved.host, isNull, reason: ref);
      }
      expect(base.resolve('h1:y').scheme, 'h1');
    });

    test('non-scheme shapes fall through to path merge', () {
      final base = Url.parse('http://h/a/b');
      // Digit-led candidate.
      expect(base.resolve('1x:y').path, '/a/1x:y');
      // Invalid rest character.
      expect(base.resolve('h@:x').path, '/a/h@:x');
    });

    test('absolute reference keeps its own query and fragment', () {
      final base = Url.parse('http://h/a?bq#f');
      final resolved = base.resolve('https://o/x?q2#f2');
      expect(resolved.query, 'q2');
      expect(resolved.fragment, 'f2');
    });

    test('absolute path drops a missing query', () {
      final base = Url.parse('http://h/a?bq=1');
      final resolved = base.resolve('/x');
      expect(resolved.path, '/x');
      expect(resolved.query, isNull);
    });

    test('relative path without query drops the base query', () {
      final base = Url.parse('http://h/a?bq=1');
      final resolved = base.resolve('c');
      expect(resolved.path, '/c');
      expect(resolved.query, isNull);
    });

    test('query-only reference merges onto the base directory', () {
      final base = Url.parse('http://h/a/b?old=1');
      final resolved = base.resolve('?q=2');
      // Merging drops the last base segment even for query-only
      // references (reference behavior, unlike strict RFC 3986).
      expect(resolved.path, '/a/');
      expect(resolved.query, 'q=2');
    });

    test('empty fragment is Some, not None', () {
      final base = Url.parse('http://h/a');
      final resolved = base.resolve('#');
      expect(resolved.fragment, '');
    });

    test('relative reference is trimmed', () {
      final base = Url.parse('http://h/a/b');
      expect(base.resolve('  d.html  ').path, '/a/d.html');
    });

    test('first # wins the fragment split', () {
      final base = Url.parse('http://h/');
      // Fragment swallows the second '#' (spec text says "from the right"
      // but the reference — and this port — split at the first).
      final resolved = base.resolve('p?a=1#b#c');
      expect(resolved.query, 'a=1');
      expect(resolved.fragment, 'b#c');
    });

    test('resolved urls serialize and reparse', () {
      final base = Url.parse('http://h/a/b/c.html');
      final resolved = base.resolve('../d.html?x=1#f');
      expect(resolved.toUrlString(), 'http://h/a/d.html?x=1#f');
      expect(Url.parse(resolved.toUrlString()).path, '/a/d.html');
    });
  });

  group('EXTRA: canonicalize corners', () {
    test('default ports drop, others stay', () {
      expect(Url.parse('http://h:80/').canonicalize().port, isNull);
      expect(Url.parse('https://h:443/').canonicalize().port, isNull);
      expect(Url.parse('ftp://h:21/').canonicalize().port, isNull);
      expect(Url.parse('http://h:8080/').canonicalize().port, 8080);
      expect(Url.parse('gopher://h:80/').canonicalize().port, 80);
    });

    test('userinfo escapes normalize', () {
      final url = Url.parse('http://%7euser@h/').canonicalize();
      expect(url.userinfo, '~user');
    });

    test('non-unreserved escapes uppercase and stay escaped', () {
      final url = Url.parse('http://h/%2f%7e').canonicalize();
      expect(url.path, '/%2F~');
    });

    test('mailto keeps its empty path', () {
      final url = Url.parse('mailto:').canonicalize();
      expect(url.host, isNull);
      expect(url.path, '');
    });

    test('canonicalize rejects truncated escapes', () {
      expect(() => Url.parse('http://h/%').canonicalize(),
          throwsA(UrlError.invalidPercentEncoding));
      expect(() => Url.parse('http://h/p?a%').canonicalize(),
          throwsA(UrlError.invalidPercentEncoding));
    });

    test('canonicalize is idempotent', () {
      final once = Url.parse('HTTP://H:80/a/./b/../%7e?q=%7b#f').canonicalize();
      expect(once.canonicalize(), equals(once));
      expect(once.toUrlString(), 'http://h/a/~?q=%7B#f');
    });
  });

  group('EXTRA: percent edges', () {
    test('encode escapes percent and multibyte', () {
      expect(percentEncode('100%'), '100%25');
      expect(percentEncode('日本語'), '%E6%97%A5%E6%9C%AC%E8%AA%9E');
      expect(percentEncode('a+b'), 'a%2Bb');
    });

    test('decode handles lowercase hex and literals', () {
      expect(percentDecode('%7e'), '~');
      expect(percentDecode('plain'), 'plain');
      expect(percentDecode('日本語'), '日本語');
      expect(percentDecode('a+b'), 'a+b');
    });

    test('decode preserves a leading byte-order mark', () {
      // Dart's utf8.decode strips a leading BOM; the reference keeps it.
      // Found by the fuzzer (seed 71).
      final bom = String.fromCharCode(0xFEFF);
      expect(percentEncode(bom), '%EF%BB%BF');
      expect(percentDecode('%EF%BB%BF'), bom);
      expect(percentDecode('%EF%BB%BFa'), '${bom}a');
      expect(percentDecode(percentEncode('${bom}x?y')), '${bom}x?y');
    });

    test('paired surrogates round-trip as astral characters', () {
      final grin = String.fromCharCode(0x1F600);
      expect(grin.length, 2); // a genuine surrogate pair
      final encoded = percentEncode('a${grin}b');
      expect(encoded, 'a%F0%9F%98%80b');
      expect(percentDecode(encoded), 'a${grin}b');
    });

    test('decode rejects bad utf8 and lone surrogates', () {
      expect(
          () => percentDecode('%FF'), throwsA(UrlError.invalidPercentEncoding));
      expect(() => percentDecode('%E6%97'),
          throwsA(UrlError.invalidPercentEncoding));
      expect(
          () => percentDecode('%'), throwsA(UrlError.invalidPercentEncoding));
      expect(
          () => percentDecode('a%2'), throwsA(UrlError.invalidPercentEncoding));
      expect(
          () => percentDecode('%2G'), throwsA(UrlError.invalidPercentEncoding));
      final loneHigh = String.fromCharCode(0xD800);
      final loneLow = String.fromCharCode(0xDC00);
      expect(() => percentDecode('a${loneHigh}b'),
          throwsA(UrlError.invalidPercentEncoding));
      expect(() => percentDecode('a${loneLow}b'),
          throwsA(UrlError.invalidPercentEncoding));
      expect(() => percentEncode('a${loneHigh}b'),
          throwsA(UrlError.invalidPercentEncoding));
    });
  });

  group('EXTRA: errors and values', () {
    test('error messages are exact', () {
      expect(UrlError.missingScheme.message, "missing scheme (expected '://')");
      expect(UrlError.invalidScheme.message,
          'invalid scheme (must be [a-z][a-z0-9+.-]*)');
      expect(UrlError.invalidPort.message, 'invalid port (must be 0-65535)');
      expect(UrlError.invalidPercentEncoding.message,
          'malformed percent-encoding');
      expect(UrlError.emptyHost.message, 'empty host in authority-based URL');
      expect(UrlError.relativeWithoutBase.message,
          'relative URL requires a base URL');
      expect(
          UrlError.missingScheme.toString(), "missing scheme (expected '://')");
    });

    test('effective port of unknown schemes', () {
      expect(Url.parse('gopher://h/').effectivePort(), isNull);
      expect(Url.parse('mailto:a@b').effectivePort(), isNull);
    });

    test('url equality covers every field including input', () {
      expect(Url.parse('http://h/p'), equals(Url.parse('http://h/p')));
      expect(
          Url.parse('http://h/p').hashCode, Url.parse('http://h/p').hashCode);
      expect(Url.parse('http://h/p') == Url.parse('http://h/q'), isFalse);
      expect(Url.parse('http://h/p') == Url.parse('http://h:80/p'), isFalse);
      // Same components, different preserved input: unequal.
      expect(Url.parse('HTTP://h/p') == Url.parse('http://h/p'), isFalse);
      expect(Url.parse('http://h/p') == 'http://h/p', isFalse);
      expect(Url.parse('http://h/p').toString(), 'http://h/p');
    });

    test('authority without host shows userinfo and port', () {
      final url = Url.parse('http://u@:8080/p');
      expect(url.host, isNull);
      expect(url.authority(), 'u@:8080');
    });
  });
}
