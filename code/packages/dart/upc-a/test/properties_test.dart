/// Property tests for `coding_adventures_upc_a`, using the in-lane
/// `pbt` package: check-digit involution, normalize/encode agreement,
/// run geometry invariants, error precedence, and payload-blindness —
/// all over hostile inputs with fixed seeds.
library;

import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_upc_a/upc_a.dart';
import 'package:test/test.dart';

/// Eleven ASCII digits.
Gen<String> _payload11() => Gen.list(11, 11, Gen.u8()).map(
      (bytes) => bytes.map((b) => String.fromCharCode(0x30 + (b % 10))).join(),
    );

/// Hostile scalar soup: ASCII digits, letters, NUL, high bytes, astral
/// markers, and surrogate halves (as raw code units, which the encoder
/// must reject before counting).
Gen<String> _hostileText() => Gen.oneOf([
      Gen.list(0, 14, Gen.u8()).map(
        (bytes) => bytes
            .map((b) => String.fromCharCode([
                  0x30 + (b % 10),
                  0x41 + (b % 26),
                  0x00,
                  0x7f,
                  0xe9,
                  0x20,
                ][b % 6]))
            .join(),
      ),
      Gen.list(0, 13, Gen.u8()).map(
        (bytes) =>
            bytes.map((b) => String.fromCharCode(b % 10 + 0x30)).join() +
            String.fromCharCode(
                0xd800 + (bytes.isEmpty ? 0 : bytes[0] % 0x800)),
      ),
      Gen.constant('A' * 4097),
      Gen.constant('0' * 4096),
    ]);

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

/// Independent check-digit oracle, written straight from the GS1 rule.
String _oracleCheck(String payload11) {
  var oddSum = 0;
  var evenSum = 0;
  for (var i = 0; i < 11; i++) {
    final value = payload11.codeUnitAt(i) - 0x30;
    if (i.isEven) {
      oddSum += value;
    } else {
      evenSum += value;
    }
  }
  return (((10 - ((oddSum * 3 + evenSum) % 10)) % 10)).toString();
}

void main() {
  group('properties', () {
    test('check digit matches the independent oracle', () {
      final verdict = checkProperty<String>(
        BigInt.from(301),
        PbtOptions(numTests: 500, seed: BigInt.from(301)),
        (rng) => _payload11().sample(rng),
        (payload) => computeUpcACheckDigit(payload) == _oracleCheck(payload),
      );
      _check('check-oracle', verdict);
    });

    test('normalize output always revalidates', () {
      final verdict = checkProperty<String>(
        BigInt.from(302),
        PbtOptions(numTests: 500, seed: BigInt.from(302)),
        (rng) => _payload11().sample(rng),
        (payload) {
          final normalized = normalizeUpcA(payload);
          if (normalized.length != 12) return false;
          // Re-normalizing is a fixed point (valid 12-digit input).
          return normalizeUpcA(normalized) == normalized;
        },
      );
      _check('normalize-fixed-point', verdict);
    });

    test('encoded runs always span 95 alternating modules', () {
      final verdict = checkProperty<String>(
        BigInt.from(303),
        PbtOptions(numTests: 300, seed: BigInt.from(303)),
        (rng) => _payload11().sample(rng),
        (payload) {
          final runs = expandUpcARuns(payload);
          var total = 0;
          for (var i = 0; i < runs.length; i++) {
            final run = runs[i];
            if (run.modules <= 0) return false;
            if (i > 0 && run.color == runs[i - 1].color) return false;
            total += run.modules;
          }
          if (total != 95) return false;
          if (runs.first.color != 'bar') return false;
          // Guards anchor both ends and the center.
          if (runs.first.sourceLabel != 'start') return false;
          if (runs.last.sourceLabel != 'end') return false;
          return runs.any((r) => r.sourceLabel == 'center');
        },
      );
      _check('run-geometry', verdict);
    });

    test('encode/expand agree digit by digit', () {
      final verdict = checkProperty<String>(
        BigInt.from(304),
        PbtOptions(numTests: 300, seed: BigInt.from(304)),
        (rng) => _payload11().sample(rng),
        (payload) {
          final encoded = encodeUpcA(payload);
          if (encoded.length != 12) return false;
          for (var i = 0; i < 12; i++) {
            final entry = encoded[i];
            if (entry.pattern.length != 7) return false;
            if (entry.sourceIndex != i) return false;
            if ((entry.encoding == 'L') != (i < 6)) return false;
            if ((entry.role == 'check') != (i == 11)) return false;
            final table = i < 6 ? upcALeftPatterns : upcARightPatterns;
            final value = entry.digit.codeUnitAt(0) - 0x30;
            if (entry.pattern != table[value]) return false;
          }
          return true;
        },
      );
      _check('encode-agreement', verdict);
    });

    test('hostile inputs fail with exactly one stable id', () {
      final verdict = checkProperty<String>(
        BigInt.from(305),
        PbtOptions(numTests: 500, seed: BigInt.from(305)),
        (rng) => _hostileText().sample(rng),
        (input) {
          const ids = {
            'input-too-long',
            'invalid-length',
            'invalid-character',
            'invalid-check-digit',
          };
          try {
            final normalized = normalizeUpcA(input);
            // Success implies a clean 12-digit payload.
            if (normalized.length != 12) return false;
            for (final unit in normalized.codeUnits) {
              if (unit < 0x30 || unit > 0x39) return false;
            }
            return true;
          } on InvalidUpcAInputError catch (e) {
            return ids.contains(e.errorId);
          } on ArgumentError {
            // Host-string rejection (lone surrogates): outside v1,
            // must still be an ArgumentError, never anything else.
            return true;
          }
        },
      );
      _check('error-totality', verdict);
    });

    test('error precedence follows the contract order', () {
      final verdict = checkProperty<String>(
        BigInt.from(306),
        PbtOptions(numTests: 300, seed: BigInt.from(306)),
        (rng) => _payload11().sample(rng),
        (payload) {
          String? idOf(String input) {
            try {
              normalizeUpcA(input);
              return null;
            } on InvalidUpcAInputError catch (e) {
              return e.errorId;
            }
          }

          // Too-long beats everything, even with bad chars and length.
          if (idOf('${payload}XYZ!@#${'q' * 4090}') != 'input-too-long') {
            return false;
          }
          // Bad length beats bad characters.
          if (idOf('A') != 'invalid-length') return false;
          if (idOf('${payload}!!') != 'invalid-length') return false;
          // Bad characters beat check-digit verification.
          if (idOf('${payload}A') != 'invalid-character') return false;
          // A correct payload with a corrupt check digit reports it.
          final good = normalizeUpcA(payload);
          final corruptDigit = good[11] == '0' ? '1' : '0';
          if (idOf('${good.substring(0, 11)}$corruptDigit') !=
              'invalid-check-digit') {
            return false;
          }
          return true;
        },
      );
      _check('precedence', verdict);
    });

    test('messages are a pure function of the error code', () {
      // Payload-blindness: the same code renders identically no matter
      // which hostile bytes triggered it.
      final seen = <UpcAErrorCode, String>{};
      final verdict = checkProperty<String>(
        BigInt.from(307),
        PbtOptions(numTests: 300, seed: BigInt.from(307)),
        (rng) => _hostileText().sample(rng),
        (input) {
          try {
            normalizeUpcA(input);
            return true;
          } on InvalidUpcAInputError catch (e) {
            if (e.message != upcAMessageFor(e.errorCode)) return false;
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
