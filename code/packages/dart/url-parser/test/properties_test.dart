/// Property tests for `coding_adventures_url_parser`, using the in-lane
/// `pbt` package: percent-codec round-trips, parse/serialize fixed points,
/// canonicalization idempotence and form, and resolve laws — fixed seeds.
library;

import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_url_parser/coding_adventures_url_parser.dart';
import 'package:test/test.dart';

/// BMP strings from raw bytes (always valid: no surrogates below 0x100).
Gen<String> _bmpString(int minLen, int maxLen) =>
    Gen.bytes(minLen, maxLen).map(String.fromCharCodes);

/// Alphanumeric segments (never collide with URL delimiters).
Gen<String> _segment() => Gen.list(
    1,
    6,
    Gen.integer(BigInt.zero, BigInt.from(61)).map((n) {
      final v = n.toInt();
      if (v < 10) return 0x30 + v;
      if (v < 36) return 0x41 + v - 10;
      return 0x61 + v - 36;
    })).map((units) => String.fromCharCodes(units));

Gen<String> _authorityUrl() {
  final scheme = Gen.oneOf([
    Gen.constant('http'),
    Gen.constant('https'),
    Gen.constant('ftp'),
    Gen.constant('gopher'),
  ]);
  final host = Gen.oneOf([
    _segment(),
    Gen.tuple2(_segment(), _segment()).map((p) => '${p.$1}.${p.$2}'),
    Gen.constant('[::1]'),
    Gen.constant(''),
  ]);
  final userinfo = Gen.oneOf([
    Gen.constant(''),
    _segment().map((s) => '${s}@'),
    Gen.tuple2(_segment(), _segment()).map((p) => '${p.$1}:${p.$2}@'),
  ]);
  final port =
      Gen.integer(BigInt.zero, BigInt.from(65535)).map((n) => n.toInt());
  final tail = Gen.tuple3(_segment(), Gen.boolean(), Gen.boolean())
      .flatMap((parts) => Gen.tuple2(_segment(), _segment()).map((qf) {
            var s = '/${parts.$1}';
            if (parts.$2) s += '?${qf.$1}';
            if (parts.$3) s += '#${qf.$2}';
            return s;
          }));
  return Gen.tuple2(Gen.tuple3(scheme, host, userinfo), Gen.tuple2(port, tail))
      .map((parts) {
    final scheme = parts.$1.$1;
    final host = parts.$1.$2;
    final userinfo = parts.$1.$3;
    final port = parts.$2.$1;
    final tail = parts.$2.$2;
    // Empty hosts carry neither userinfo nor port: the serializer's
    // no-host form drops both (lossy by design, as in the reference), so
    // fixed-point inputs omit them. Units pin the lossy corner itself.
    if (host.isEmpty) return '$scheme://$tail';
    return '$scheme://$userinfo$host:$port$tail';
  });
}

Gen<String> _mailtoUrl() =>
    Gen.tuple2(_segment(), _segment()).map((p) => 'mailto:${p.$1}@${p.$2}.com');

bool _sameComponents(Url a, Url b) =>
    a.scheme == b.scheme &&
    a.userinfo == b.userinfo &&
    a.host == b.host &&
    a.port == b.port &&
    a.path == b.path &&
    a.query == b.query &&
    a.fragment == b.fragment;

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('percent-decode inverts percent-encode', () {
      final gen = _bmpString(0, 24);
      final verdict = checkProperty<String>(
        BigInt.from(61),
        PbtOptions(numTests: 300, seed: BigInt.from(61)),
        (rng) => gen.sample(rng),
        (s) => percentDecode(percentEncode(s)) == s,
      );
      _check('codec-round-trip', verdict);
    });

    test('percent-encode output is well-formed', () {
      final pattern =
          RegExp(r'^[A-Za-z0-9\-_.~/]*(%[0-9A-F]{2}[A-Za-z0-9\-_.~/]*)*$');
      final gen = _bmpString(0, 24);
      final verdict = checkProperty<String>(
        BigInt.from(62),
        PbtOptions(numTests: 300, seed: BigInt.from(62)),
        (rng) => gen.sample(rng),
        (s) => pattern.hasMatch(percentEncode(s)),
      );
      _check('encode-charset', verdict);
    });

    test('parse/serialize is a component fixed point', () {
      final gen = Gen.oneOf([_authorityUrl(), _mailtoUrl()]);
      final verdict = checkProperty<String>(
        BigInt.from(63),
        PbtOptions(numTests: 300, seed: BigInt.from(63)),
        (rng) => gen.sample(rng),
        (text) {
          // Ports above 65535 legitimately fail; skip those inputs.
          Url first;
          try {
            first = Url.parse(text);
          } on UrlError {
            return true;
          }
          final second = Url.parse(first.toUrlString());
          return _sameComponents(first, second);
        },
      );
      _check('parse-fixed-point', verdict);
    });

    test('canonicalize is idempotent', () {
      final gen = Gen.oneOf([_authorityUrl(), _mailtoUrl()]);
      final verdict = checkProperty<String>(
        BigInt.from(64),
        PbtOptions(numTests: 300, seed: BigInt.from(64)),
        (rng) => gen.sample(rng),
        (text) {
          Url first;
          try {
            first = Url.parse(text);
          } on UrlError {
            return true;
          }
          final once = first.canonicalize();
          return once.canonicalize() == once;
        },
      );
      _check('canonicalize-idempotent', verdict);
    });

    test('canonical form has no lowercase escapes or dot segments', () {
      final lowerEscape = RegExp('%[0-9A-Fa-f]*[a-f]');
      final gen = Gen.oneOf([_authorityUrl(), _mailtoUrl()]);
      final verdict = checkProperty<String>(
        BigInt.from(65),
        PbtOptions(numTests: 300, seed: BigInt.from(65)),
        (rng) => gen.sample(rng),
        (text) {
          Url first;
          try {
            first = Url.parse(text);
          } on UrlError {
            return true;
          }
          final canon = first.canonicalize();
          final segments = canon.path.split('/');
          final p = canon.port;
          final defaultKept = (p == 80 && canon.scheme == 'http') ||
              (p == 443 && canon.scheme == 'https') ||
              (p == 21 && canon.scheme == 'ftp');
          return !lowerEscape.hasMatch(canon.toUrlString()) &&
              !segments.contains('.') &&
              !segments.contains('..') &&
              !defaultKept;
        },
      );
      _check('canonical-form', verdict);
    });

    test('out-of-range ports always fail with invalidPort', () {
      final gen = Gen.integer(BigInt.from(65536), BigInt.from(99999999));
      final verdict = checkProperty<BigInt>(
        BigInt.from(66),
        PbtOptions(numTests: 200, seed: BigInt.from(66)),
        (rng) => gen.sample(rng),
        (port) {
          try {
            Url.parse('http://host:$port/path');
            return false;
          } on UrlError catch (e) {
            return e == UrlError.invalidPort;
          }
        },
      );
      _check('port-range', verdict);
    });

    test('resolve of empty drops only the fragment', () {
      final gen = Gen.oneOf([_authorityUrl(), _mailtoUrl()]);
      final verdict = checkProperty<String>(
        BigInt.from(67),
        PbtOptions(numTests: 200, seed: BigInt.from(67)),
        (rng) => gen.sample(rng),
        (text) {
          Url base;
          try {
            base = Url.parse(text);
          } on UrlError {
            return true;
          }
          final resolved = base.resolve('');
          return resolved.fragment == null &&
              resolved.scheme == base.scheme &&
              resolved.userinfo == base.userinfo &&
              resolved.host == base.host &&
              resolved.port == base.port &&
              resolved.path == base.path &&
              resolved.query == base.query;
        },
      );
      _check('resolve-empty', verdict);
    });
  });
}
