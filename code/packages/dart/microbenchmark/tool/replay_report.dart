import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_microbenchmark/coding_adventures_microbenchmark.dart';

const _maxReportBytes = 4 * 1024 * 1024;

Future<Uint8List> _readBounded(File file) async {
  final bytes = BytesBuilder(copy: false);
  var length = 0;
  await for (final chunk in file.openRead()) {
    length += chunk.length;
    if (length > _maxReportBytes) {
      throw FormatException('input exceeds 4 MiB');
    }
    bytes.add(chunk);
  }
  return bytes.takeBytes();
}

Future<void> main(List<String> arguments) async {
  if (arguments.length != 1) throw ArgumentError('expected report path');
  final report = Report.fromBytes(await _readBounded(File(arguments.single)));
  if (report.pairs.length != 40 ||
      report.pairs.any(
          (p) => p.baselineNs <= BigInt.zero || p.candidateNs <= BigInt.zero)) {
    throw StateError('native smoke report did not contain 40 positive pairs');
  }
  stdout.writeln(
      'replayed ${report.pairs.length} positive pairs: ${report.analysis.verdict}');
}
