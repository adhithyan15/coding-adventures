import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_microbenchmark/coding_adventures_microbenchmark.dart';
import 'package:test/test.dart';

import '../.dart_tool/microbenchmark_pairs_snapshot.dart';

void main() {
  test('fixture package copy matches the canonical input', () {
    expect(
        File('testdata/microbenchmark-pairs-v1.json').readAsBytesSync(),
        File('../../../specs/fixtures/microbenchmark-pairs-v1.json')
            .readAsBytesSync());
  }, skip: const bool.fromEnvironment('dart.library.js_interop'));

  test('all shared observation cases retain their classifications', () {
    final fixture =
        jsonDecode(microbenchmarkPairsFixture) as Map<String, dynamic>;
    for (final item in fixture['cases'] as List<dynamic>) {
      final row = item as Map<String, dynamic>;
      final values = row['candidate_ns'] as List<dynamic>;
      final pairs = values
          .asMap()
          .entries
          .map((entry) => Pair(
                BigInt.from(row['baseline_ns'] as int),
                BigInt.from(entry.value as int),
                entry.key.isEven ? 'baseline-first' : 'candidate-first',
              ))
          .toList();
      final result = analyze(pairs, fixture['confidence_percent'] as int,
          (fixture['minimum_factor'] as num).toDouble());
      expect(result.verdict, row['verdict'], reason: row['name'] as String);
      expect(result.ratio, (row['ratio'] as num).toDouble(),
          reason: row['name'] as String);
    }
  });

  test('analysis accepts 10000 pairs and refuses 10001 before statistics', () {
    final maximum = List<Pair>.generate(
      10000,
      (index) => Pair(BigInt.one, BigInt.one,
          index.isEven ? 'baseline-first' : 'candidate-first'),
    );
    expect(analyze(maximum, 95, 1.05).ratio, 1.0);
    expect(
      () => analyze(
          [...maximum, Pair(BigInt.one, BigInt.one, 'invalid')], 95, 1.05),
      throwsArgumentError,
    );
  });

  test('both public workload variants read every caller value', () {
    final values = makeValues();
    expect(indexedBatch(values, BigInt.one), BigInt.from(511533));
    expect(iteratorBatch(values, BigInt.one), BigInt.from(511533));
    values[0] = BigInt.from(700);
    expect(indexedBatch(values, BigInt.one), BigInt.from(512230));
    expect(iteratorBatch(values, BigInt.one), BigInt.from(512230));
    expect(() => indexedBatch(values, BigInt.from(-1)), throwsArgumentError);
    expect(() => iteratorBatch(values, BigInt.from(-1)), throwsArgumentError);
    expect(() => indexedBatch([], BigInt.one), throwsArgumentError);
    final overflowing = [BigInt.parse('9223372036854775807'), BigInt.one];
    expect(() => indexedBatch(overflowing, BigInt.one), throwsRangeError);
    expect(() => iteratorBatch(overflowing, BigInt.one), throwsRangeError);
    final overflowingThenCancel = [...overflowing, -BigInt.one];
    expect(() => indexedBatch(overflowingThenCancel, BigInt.one),
        throwsRangeError);
    expect(() => iteratorBatch(overflowingThenCancel, BigInt.one),
        throwsRangeError);
    expect(
        () =>
            indexedBatch([BigInt.parse('4611686018427387903')], BigInt.from(3)),
        throwsRangeError);
    expect(
        () => iteratorBatch(
            [BigInt.parse('4611686018427387903')], BigInt.from(3)),
        throwsRangeError);
    expect(() => indexedBatch(values, BigInt.parse('9223372036854775808')),
        throwsArgumentError);
  });

  test('fake clock collection schedules, analyzes, and exactly replays', () {
    var tick = BigInt.zero;
    BigInt now() {
      final previous = tick;
      tick += BigInt.one;
      return previous;
    }

    BigInt batch(BigInt n) => BigInt.from(42) * n;
    final report = collect(
      config: Config(
          pairs: 3,
          confidencePercent: 95,
          targetNs: BigInt.one,
          maxBatch: BigInt.one),
      expectedChecksum: BigInt.from(42),
      baseline: batch,
      candidate: batch,
      now: now,
      metadata: const {
        'workload': 'test',
        'invocation': <String>[],
        'revision': 'abc',
        'runtime': 'Dart',
        'os': 'test',
        'arch': 'test'
      },
    );
    expect(
        report.pairs.where((p) => p.order == 'baseline-first'), hasLength(2));
    final encoded = report.toJson();
    expect(Report.fromJson(encoded).pairs, hasLength(3));
    String withWorkloadToken(String token) =>
        encoded.replaceFirst('"workload":"test"', '"workload":$token');
    for (final workload in [
      'indexed-array-sum-4096',
      'dependent-link-order',
      '\u2003'
    ]) {
      final replayed = Report.fromJson(withWorkloadToken(jsonEncode(workload)));
      expect(replayed.metadata['workload'], workload);
    }
    for (final entry in <(String, String)>[
      (r'"\ud83d\udca1"', '\u{1f4a1}'),
      (r'"\ufffd"', '\uFFFD'),
      (r'"\\ud800"', r'\ud800'),
    ]) {
      final replayed = Report.fromJson(withWorkloadToken(entry.$1));
      expect(replayed.metadata['workload'], entry.$2);
    }
    for (final invalid in [r'"\ud800"', r'"\udfff"']) {
      expect(() => Report.fromJson(withWorkloadToken(invalid)),
          throwsFormatException,
          reason: 'accepted invalid surrogate $invalid');
    }
    for (final workload in [
      '',
      ' ',
      '\t',
      '\n',
      '\u000b',
      '\f',
      '\r',
      '\u0000',
      'bad\u0000id'
    ]) {
      expect(() => Report.fromJson(withWorkloadToken(jsonEncode(workload))),
          throwsFormatException);
    }
    for (final invalid in ['null', '17', 'true', '[]', '{}']) {
      expect(() => Report.fromJson(withWorkloadToken(invalid)),
          throwsFormatException);
    }
    final aboveJsSafe = Report.fromJson(
        encoded.replaceFirst('"seed":0', '"seed":9007199254740993'));
    expect(aboveJsSafe.config.seed, BigInt.parse('9007199254740993'));
    final minimum = Report.fromJson(
        encoded.replaceFirst('"seed":0', '"seed":-9223372036854775808'));
    expect(minimum.config.seed, BigInt.parse('-9223372036854775808'));
    final maximum = Report.fromJson(
        encoded.replaceFirst('"seed":0', '"seed":9223372036854775807'));
    expect(maximum.config.seed, BigInt.parse('9223372036854775807'));
    final integerFactor = Report.fromJson(
        encoded.replaceFirst('"minimum_factor":1.05', '"minimum_factor":2'));
    expect(integerFactor.config.minimumFactor, 2.0);
    final maximumFactor = Report.fromJson(encoded.replaceFirst(
        '"minimum_factor":1.05', '"minimum_factor":9223372036854775807'));
    expect(maximumFactor.config.minimumFactor,
        BigInt.parse('9223372036854775807').toDouble());
    expect(
        () => Report.fromJson(encoded.replaceFirst(
            '"minimum_factor":1.05', '"minimum_factor":-9223372036854775808')),
        throwsArgumentError);
    final tooLargeFactor = '1${List<String>.filled(400, '0').join()}';
    expect(
        () => Report.fromJson(encoded.replaceFirst(
            '"minimum_factor":1.05', '"minimum_factor":$tooLargeFactor')),
        throwsFormatException);
    final maximumDuration = Report.fromJson(encoded
        .replaceFirst('"baseline_ns":1', '"baseline_ns":9223372036854775807')
        .replaceFirst(
            '"candidate_ns":1', '"candidate_ns":9223372036854775807'));
    expect(maximumDuration.pairs.first.baselineNs,
        BigInt.parse('9223372036854775807'));
    expect(() => Report.fromJson(encoded.replaceFirst('"seed":0', '"seed":-0')),
        throwsFormatException);
    expect(
        () => Report.fromJson(
            encoded.replaceFirst('"seed":0', '"seed":9223372036854775808')),
        throwsFormatException);
    expect(
        () => Report.fromJson(encoded.replaceFirst(
            '"workload":"test"', '"workload":"test","\\u0077orkload":"evil"')),
        throwsFormatException);
    expect(
        () =>
            analyze([Pair(BigInt.one, BigInt.one, 'baseline-first')], 95, 1.05),
        returnsNormally);
  });

  test('collect fails atomically on callback checksum and clock errors', () {
    var tick = BigInt.zero;
    BigInt now() => tick += BigInt.one;
    final config = Config(pairs: 2, targetNs: BigInt.one, maxBatch: BigInt.one);
    expect(
        () => collect(
            config: config,
            expectedChecksum: BigInt.one,
            baseline: (_) => BigInt.zero,
            candidate: (_) => BigInt.one,
            now: now),
        throwsStateError);
    tick = BigInt.zero;
    expect(
        () => collect(
            config: config,
            expectedChecksum: BigInt.one,
            baseline: (_) => BigInt.one,
            candidate: (_) => BigInt.one,
            now: () => BigInt.from(-1)),
        throwsStateError);
  });

  test('zero seed maps to a reproducible shuffled xorshift state', () {
    List<String> orders() {
      var tick = BigInt.zero;
      BigInt clock() {
        final value = tick;
        tick += BigInt.one;
        return value;
      }

      return collect(
              config:
                  Config(pairs: 40, targetNs: BigInt.one, maxBatch: BigInt.one),
              expectedChecksum: BigInt.one,
              baseline: (n) => n,
              candidate: (n) => n,
              now: clock)
          .pairs
          .map((pair) => pair.order)
          .toList();
    }

    final first = orders();
    expect(orders(), first);
    expect(first.where((order) => order == 'baseline-first'), hasLength(20));
    expect(first.where((order) => order == 'candidate-first'), hasLength(20));
    expect(
        first,
        isNot(equals(List<String>.generate(40,
            (index) => index.isEven ? 'baseline-first' : 'candidate-first'))));
  });

  test('rejects parameter bounds, unbalanced data, and calibration exhaustion',
      () {
    expect(() => Config(pairs: 0).validate(), throwsArgumentError);
    expect(
        () => Config(confidencePercent: 100).validate(), throwsArgumentError);
    expect(() => Config(minimumFactor: double.infinity).validate(),
        throwsArgumentError);
    expect(() => Config(targetNs: BigInt.zero).validate(), throwsArgumentError);
    expect(() => analyze([], 95, 1.05), throwsArgumentError);
    expect(() => analyze([Pair(BigInt.one, BigInt.one, 'bad')], 95, 1.05),
        throwsArgumentError);
    expect(
        () => analyze([
              Pair(BigInt.one, BigInt.one, 'baseline-first'),
              Pair(BigInt.one, BigInt.one, 'baseline-first'),
            ], 95, 1.05),
        throwsArgumentError);
    var tick = BigInt.zero;
    BigInt now() {
      final result = tick;
      tick += BigInt.one;
      return result;
    }

    expect(
        () => collect(
            config: Config(
                pairs: 1, targetNs: BigInt.from(3), maxBatch: BigInt.one),
            expectedChecksum: BigInt.one,
            baseline: (n) => n,
            candidate: (n) => n,
            now: now),
        throwsStateError);
  });

  test('replay rejects malformed schemas and forged analysis within bounds',
      () {
    var tick = BigInt.zero;
    BigInt now() {
      final value = tick;
      tick += BigInt.one;
      return value;
    }

    BigInt batch(BigInt n) => n;
    final report = collect(
        config: Config(pairs: 1, targetNs: BigInt.one, maxBatch: BigInt.one),
        expectedChecksum: BigInt.one,
        baseline: batch,
        candidate: batch,
        now: now,
        metadata: const {
          'workload': 'test',
          'invocation': <String>[],
          'revision': 'abc',
          'runtime': 'Dart',
          'os': 'test',
          'arch': 'test'
        });
    final encoded = report.toJson();
    expect(() => Report.fromJson('[]'), throwsFormatException);
    expect(() => Report.fromJson(''), throwsFormatException);
    expect(
        () => Report.fromJson(
            encoded.replaceFirst(RegExp(r'"ratio":1(?:\.0)?'), '"ratio":0.5')),
        throwsFormatException);
    expect(
        () => Report.fromJson(
            encoded.replaceFirst(RegExp(r'"ratio":1(?:\.0)?'), '"ratio":1')),
        returnsNormally);
    expect(
        () => Report.fromJson(
            encoded.replaceFirst(RegExp(r'"ratio":1(?:\.0)?'), '"ratio":"1"')),
        throwsFormatException);
    expect(
        () => Report.fromJson(
            encoded.replaceFirst('"config":', '"extra":0,"config":')),
        throwsFormatException);
    expect(
        () => Report.fromJson(encoded.replaceFirst('"pairs":1', '"pairs":2')),
        throwsFormatException);
    expect(
        () => Report.fromJson(
            encoded.replaceFirst('"seed":0', '"seed":0,"junk":1')),
        throwsFormatException);
    expect(
        () => Report.fromJson(encoded.replaceFirst(
            '"minimum_factor":1.05', '"minimum_factor":"bad"')),
        throwsFormatException);
    expect(
        () => Report.fromJson(
            encoded.replaceFirst('"batch_size":1', '"batch_size":0')),
        throwsFormatException);
    expect(
        () => Report.fromJson(encoded.replaceFirst(
            '"order":"baseline-first"', '"order":"candidate-first"')),
        throwsArgumentError);
    expect(
        () => Report.fromJson(encoded.replaceFirst(
            '"order":"baseline-first"', '"order":"baseline-first","extra":0')),
        throwsFormatException);
    expect(
        () => Report.fromJson(
            encoded.replaceFirst('"workload":"test"', '"workload":""')),
        throwsFormatException);
    expect(
        () => Report.fromJson(
            encoded.replaceFirst('"invocation":[]', '"invocation":0')),
        throwsFormatException);
    expect(
        () => Report.fromJson(encoded.replaceFirst(
            '"analysis":{"ratio":', '"analysis":{"extra":0,"ratio":')),
        throwsFormatException);
    expect(
        () => Report.fromJson(
            encoded.replaceFirst('"interval":null', '"interval":[]')),
        throwsFormatException);
    expect(
        () => Report.fromJson(encoded.replaceFirst(
            '"interval":null', '"interval":[1e999,1e999]')),
        throwsFormatException);
    expect(
        () => Report.fromJson(
            encoded.replaceFirst('"interval":null', '"interval":[1.0,2.0]')),
        throwsFormatException);
    expect(() => Report.fromJson('{"config" 1}'), throwsFormatException);
    expect(() => Report.fromJson('{"config":1,"pairs":[],"analysis":{}}'),
        throwsFormatException);
    expect(() => Report.fromBytes([0xff]), throwsFormatException);
    expect(
      () => Report.fromBytes(List<int>.filled(4 * 1024 * 1024 + 1, 0xff)),
      throwsFormatException,
      reason: 'byte bound is checked before UTF-8 decoding',
    );
    final tooDeep =
        '${List<String>.filled(129, '[').join()}0${List<String>.filled(129, ']').join()}';
    expect(() => Report.fromJson(tooDeep), throwsFormatException);
    expect(
        () => Report.fromJson(
            List<String>.filled(4 * 1024 * 1024 + 1, ' ').join()),
        throwsFormatException);
    expect(() => signed64(BigInt.parse('9223372036854775808'), 'field'),
        throwsFormatException);
    expect(
        () => Report(report.config, report.batchSize, report.pairs,
            report.analysis, {'bad': Object()}).toJson(),
        throwsArgumentError);
  });

  test('measured callback and zero-duration paths fail without reports', () {
    var calls = 0;
    BigInt mismatchesAfterWarmup(BigInt n) {
      calls++;
      return calls < 3 ? n : BigInt.zero;
    }

    var tick = BigInt.zero;
    BigInt clock() {
      final v = tick;
      tick += BigInt.one;
      return v;
    }

    expect(
        () => collect(
            config:
                Config(pairs: 1, targetNs: BigInt.one, maxBatch: BigInt.one),
            expectedChecksum: BigInt.one,
            baseline: mismatchesAfterWarmup,
            candidate: (n) => n,
            now: clock),
        throwsStateError);
    tick = BigInt.zero;
    expect(
        () => collect(
            config: Config(
                pairs: 1, targetNs: BigInt.from(3), maxBatch: BigInt.from(3)),
            expectedChecksum: BigInt.one,
            baseline: (n) => n,
            candidate: (n) => n,
            now: clock),
        throwsStateError);
    expect(
        () => indexedBatch(
            [BigInt.parse('9223372036854775807'), BigInt.one], BigInt.one),
        throwsRangeError);
    var clockRead = 0;
    BigInt zeroOnMeasure() {
      final read = clockRead++;
      return BigInt.from(read < 4 ? read : 4);
    }

    expect(
        () => collect(
            config:
                Config(pairs: 1, targetNs: BigInt.one, maxBatch: BigInt.one),
            expectedChecksum: BigInt.one,
            baseline: (n) => n,
            candidate: (n) => n,
            now: zeroOnMeasure),
        throwsStateError);
  });

  test('replays valid interval values and optional profile metadata', () {
    var tick = BigInt.zero;
    BigInt now() {
      final value = tick;
      tick += BigInt.one;
      return value;
    }

    BigInt batch(BigInt n) => n;
    final report = collect(
        config: Config(pairs: 6, targetNs: BigInt.one, maxBatch: BigInt.one),
        expectedChecksum: BigInt.one,
        baseline: batch,
        candidate: batch,
        now: now,
        metadata: const {
          'workload': 'test',
          'invocation': <String>[],
          'revision': 'r',
          'runtime': 'Dart',
          'os': 'test',
          'arch': 'test',
          'profile': 'debug'
        });
    expect(report.analysis.interval, [1.0, 1.0]);
    expect(Report.fromJson(report.toJson()).metadata['profile'], 'debug');
    final decoded = jsonDecode(report.toJson()) as Map<String, dynamic>;
    final core = Map<String, dynamic>.from(decoded)
      ..removeWhere((key, _) => [
            'workload',
            'invocation',
            'revision',
            'runtime',
            'os',
            'arch',
            'profile'
          ].contains(key));
    expect(Report.fromJson(jsonEncode(core)).metadata, isEmpty);
    expect(() => Report.fromJson(jsonEncode({...core, 'profile': 'debug'})),
        throwsFormatException);
    final emptyArg = Map<String, dynamic>.from(decoded)..['invocation'] = [''];
    expect(Report.fromJson(jsonEncode(emptyArg)).metadata['invocation'], ['']);
    final whitespaceRevision = Map<String, dynamic>.from(decoded)
      ..['revision'] = ' ';
    expect(Report.fromJson(jsonEncode(whitespaceRevision)).metadata['revision'],
        ' ');
    for (final key in [
      'workload',
      'revision',
      'runtime',
      'os',
      'arch',
      'profile'
    ]) {
      final nul = Map<String, dynamic>.from(decoded)..[key] = 'bad\u0000value';
      expect(() => Report.fromJson(jsonEncode(nul)), throwsFormatException);
    }
    final nulArg = Map<String, dynamic>.from(decoded)
      ..['invocation'] = ['bad\u0000value'];
    expect(() => Report.fromJson(jsonEncode(nulArg)), throwsFormatException);
    final partial = Map<String, dynamic>.from(core)..['workload'] = 'test';
    expect(() => Report.fromJson(jsonEncode(partial)), throwsFormatException);
    expect(
        () => Report.fromJson(
            report.toJson().replaceFirst('"profile":"debug"', '"profile":""')),
        throwsFormatException);
    expect(
        () => Report.fromJson(
            report.toJson().replaceFirst('"profile":"debug"', '"profile":1')),
        throwsFormatException);
  });
}
