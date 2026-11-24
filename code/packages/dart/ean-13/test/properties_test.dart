/// Property tests for `coding_adventures_ean_13`, using the in-lane
/// `pbt` package: check-digit involution, parity selection, run
/// geometry, error precedence, and payload-blindness — all over hostile
/// inputs with fixed seeds.
library;

import 'package:coding_adventures_ean_13/ean_13.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

/// Twelve ASCII digits.
Gen<String> _payload12() => Gen.list(12, 12, Gen.u8()).map(
      (bytes) => bytes.map((b) => String.fromCharCode(0x30 + (b % 10))).join(),
    );

/// Hostile scalar soup: ASCII digits, letters, NUL, high bytes, and
/// surrogate halves (as raw code units, which the encoder must reject
/// before counting).
Gen<String> _hostileText() => Gen.oneOf([
      Gen.list(0, 15, Gen.u8()).map(
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
      Gen.list(0, 14, Gen.u8()).map(
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

/// Independent check-digit oracle, written straight from the GS1 rule
/// (weights 3/1 from the right, starting with 3).
String _oracleCheck(String payload12) {
  var total = 0;
  for (var index = 0; index < 12; index++) {
    final value = payload12.codeUnitAt(11 - index) - 0x30;
    total += value * (index.isEven ? 3 : 1);
  }
  return ((10 - (total % 10)) % 10).toString();
}

void main() {
  group('properties', () {
    test('check digit matches the independent oracle', () {
      final verdict = checkProperty<String>(
        BigInt.from(321),
        PbtOptions(numTests: 500, seed: BigInt.from(321)),
        (rng) => _payload12().sample(rng),
        (payload) => computeEan13CheckDigit(payload) == _oracleCheck(payload),
      );
      _check('check-oracle', verdict);
    });

    test('normalize output always revalidates', () {
      final verdict = checkProperty<String>(
        BigInt.from(322),
        PbtOptions(numTests: 500, seed: BigInt.from(322)),
        (rng) => _payload12().sample(rng),
        (payload) {
          final normalized = normalizeEan13(payload);
          if (normalized.length != 13) return false;
          return normalizeEan13(normalized) == normalized;
        },
      );
      _check('normalize-fixed-point', verdict);
    });

    test('parity selection follows the leading digit', () {
      final verdict = checkProperty<String>(
        BigInt.from(323),
        PbtOptions(numTests: 500, seed: BigInt.from(323)),
        (rng) => _payload12().sample(rng),
        (payload) {
          final normalized = normalizeEan13(payload);
          final leading = normalized.codeUnitAt(0) - 0x30;
          final parity = leftParityPattern(payload);
          if (parity != ean13LeftParityPatterns[leading]) return false;
          // The encoded left half honors the parity mix exactly.
          final encoded = encodeEan13(payload);
          for (var i = 0; i < 6; i++) {
            if (encoded[i].encoding != parity[i]) return false;
            final table = parity[i] == 'L' ? ean13LeftPatterns : ean13GPatterns;
            final value = encoded[i].digit.codeUnitAt(0) - 0x30;
            if (encoded[i].pattern != table[value]) return false;
          }
          return true;
        },
      );
      _check('parity-selection', verdict);
    });

    test('encoded runs always span 95 alternating modules', () {
      final verdict = checkProperty<String>(
        BigInt.from(324),
        PbtOptions(numTests: 300, seed: BigInt.from(324)),
        (rng) => _payload12().sample(rng),
        (payload) {
          final runs = expandEan13Runs(payload);
          var total = 0;
          for (var i = 0; i < runs.length; i++) {
            final run = runs[i];
            if (run.modules <= 0) return false;
            if (i > 0 && run.color == runs[i - 1].color) return false;
            total += run.modules;
          }
          if (total != 95) return false;
          if (runs.first.color != 'bar') return false;
          if (runs.first.sourceLabel != 'start') return false;
          if (runs.last.sourceLabel != 'end') return false;
          return runs.any((r) => r.sourceLabel == 'center');
        },
      );
      _check('run-geometry', verdict);
    });

    test('hostile inputs fail with exactly one stable id', () {
      final verdict = checkProperty<String>(
        BigInt.from(325),
        PbtOptions(numTests: 500, seed: BigInt.from(325)),
        (rng) => _hostileText().sample(rng),
        (input) {
          const ids = {
            'input-too-long',
            'invalid-length',
            'invalid-character',
            'invalid-check-digit',
          };
          try {
            final normalized = normalizeEan13(input);
            if (normalized.length != 13) return false;
            for (final unit in normalized.codeUnits) {
              if (unit < 0x30 || unit > 0x39) return false;
            }
            return true;
          } on InvalidEan13InputError catch (e) {
            return ids.contains(e.errorId);
          } on ArgumentError {
            return true;
          }
        },
      );
      _check('error-totality', verdict);
    });

    test('error precedence follows the contract order', () {
      final verdict = checkProperty<String>(
        BigInt.from(326),
        PbtOptions(numTests: 300, seed: BigInt.from(326)),
        (rng) => _payload12().sample(rng),
        (payload) {
          String? idOf(String input) {
            try {
              normalizeEan13(input);
              return null;
            } on InvalidEan13InputError catch (e) {
              return e.errorId;
            }
          }

          if (idOf('${payload}XYZ!@#${'q' * 4090}') != 'input-too-long') {
            return false;
          }
          if (idOf('A') != 'invalid-length') return false;
          if (idOf('${payload}!!') != 'invalid-length') return false;
          if (idOf('${payload}A') != 'invalid-character') return false;
          final good = normalizeEan13(payload);
          final corruptDigit = good[12] == '0' ? '1' : '0';
          if (idOf('${good.substring(0, 12)}$corruptDigit') !=
              'invalid-check-digit') {
            return false;
          }
          return true;
        },
      );
      _check('precedence', verdict);
    });

    test('messages are a pure function of the error code', () {
      final seen = <Ean13ErrorCode, String>{};
      final verdict = checkProperty<String>(
        BigInt.from(327),
        PbtOptions(numTests: 300, seed: BigInt.from(327)),
        (rng) => _hostileText().sample(rng),
        (input) {
          try {
            normalizeEan13(input);
            return true;
          } on InvalidEan13InputError catch (e) {
            if (e.message != ean13MessageFor(e.errorCode)) return false;
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
