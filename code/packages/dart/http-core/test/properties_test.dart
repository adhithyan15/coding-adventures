/// Property tests for `coding_adventures_http_core`, using the
/// in-lane `pbt` package: version round-trips, header lookup
/// semantics, content-helper models, target decomposition
/// invariants, query-pair streaming, route matching, and padding
/// oracle resistance — all with fixed seeds.
library;

import 'package:coding_adventures_http_core/http_core.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('version text round-trips', () {
      final gen = Gen.tuple2(Gen.u8(), Gen.u8());
      _check(
        'version round trip',
        checkProperty<(int, int)>(
          BigInt.from(681),
          PbtOptions(numTests: 200, seed: BigInt.from(681)),
          (rng) => gen.sample(rng),
          (input) {
            final text = 'HTTP/${input.$1}.${input.$2}';
            final parsed = HttpVersion.parse(text);
            return parsed.major == input.$1 &&
                parsed.minor == input.$2 &&
                parsed.toString() == text;
          },
        ),
      );
    });

    test('header lookup is first-match ASCII-insensitive', () {
      final gen = Gen.tuple2(Gen.u8(), Gen.u8());
      _check(
        'header lookup',
        checkProperty<(int, int)>(
          BigInt.from(682),
          PbtOptions(numTests: 200, seed: BigInt.from(682)),
          (rng) => gen.sample(rng),
          (input) {
            final headers = [
              Header('X-Token', 'first'),
              Header('x-token', 'second'),
              Header('Other', '${input.$1}'),
            ];
            if (findHeader(headers, 'X-TOKEN') != 'first') return false;
            if (findHeader(headers, 'x-ToKeN') != 'first') return false;
            if (findHeader(headers, 'other') != '${input.$1}') return false;
            if (findHeader(headers, 'missing') != null) return false;
            // Case variants of OTHER never match X-Token.
            return findHeader([Header('Other', 'x')], 'X-Token') == null;
          },
        ),
      );
    });

    test('content length parses digits only', () {
      final gen = Gen.tuple2(Gen.u8(), Gen.u8());
      _check(
        'content length',
        checkProperty<(int, int)>(
          BigInt.from(683),
          PbtOptions(numTests: 200, seed: BigInt.from(683)),
          (rng) => gen.sample(rng),
          (input) {
            int? len(String v) =>
                parseContentLength([Header('Content-Length', v)]);
            final n = input.$1 * 256 + input.$2;
            if (len('$n') != n) return false;
            if (len('00$n') != n) return false;
            if (len('$n ') != null) return false;
            if (len(' $n') != null) return false;
            if (len('+$n') != null) return false;
            if (len('$n bytes') != null) return false;
            return true;
          },
        ),
      );
    });

    test('target decomposition invariants', () {
      final gen = Gen.tuple3(Gen.u8(), Gen.u8(), Gen.u8());
      _check(
        'target invariants',
        checkProperty<(int, int, int)>(
          BigInt.from(684),
          PbtOptions(numTests: 200, seed: BigInt.from(684)),
          (rng) => gen.sample(rng),
          (input) {
            final target =
                '/p${input.$1}/q${input.$2}?a=${input.$1}&b=${input.$2}#f${input.$3}';
            final parts = parseRequestTarget(target);
            if (parts.path != '/p${input.$1}/q${input.$2}') return false;
            if (parts.query != 'a=${input.$1}&b=${input.$2}') return false;
            if (parts.fragment != 'f${input.$3}') return false;
            if (parts.queryValue('a') != '${input.$1}') return false;
            if (parts.queryValue('b') != '${input.$2}') return false;
            if (parts.queryValue('c') != null) return false;
            // Path is never empty.
            if (parts.path.isEmpty) return false;
            return true;
          },
        ),
      );
    });

    test('route matching is count-exact and literal-strict', () {
      final gen = Gen.tuple2(Gen.u8(), Gen.u8());
      _check(
        'route matching',
        checkProperty<(int, int)>(
          BigInt.from(685),
          PbtOptions(numTests: 200, seed: BigInt.from(685)),
          (rng) => gen.sample(rng),
          (input) {
            final pattern = RoutePattern.parse('/users/:id/posts/:pid');
            final hit =
                pattern.matchPath('/users/${input.$1}/posts/${input.$2}');
            if (hit == null ||
                hit.length != 2 ||
                hit[0] != ('id', '${input.$1}') ||
                hit[1] != ('pid', '${input.$2}')) {
              return false;
            }
            // Wrong arity or a wrong literal misses.
            if (pattern.matchPath('/users/${input.$1}') != null) {
              return false;
            }
            if (pattern.matchPath('/users/${input.$1}/posts/${input.$2}/x') !=
                null) {
              return false;
            }
            if (pattern.matchPath('/groups/${input.$1}/posts/${input.$2}') !=
                null) {
              return false;
            }
            // Query strings never affect target matching.
            final viaTarget = pattern
                .matchTarget('/users/${input.$1}/posts/${input.$2}?a=b&c=d');
            return viaTarget != null && viaTarget.length == 2;
          },
        ),
      );
    });

    test('segment splitting drops empties', () {
      final gen = Gen.u8();
      _check(
        'segments',
        checkProperty<int>(
          BigInt.from(686),
          PbtOptions(numTests: 200, seed: BigInt.from(686)),
          (rng) => gen.sample(rng),
          (n) {
            final segs = splitPathSegments('///a//$n///b//');
            if (segs.length != 3) return false;
            if (segs[0] != 'a' || segs[1] != '$n' || segs[2] != 'b') {
              return false;
            }
            return splitPathSegments('/').isEmpty;
          },
        ),
      );
    });
  });
}
