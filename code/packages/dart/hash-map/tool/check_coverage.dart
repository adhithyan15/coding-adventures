import 'dart:io';

void main(List<String> arguments) {
  final path = arguments
      .where((argument) => argument.startsWith('--lcov='))
      .map((argument) => argument.substring('--lcov='.length))
      .firstOrNull;
  if (path == null) {
    stderr.writeln('Usage: dart run tool/check_coverage.dart --lcov=<path>');
    exitCode = 64;
    return;
  }

  final records = File(path).readAsLinesSync();
  var inLibrary = false;
  var linesFound = 0;
  var linesHit = 0;
  var branchesFound = 0;
  var branchesHit = 0;
  for (final record in records) {
    if (record.startsWith('SF:')) {
      inLibrary = record.substring(3).replaceAll('\\', '/').contains('/lib/');
    } else if (record == 'end_of_record') {
      inLibrary = false;
    } else if (inLibrary && record.startsWith('DA:')) {
      final fields = record.substring(3).split(',');
      linesFound++;
      if (int.parse(fields[1]) > 0) linesHit++;
    } else if (inLibrary && record.startsWith('BRDA:')) {
      final taken = record.split(',').last;
      if (taken == '-') continue;
      branchesFound++;
      if (int.parse(taken) > 0) branchesHit++;
    }
  }

  final linePercent = linesFound == 0 ? 0.0 : linesHit * 100 / linesFound;
  final branchPercent =
      branchesFound == 0 ? 0.0 : branchesHit * 100 / branchesFound;
  stdout.writeln(
    'production lib line coverage: $linesHit/$linesFound '
    '(${linePercent.toStringAsFixed(2)}%)',
  );
  stdout.writeln(
    'production lib branch coverage: $branchesHit/$branchesFound '
    '(${branchPercent.toStringAsFixed(2)}%)',
  );
  if (linesFound == 0 || branchesFound == 0) {
    stderr.writeln('Coverage report must contain line and branch records.');
    exitCode = 1;
  } else if (linePercent <= 95 || branchPercent <= 95) {
    stderr.writeln('Line and branch coverage must each exceed 95%.');
    exitCode = 1;
  }
}
