import 'dart:io';

import 'package:coding_adventures_microbenchmark/coding_adventures_microbenchmark.dart';

void main(List<String> arguments) {
  if (arguments.length != 1) throw ArgumentError('expected report path');
  final report = Report.fromJson(File(arguments.single).readAsStringSync());
  if (report.pairs.length != 40 ||
      report.pairs.any(
          (p) => p.baselineNs <= BigInt.zero || p.candidateNs <= BigInt.zero)) {
    throw StateError('native smoke report did not contain 40 positive pairs');
  }
  stdout.writeln(
      'replayed ${report.pairs.length} positive pairs: ${report.analysis.verdict}');
}
