/// Property tests for `coding_adventures_code_39`, using the in-lane
/// `pbt` package: folding laws, framing invariants, run geometry,
/// error precedence, and payload-blindness — all over hostile inputs
/// with fixed seeds.
library;

import 'package:coding_adventures_code_39/code_39.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

const String _alphabet = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. \$/%+';

/// Valid payloads, sometimes lowercase (to exercise folding).
Gen<String> _payload() => Gen.list(0, 12, Gen.u8()).map(
      (bytes) => bytes.map((b) {
        final ch = _alphabet[b % _alphabet.length];
        if (b % 3 == 0 &&
            ch.codeUnitAt(0) >= 0x41 &&
            ch.codeUnitAt(0) <= 0x5a) {
          return String.fromCharCode(ch.codeUnitAt(0) + 0x20);
        }
        return ch;
      }).join(),
    );

/// Hostile scalar soup: alphabet chars, lowercase, `*`, `@`, NUL,
/// astral markers, and surrogate halves (as raw code units, which the
/// encoder must reject before counting).
Gen<String> _hostileText() => Gen.oneOf([
      Gen.list(0, 14, Gen.u8()).map(
        (bytes) => bytes
            .map((b) => String.fromCharCode([
                  0x41 + (b % 26),
                  0x61 + (b % 26),
                  0x2a,
                  0x40,
                  0x00,
                  0xdf,
                  0x20,
                ][b % 7]))
            .join(),
      ),
      Gen.list(0, 13, Gen.u8()).map(
        (bytes) =>
            bytes.map((b) => String.fromCharCode(0x41 + (b % 26))).join() +
            String.fromCharCode(
                0xd800 + (bytes.isEmpty ? 0 : bytes[0] % 0x800)),
      ),
      Gen.constant('*'),
      Gen.constant('A' * 4097),
    ]);

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('normalize folds ASCII and frames nothing', () {
      final verdict = checkProperty<String>(
        BigInt.from(341),
        PbtOptions(numTests: 500, seed: BigInt.from(341)),
        (rng) => _payload().sample(rng),
        (payload) {
          final normalized = normalizeCode39(payload);
          if (normalized.length != payload.length) return false;
          for (var i = 0; i < payload.length; i++) {
            final unit = payload.codeUnitAt(i);
            final folded = unit >= 0x61 && unit <= 0x7a ? unit - 0x20 : unit;
            if (normalized.codeUnitAt(i) != folded) return false;
          }
          // Fixed point: normalizing twice is stable.
          return normalizeCode39(normalized) == normalized;
        },
      );
      _check('fold-laws', verdict);
    });

    test('encode frames every payload in start/stop', () {
      final verdict = checkProperty<String>(
        BigInt.from(342),
        PbtOptions(numTests: 500, seed: BigInt.from(342)),
        (rng) => _payload().sample(rng),
        (payload) {
          final normalized = normalizeCode39(payload);
          final encoded = encodeCode39(payload);
          if (encoded.length != normalized.length + 2) return false;
          if (!encoded.first.isStartStop || !encoded.last.isStartStop) {
            return false;
          }
          if (encoded.first.ch != '*' || encoded.last.ch != '*') {
            return false;
          }
          for (var i = 1; i <= normalized.length; i++) {
            if (encoded[i].ch != normalized[i - 1]) return false;
            if (encoded[i].isStartStop) return false;
            if (encoded[i].pattern.length != 9) return false;
          }
          return true;
        },
      );
      _check('framing', verdict);
    });

    test('runs alternate with gaps between symbols only', () {
      final verdict = checkProperty<String>(
        BigInt.from(343),
        PbtOptions(numTests: 300, seed: BigInt.from(343)),
        (rng) => _payload().sample(rng),
        (payload) {
          final encoded = encodeCode39(payload);
          final runs = expandCode39Runs(payload);
          // Nine elements per char plus one gap between neighbors.
          if (runs.length != encoded.length * 9 + (encoded.length - 1)) {
            return false;
          }
          var cursor = 0;
          for (var c = 0; c < encoded.length; c++) {
            for (var e = 0; e < 9; e++) {
              final run = runs[cursor++];
              if (run.modules <= 0) return false;
              if (run.sourceIndex != c) return false;
              if (run.sourceLabel != encoded[c].ch) return false;
            }
            if (c < encoded.length - 1) {
              final gap = runs[cursor++];
              if (gap.role != 'inter-character-gap') return false;
              if (gap.color != 'space' || gap.modules != 1) return false;
            }
          }
          for (var i = 1; i < runs.length; i++) {
            if (runs[i].color == runs[i - 1].color) return false;
          }
          // Start/stop roles anchor the ends.
          if (runs.first.role != 'start') return false;
          if (runs.last.role != 'stop') return false;
          return true;
        },
      );
      _check('run-geometry', verdict);
    });

    test('hostile inputs fail with exactly one stable id', () {
      final verdict = checkProperty<String>(
        BigInt.from(344),
        PbtOptions(numTests: 500, seed: BigInt.from(344)),
        (rng) => _hostileText().sample(rng),
        (input) {
          const ids = {
            'input-too-long',
            'invalid-character',
            'reserved-character',
          };
          try {
            final normalized = normalizeCode39(input);
            for (var i = 0; i < normalized.length; i++) {
              if (!code39Patterns.containsKey(normalized[i]) ||
                  normalized[i] == '*') {
                return false;
              }
            }
            return true;
          } on InvalidCode39InputError catch (e) {
            return ids.contains(e.errorId);
          } on ArgumentError {
            return true;
          }
        },
      );
      _check('error-totality', verdict);
    });

    test('scan reports the leftmost offender', () {
      final verdict = checkProperty<String>(
        BigInt.from(345),
        PbtOptions(numTests: 300, seed: BigInt.from(345)),
        (rng) => _payload().sample(rng),
        (payload) {
          String? idOf(String input) {
            try {
              normalizeCode39(input);
              return null;
            } on InvalidCode39InputError catch (e) {
              return e.errorId;
            }
          }

          // Too-long beats the scan, whatever the scan would say.
          if (idOf('@' * 4097) != 'input-too-long') return false;
          if (idOf('*' * 4097) != 'input-too-long') return false;
          // Leftmost offender wins, either kind.
          if (idOf('@*') != 'invalid-character') return false;
          if (idOf('*@') != 'reserved-character') return false;
          if (idOf('${payload}*') != 'reserved-character') return false;
          if (idOf('${payload}@') != 'invalid-character') return false;
          // Lowercase folds instead of failing.
          if (idOf('${payload.toLowerCase()}') != null) return false;
          // But \u{DF} never folds.
          if (idOf('${payload}\u{DF}') != 'invalid-character') return false;
          return true;
        },
      );
      _check('scan-order', verdict);
    });

    test('messages are a pure function of the error code', () {
      final seen = <Code39ErrorCode, String>{};
      final verdict = checkProperty<String>(
        BigInt.from(346),
        PbtOptions(numTests: 300, seed: BigInt.from(346)),
        (rng) => _hostileText().sample(rng),
        (input) {
          try {
            normalizeCode39(input);
            return true;
          } on InvalidCode39InputError catch (e) {
            if (e.message != code39MessageFor(e.errorCode)) return false;
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
