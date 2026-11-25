import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_rank_statistics/coding_adventures_rank_statistics.dart';
import 'package:test/test.dart';

import '../.dart_tool/microbenchmark_pairs_snapshot.dart';

void main() {
  test('fixture package copy matches the canonical file', () {
    expect(
        File('testdata/microbenchmark-pairs-v1.json').readAsBytesSync(),
        File('../../../specs/fixtures/microbenchmark-pairs-v1.json')
            .readAsBytesSync());
  }, skip: const bool.fromEnvironment('dart.library.js_interop'));

  test('median preserves input and uses safe midpoint at finite extremes', () {
    final values = [double.maxFinite, -double.maxFinite];
    expect(median(values), 0);
    expect(median([double.maxFinite, double.maxFinite]), double.maxFinite);
    final subnormal = double.minPositive;
    expect(median([subnormal, subnormal * 2]), subnormal * 2);
    const normalMinimum = 2.2250738585072014e-308;
    expect(median([normalMinimum, normalMinimum * 2]), normalMinimum * 1.5);
    expect(values, [double.maxFinite, -double.maxFinite]);
    expect(median([1, 2, 3, 4]), 2.5);
    expect(median([1, 2, 3]), 2);
  });

  test('rejects empty and nonfinite samples and invalid confidence', () {
    expect(() => median([]), throwsArgumentError);
    expect(() => median([double.nan]), throwsArgumentError);
    expect(() => medianInterval([1], 0), throwsArgumentError);
  });

  test('small samples agree with independent sign enumeration', () {
    for (var n = 1; n <= 12; n++) {
      final values = List<double>.generate(n, (i) => (i * 7 % 11).toDouble());
      for (var confidence = 1; confidence <= 99; confidence += 7) {
        final actual = medianInterval(values, confidence);
        var expectedRank = 0;
        for (var rank = 1; rank <= n ~/ 2 + 1; rank++) {
          if (oracleRankQualifies(n, rank, confidence)) expectedRank = rank;
        }
        if (expectedRank == 0) {
          expect(actual, isNull);
        } else {
          final sorted = List<double>.of(values)..sort();
          expect(actual!.low, sorted[expectedRank - 1]);
          expect(actual.high, sorted[n - expectedRank]);
        }
      }
    }
  });

  test('wide cutoff stays exact and endpoints come from the sample', () {
    final values = List<double>.generate(1000, (i) => i.toDouble());
    final interval = medianInterval(values, 95)!;
    expect(interval.low, 468);
    expect(interval.high, 531);
    expect(values.contains(interval.low), isTrue);
    expect(values.contains(interval.high), isTrue);
  });

  test('shared paired observations conform to medians and intervals', () {
    final fixture =
        jsonDecode(microbenchmarkPairsFixture) as Map<String, dynamic>;
    for (final item in fixture['cases'] as List<dynamic>) {
      final row = item as Map<String, dynamic>;
      final ratios = (row['candidate_ns'] as List<dynamic>)
          .map((n) => n / row['baseline_ns'])
          .cast<double>()
          .toList();
      expect(median(ratios), row['ratio'], reason: row['name']);
      final interval =
          medianInterval(ratios, fixture['confidence_percent'] as int);
      final expected = row['interval'] as List<dynamic>?;
      if (expected == null) {
        expect(interval, isNull, reason: row['name']);
      } else {
        expect([interval!.low, interval.high], expected, reason: row['name']);
      }
    }
  });
}
