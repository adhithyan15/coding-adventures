import 'dart:io';

const _minimumCoveragePercent = 95.0;

void main(List<String> arguments) {
  final input = _inputPath(arguments);
  final report = File(input).readAsLinesSync();
  final totals = _CoverageTotals();
  final section = _CoverageSection();
  for (final line in report) {
    if (line.startsWith('SF:')) {
      totals.add(section);
      section.reset();
    } else {
      section.consume(line);
    }
  }
  totals.add(section);

  _printCoverage('Line', totals.linesHit, totals.linesFound);
  _printCoverage('Branch', totals.branchesHit, totals.branchesFound);
  if (totals.functionsFound == 0) {
    stdout.writeln('Function coverage: unavailable in this LCOV report');
  } else {
    _printCoverage('Function', totals.functionsHit, totals.functionsFound);
  }

  _requireThreshold('line', totals.linesHit, totals.linesFound);
  _requireThreshold('branch', totals.branchesHit, totals.branchesFound);
}

String _inputPath(List<String> arguments) {
  if (arguments.length != 1 || !arguments.single.startsWith('--lcov=')) {
    stderr.writeln('Usage: dart run tool/check_coverage.dart --lcov=<path>');
    exitCode = 64;
    return '';
  }
  return arguments.single.substring('--lcov='.length);
}

void _printCoverage(String label, int hit, int found) {
  final percent = found == 0 ? 0.0 : hit * 100 / found;
  stdout
      .writeln('$label coverage: ${percent.toStringAsFixed(2)}% ($hit/$found)');
}

void _requireThreshold(String label, int hit, int found) {
  if (found == 0) {
    throw StateError('No $label coverage records found for lib/');
  }
  final percent = hit * 100 / found;
  if (percent < _minimumCoveragePercent) {
    throw StateError(
      'Production library $label coverage ${percent.toStringAsFixed(2)}% '
      'is below $_minimumCoveragePercent%',
    );
  }
}

class _CoverageTotals {
  int linesFound = 0;
  int linesHit = 0;
  int branchesFound = 0;
  int branchesHit = 0;
  int functionsFound = 0;
  int functionsHit = 0;

  void add(_CoverageSection section) {
    linesFound += section.linesFound;
    linesHit += section.linesHit;
    if (section.branchSummaryFound > 0) {
      branchesFound += section.branchSummaryFound;
      branchesHit += section.branchSummaryHit;
    } else {
      branchesFound += section.branchRecords;
      branchesHit += section.branchRecordsHit;
    }
    functionsFound += section.functionsFound;
    functionsHit += section.functionsHit;
  }
}

class _CoverageSection {
  int linesFound = 0;
  int linesHit = 0;
  int branchSummaryFound = 0;
  int branchSummaryHit = 0;
  int branchRecords = 0;
  int branchRecordsHit = 0;
  int functionsFound = 0;
  int functionsHit = 0;

  void reset() {
    linesFound = 0;
    linesHit = 0;
    branchSummaryFound = 0;
    branchSummaryHit = 0;
    branchRecords = 0;
    branchRecordsHit = 0;
    functionsFound = 0;
    functionsHit = 0;
  }

  void consume(String line) {
    if (line.startsWith('LF:')) {
      linesFound += int.parse(line.substring(3));
    } else if (line.startsWith('LH:')) {
      linesHit += int.parse(line.substring(3));
    } else if (line.startsWith('BRF:')) {
      branchSummaryFound += int.parse(line.substring(4));
    } else if (line.startsWith('BRH:')) {
      branchSummaryHit += int.parse(line.substring(4));
    } else if (line.startsWith('BRDA:')) {
      final taken = line.split(',').last;
      branchRecords++;
      if (taken != '-' && int.parse(taken) > 0) branchRecordsHit++;
    } else if (line.startsWith('FNF:')) {
      functionsFound += int.parse(line.substring(4));
    } else if (line.startsWith('FNH:')) {
      functionsHit += int.parse(line.substring(4));
    }
  }
}
