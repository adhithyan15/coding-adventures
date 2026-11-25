/// Property tests for `coding_adventures_code_128`, using the in-lane
/// `pbt` package: checksum oracle, framing invariants, run geometry,
/// error totality, and payload-blindness — all over hostile inputs with
/// fixed seeds.
library;

import 'package:coding_adventures_code_128/code_128.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

/// Printable ASCII payloads (space through tilde).
Gen<String> _payload() => Gen.list(0, 24, Gen.u8()).map(
      (bytes) => bytes.map((b) => String.fromCharCode(0x20 + (b % 95))).join(),
    );

/// Hostile scalar soup: printable ASCII, controls, DEL, NUL, é, and
/// surrogate halves (as raw code units, which the encoder must reject
/// before counting).
Gen<String> _hostileText() => Gen.oneOf([
      Gen.list(0, 16, Gen.u8()).map(
        (bytes) => bytes
            .map((b) => String.fromCharCode([
                  0x20 + (b % 95),
                  0x00,
                  0x1f,
                  0x7f,
                  0xe9,
                ][b % 5]))
            .join(),
      ),
      Gen.list(0, 14, Gen.u8()).map(
        (bytes) =>
            bytes.map((b) => String.fromCharCode(0x41 + (b % 26))).join() +
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

/// Independent checksum oracle, written straight from the ISO rule.
int _oracleChecksum(List<int> values) {
  var sum = 104;
  for (var i = 0; i < values.length; i++) {
    sum += values[i] * (i + 1);
  }
  return sum % 103;
}

void main() {
  group('properties', () {
    test('checksum matches the independent oracle', () {
      final verdict = checkProperty<String>(
        BigInt.from(381),
        PbtOptions(numTests: 500, seed: BigInt.from(381)),
        (rng) => _payload().sample(rng),
        (payload) {
          final values = payload.codeUnits.map((u) => u - 32).toList();
          if (computeCode128Checksum(values) != _oracleChecksum(values)) {
            return false;
          }
          final encoded = encodeCode128B(payload);
          return encoded[encoded.length - 2].value == _oracleChecksum(values);
        },
      );
      _check('check-oracle', verdict);
    });

    test('framing wraps data in start, check, and stop', () {
      final verdict = checkProperty<String>(
        BigInt.from(382),
        PbtOptions(numTests: 500, seed: BigInt.from(382)),
        (rng) => _payload().sample(rng),
        (payload) {
          final encoded = encodeCode128B(payload);
          if (encoded.length != payload.length + 3) return false;
          if (encoded.first.role != 'start') return false;
          if (encoded.last.role != 'stop') return false;
          if (encoded[encoded.length - 2].role != 'check') return false;
          if (encoded.first.value != 104) return false;
          if (encoded.last.value != 106) return false;
          for (var i = 1; i <= payload.length; i++) {
            if (encoded[i].role != 'data') return false;
            if (encoded[i].sourceIndex != i - 1) return false;
          }
          // Fixed point: normalizing is the identity on valid input.
          return normalizeCode128B(payload) == payload;
        },
      );
      _check('framing', verdict);
    });

    test('runs concatenate gapless with strict alternation', () {
      final verdict = checkProperty<String>(
        BigInt.from(383),
        PbtOptions(numTests: 300, seed: BigInt.from(383)),
        (rng) => _payload().sample(rng),
        (payload) {
          final encoded = encodeCode128B(payload);
          final runs = expandCode128Runs(payload);
          // No inter-character gaps in Code 128.
          for (final run in runs) {
            if (run.role == 'inter-character-gap') return false;
          }
          for (var i = 1; i < runs.length; i++) {
            if (runs[i].color == runs[i - 1].color) return false;
          }
          if (runs.first.color != 'bar') return false;
          // Module total: 11 per symbol plus 2 extra stop modules.
          final total = runs.fold<int>(0, (sum, run) => sum + run.modules);
          if (total != encoded.length * 11 + 2) return false;
          return true;
        },
      );
      _check('run-geometry', verdict);
    });

    test('hostile inputs fail with exactly one stable id', () {
      final verdict = checkProperty<String>(
        BigInt.from(384),
        PbtOptions(numTests: 500, seed: BigInt.from(384)),
        (rng) => _hostileText().sample(rng),
        (input) {
          const ids = {'input-too-long', 'invalid-character'};
          try {
            final normalized = normalizeCode128B(input);
            if (normalized != input) return false;
            for (final unit in normalized.codeUnits) {
              if (unit < 0x20 || unit > 0x7e) return false;
            }
            return true;
          } on InvalidCode128InputError catch (e) {
            return ids.contains(e.errorId);
          } on ArgumentError {
            return true;
          }
        },
      );
      _check('error-totality', verdict);
    });

    test('cap precedes the alphabet scan', () {
      final verdict = checkProperty<String>(
        BigInt.from(385),
        PbtOptions(numTests: 200, seed: BigInt.from(385)),
        (rng) => _payload().sample(rng),
        (payload) {
          String? idOf(String input) {
            try {
              normalizeCode128B(input);
              return null;
            } on InvalidCode128InputError catch (e) {
              return e.errorId;
            }
          }

          if (idOf('@' * 4097) != 'input-too-long') return false;
          if (idOf('A' * 4097) != 'input-too-long') return false;
          if (idOf((payload + String.fromCharCode(0))) != 'invalid-character')
            return false;
          if (idOf('$payload\x7f') != 'invalid-character') return false;
          if (idOf('${payload}\u{E9}') != 'invalid-character') return false;
          return true;
        },
      );
      _check('precedence', verdict);
    });

    test('messages are a pure function of the error code', () {
      final seen = <Code128ErrorCode, String>{};
      final verdict = checkProperty<String>(
        BigInt.from(386),
        PbtOptions(numTests: 300, seed: BigInt.from(386)),
        (rng) => _hostileText().sample(rng),
        (input) {
          try {
            normalizeCode128B(input);
            return true;
          } on InvalidCode128InputError catch (e) {
            if (e.message != code128MessageFor(e.errorCode)) return false;
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
