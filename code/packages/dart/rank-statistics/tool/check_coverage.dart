import 'dart:io';

void main(List<String> arguments) {
  if (arguments.length != 1 || !arguments.single.startsWith('--lcov=')) {
    stderr.writeln('usage: check_coverage.dart --lcov=PATH');
    exitCode = 2;
    return;
  }
  final path = arguments.single.substring('--lcov='.length);
  final records = File(path).readAsStringSync().split('end_of_record');
  var found = false;
  var hit = 0;
  var total = 0;
  for (final record in records) {
    if (!record.contains('rank_statistics.dart')) continue;
    found = true;
    for (final line in record.split('\n')) {
      if (!line.startsWith('DA:')) continue;
      final count = int.parse(line.substring(3).split(',')[1]);
      total++;
      if (count > 0) hit++;
    }
  }
  final percent = total == 0 ? 0.0 : 100 * hit / total;
  stdout.writeln(
      'rank-statistics line coverage: ${percent.toStringAsFixed(2)}% ($hit/$total)');
  if (!found || total == 0 || percent < 95) exitCode = 1;
}
