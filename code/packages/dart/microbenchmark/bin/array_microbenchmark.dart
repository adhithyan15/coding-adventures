import 'dart:io';

import 'package:coding_adventures_microbenchmark/coding_adventures_microbenchmark.dart';

Future<void> main(List<String> args) async {
  final values = makeValues();
  final expected = indexedBatch(values, BigInt.one);
  final watch = Stopwatch()..start();
  BigInt now() =>
      BigInt.from(watch.elapsedTicks) *
      BigInt.from(1000000000) ~/
      BigInt.from(watch.frequency);
  final version = Platform.version.split(' on ').last.replaceAll('"', '');
  final metadata = <String, Object?>{
    'workload': 'indexed-array-sum-4096',
    'invocation': ['array-microbenchmark', ...args],
    'revision': (Platform.environment['GIT_REVISION']?.isNotEmpty ?? false)
        ? Platform.environment['GIT_REVISION']!
        : 'unknown',
    'runtime': 'Dart ${Platform.version.split(' ').take(3).join(' ')}',
    'os': Platform.operatingSystem,
    'arch': version,
  };
  try {
    final report = collect(
      config: Config(),
      expectedChecksum: expected,
      baseline: (n) => indexedBatch(values, n),
      candidate: (n) => iteratorBatch(values, n),
      now: now,
      metadata: metadata,
    );
    stdout.write('${report.toJson()}\n');
    await stdout.flush();
  } catch (error) {
    stderr.writeln('benchmark failed: $error');
    exitCode = 1;
  }
}
