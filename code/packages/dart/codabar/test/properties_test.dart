/// Property tests for `coding_adventures_codabar`, using the in-lane
/// `pbt` package: guard framing laws, option defaults, fold laws, run
/// geometry, error precedence, and payload-blindness — all over hostile
/// inputs with fixed seeds.
library;

import 'package:coding_adventures_codabar/codabar.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

const String _bodyAlphabet = '0123456789-\$:/.+';
const String _guards = 'ABCD';

/// Valid bodies, sometimes lowercase (folding only affects guards, but
/// mixed case exercises the fold path uniformly).
Gen<String> _body() => Gen.list(0, 10, Gen.u8()).map(
      (bytes) =>
          bytes.map((b) => _bodyAlphabet[b % _bodyAlphabet.length]).join(),
    );

Gen<String?> _guardOption() => Gen.oneOf([
      Gen.constant(null),
      Gen.integer(BigInt.zero, BigInt.from(3)).map((i) => _guards[i.toInt()]),
      Gen.constant('a'),
      Gen.constant('X'),
    ]);

/// Hostile scalar soup: body chars, guards, lowercase, `*`, `@`, NUL,
/// astral markers, and surrogate halves (as raw code units, which the
/// encoder must reject before counting).
Gen<String> _hostileText() => Gen.oneOf([
      Gen.list(0, 14, Gen.u8()).map(
        (bytes) => bytes
            .map((b) => String.fromCharCode([
                  0x30 + (b % 10),
                  0x41 + (b % 26),
                  0x61 + (b % 26),
                  0x2a,
                  0x40,
                  0x00,
                  0xdf,
                  0x2d,
                ][b % 8]))
            .join(),
      ),
      Gen.list(0, 13, Gen.u8()).map(
        (bytes) =>
            bytes.map((b) => String.fromCharCode(0x30 + (b % 10))).join() +
            String.fromCharCode(
                0xd800 + (bytes.isEmpty ? 0 : bytes[0] % 0x800)),
      ),
      Gen.constant('A' * 4097),
    ]);

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('unguarded bodies frame with independent defaults', () {
      final gen = Gen.tuple3(_body(), _guardOption(), _guardOption());
      final verdict = checkProperty<(String, String?, String?)>(
        BigInt.from(361),
        PbtOptions(numTests: 500, seed: BigInt.from(361)),
        (rng) => gen.sample(rng),
        (input) {
          final body = input.$1;
          // Skip option values the contract rejects; they are covered
          // by the precedence property below.
          if ((input.$2 != null && !_isGuardValue(input.$2!)) ||
              (input.$3 != null && !_isGuardValue(input.$3!))) {
            return true;
          }
          final normalized = normalizeCodabar(
            body,
            start: input.$2,
            stop: input.$3,
          );
          final wantStart = (input.$2 ?? 'A').toUpperCase();
          final wantStop = (input.$3 ?? 'A').toUpperCase();
          if (!normalized.startsWith(wantStart)) return false;
          if (!normalized.endsWith(wantStop)) return false;
          if (normalized.substring(1, normalized.length - 1) != body) {
            return false;
          }
          // Fixed point: normalizing the guarded form is stable.
          return normalizeCodabar(normalized) == normalized;
        },
      );
      _check('framing-defaults', verdict);
    });

    test('guarded input passes through with agreeing options', () {
      final gen = Gen.tuple3(_body(), Gen.u8(), Gen.u8());
      final verdict = checkProperty<(String, int, int)>(
        BigInt.from(362),
        PbtOptions(numTests: 500, seed: BigInt.from(362)),
        (rng) => gen.sample(rng),
        (input) {
          final start = _guards[input.$2 % 4];
          final stop = _guards[input.$3 % 4];
          final guarded = '$start${input.$1}$stop';
          if (normalizeCodabar(guarded) != guarded) return false;
          if (normalizeCodabar(guarded, start: start, stop: stop) != guarded) {
            return false;
          }
          // Lowercase options fold before agreement.
          return normalizeCodabar(
                guarded,
                start: start.toLowerCase(),
                stop: stop.toLowerCase(),
              ) ==
              guarded;
        },
      );
      _check('guarded-passthrough', verdict);
    });

    test('runs alternate with gaps between symbols only', () {
      final gen = Gen.tuple3(_body(), _guardOption(), _guardOption());
      final verdict = checkProperty<(String, String?, String?)>(
        BigInt.from(363),
        PbtOptions(numTests: 300, seed: BigInt.from(363)),
        (rng) => gen.sample(rng),
        (input) {
          if ((input.$2 != null && !_isGuardValue(input.$2!)) ||
              (input.$3 != null && !_isGuardValue(input.$3!))) {
            return true;
          }
          final encoded = encodeCodabar(
            input.$1,
            start: input.$2,
            stop: input.$3,
          );
          final runs = expandCodabarRuns(
            input.$1,
            start: input.$2,
            stop: input.$3,
          );
          // Gaps: exactly one between neighbors, never trailing.
          final gaps =
              runs.where((r) => r.role == 'inter-character-gap').toList();
          if (gaps.length != encoded.length - 1) return false;
          for (final gap in gaps) {
            if (gap.color != 'space' || gap.modules != 1) return false;
          }
          for (var i = 1; i < runs.length; i++) {
            if (runs[i].color == runs[i - 1].color) return false;
          }
          if (runs.first.role != 'start') return false;
          if (runs.last.role != 'stop') return false;
          return true;
        },
      );
      _check('run-geometry', verdict);
    });

    test('hostile inputs fail with exactly one stable id', () {
      final gen = Gen.tuple3(_hostileText(), _guardOption(), _guardOption());
      final verdict = checkProperty<(String, String?, String?)>(
        BigInt.from(364),
        PbtOptions(numTests: 500, seed: BigInt.from(364)),
        (rng) => gen.sample(rng),
        (input) {
          const ids = {
            'input-too-long',
            'invalid-character',
            'invalid-guard',
            'guard-conflict',
          };
          try {
            final normalized = normalizeCodabar(
              input.$1,
              start: input.$2,
              stop: input.$3,
            );
            // Success implies a fully guarded clean string.
            if (normalized.length < 2) return false;
            if (!_isGuardValue(normalized[0])) return false;
            if (!_isGuardValue(normalized[normalized.length - 1])) {
              return false;
            }
            return true;
          } on InvalidCodabarInputError catch (e) {
            return ids.contains(e.errorId);
          } on ArgumentError {
            return true;
          }
        },
      );
      _check('error-totality', verdict);
    });

    test('validation order follows the contract', () {
      final verdict = checkProperty<String>(
        BigInt.from(365),
        PbtOptions(numTests: 300, seed: BigInt.from(365)),
        (rng) => _body().sample(rng),
        (body) {
          String? idOf(String input, {String? start, String? stop}) {
            try {
              normalizeCodabar(input, start: start, stop: stop);
              return null;
            } on InvalidCodabarInputError catch (e) {
              return e.errorId;
            }
          }

          // Too-long beats everything.
          if (idOf('@' * 4097) != 'input-too-long') return false;
          // Guard form precedes the body scan.
          if (idOf('A$body@') != 'invalid-guard') return false;
          // Options precede the body scan.
          if (idOf('12*34', start: 'X') != 'invalid-guard') return false;
          // Conflict precedes the body scan.
          if (idOf('A*B', start: 'C') != 'guard-conflict') return false;
          // Fully guarded input with disagreeing options conflicts
          // even when the body is also bad.
          if (idOf('A@B', start: 'C', stop: 'C') != 'guard-conflict') {
            return false;
          }
          // Unguarded bodies report the leftmost offender.
          if (idOf('@0') != 'invalid-character') return false;
          if (idOf('0@') != 'invalid-character') return false;
          if (idOf('0A0') != 'invalid-guard') return false;
          return true;
        },
      );
      _check('precedence', verdict);
    });

    test('messages are a pure function of the error code', () {
      final seen = <CodabarErrorCode, String>{};
      final gen = Gen.tuple3(_hostileText(), _guardOption(), _guardOption());
      final verdict = checkProperty<(String, String?, String?)>(
        BigInt.from(366),
        PbtOptions(numTests: 300, seed: BigInt.from(366)),
        (rng) => gen.sample(rng),
        (input) {
          try {
            normalizeCodabar(input.$1, start: input.$2, stop: input.$3);
            return true;
          } on InvalidCodabarInputError catch (e) {
            if (e.message != codabarMessageFor(e.errorCode)) return false;
            final first = seen[e.errorCode];
            if (first == null) {
              seen[e.errorCode] = e.toString();
              return true;
            }
            return first == e.toString();
          } on ArgumentError {
            return true;
          }
        },
      );
      _check('payload-blindness', verdict);
    });
  });
}

bool _isGuardValue(String value) =>
    value.length == 1 && _guards.contains(value.toUpperCase());
