/// Bounded fuzz campaign for `coding_adventures_test_matchers`, using the
/// in-lane `fuzz` package: hostile byte strings decode into nested values
/// (bounded depth, total decoder), and every decoded value must survive
/// conversion, printing, comparison, and self-assertion. Fixed seed, so the
/// campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_test_matchers/coding_adventures_test_matchers.dart'
    hide expect;
import 'package:test/test.dart';

/// Total byte-string decoder: every read is bounds-checked (past-end reads
/// yield tag 0, a null leaf), recursion is capped at depth 70, and the only
/// fallible construction ([RegExp]) degrades to a fallback string. The
/// decoder itself can never throw, so any campaign crash implicates the
/// library under test.
class _Decoder {
  final Uint8List input;
  final CoverageBitmap cov;
  int cursor = 0;

  _Decoder(this.input, this.cov);

  int takeByte() {
    if (cursor >= input.length) return 0;
    return input[cursor++];
  }

  Object? decodeValue(int depth) {
    final tag = takeByte() % 12;
    cov.recordEdge(tag);
    cov.recordEdge(100 + (depth > 70 ? 70 : depth));
    switch (tag) {
      case 0:
        return null;
      case 1:
        return takeByte().isOdd;
      case 2:
        return takeByte();
      case 3:
        return -takeByte();
      case 4:
        var bits = BigInt.zero;
        for (var i = 0; i < 8; i++) {
          bits |= BigInt.from(takeByte()) << (8 * i);
        }
        return f64BitsToNumber(bits);
      case 5:
        final len = takeByte();
        return String.fromCharCodes([for (var i = 0; i < len; i++) takeByte()]);
      case 6:
        final len = takeByte();
        return Uint8List.fromList([for (var i = 0; i < len; i++) takeByte()]);
      case 7:
        final len = takeByte() % 9;
        var magnitude = BigInt.zero;
        for (var i = 0; i < len; i++) {
          magnitude = (magnitude << 8) | BigInt.from(takeByte());
        }
        return takeByte().isOdd ? -magnitude : magnitude;
      case 8:
        final ms = (takeByte() << 24) |
            (takeByte() << 16) |
            (takeByte() << 8) |
            takeByte();
        return DateTime.fromMillisecondsSinceEpoch(ms);
      case 9:
        final len = takeByte() % 8;
        final source =
            String.fromCharCodes([for (var i = 0; i < len; i++) takeByte()]);
        cov.recordEdge(300 + source.length);
        try {
          return RegExp(source,
              caseSensitive: takeByte().isEven, multiLine: takeByte().isOdd);
        } on FormatException {
          // Invalid patterns are rejected by the SDK before our code ever
          // sees a RegExp object; degrade to keep the decoder total.
          return '(invalid-pattern)';
        }
      case 10 when depth < 70:
        final count = takeByte() % 5;
        return [for (var i = 0; i < count; i++) decodeValue(depth + 1)];
      case 11 when depth < 70:
        final count = takeByte() % 5;
        return {
          for (var i = 0; i < count; i++)
            decodeValue(depth + 1): decodeValue(depth + 1)
        };
      default:
        return takeByte();
    }
  }
}

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    final value = _Decoder(input, cov).decodeValue(0);
    cov.recordEdge(1000);
    final converted = toValue(value);
    cov.recordEdge(1001);
    final text = encodeValue(converted);
    cov.recordEdge(1002);
    if (text.isEmpty) {
      return const OutcomeCrash('empty-encoding', ['encodeValue-empty']);
    }
    if (!deepEqual(value, value)) {
      return const OutcomeCrash('reflexivity', ['deepEqual-self-false']);
    }
    cov.recordEdge(1003);
    // Every assertion below is a self-equality: any failure is a real bug.
    Expectation(value).toEqual(value);
    cov.recordEdge(1004);
    if (value is List) {
      Expectation(value).toHaveLength(value.length);
      if (value.isNotEmpty) Expectation(value).toContain(value.first);
    } else if (value is String) {
      Expectation(value).toContain(value);
    } else if (value is Map && value.isNotEmpty) {
      final key = value.keys.first;
      Expectation(value).toHaveProperty(key);
      Expectation(value).toContain(key);
    }
    cov.recordEdge(1005);
    return const OutcomePass();
  } on AssertionError {
    return const OutcomeCrash('self-assertion', ['matcher-self-false']);
  } on StateError {
    // Documented depth bound: structures past 64 levels.
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

/// Hand-built seeds: empty input, mixed leaves, deep and over-deep nesting
/// (the depth-bound path), maps, hostile doubles, and string escapes.
List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList([10, 4, 2, 5, 1, 2, 3, 0]),
      Uint8List.fromList([
        for (var i = 0; i < 30; i++) ...[10, 1],
        0
      ]),
      Uint8List.fromList([
        for (var i = 0; i < 70; i++) ...[10, 1],
        0
      ]),
      Uint8List.fromList([11, 2, 2, 7, 5, 3, 1, 2, 3, 0]),
      Uint8List.fromList(
          [4, 0, 0, 0, 0, 0, 0, 0xf0, 0x7f, 4, 0, 0, 0, 0, 0, 0, 0, 0x80]),
      Uint8List.fromList([5, 4, 0x22, 0x5c, 0x0a, 0x00]),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 99); longer
/// ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(99)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: no crashes, feedback edges flow', () {
      final report = runCampaign();
      expect(report.crashes, isEmpty,
          reason: report.crashes
              .map((c) =>
                  '${c.signature} ${c.kind} ${c.frames} input=${c.minimalInput}')
              .join('; '));
      expect(report.iterations, greaterThan(100));
      expect(report.totalEdges, greaterThan(10));
    });
  });
}
