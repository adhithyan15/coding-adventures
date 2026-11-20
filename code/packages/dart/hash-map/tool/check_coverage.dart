import 'dart:io';

const _knownLibraryFiles = <String>{
  'coding_adventures_hash_map.dart',
  'src/errors.dart',
  'src/hash_algorithm.dart',
  'src/hash_map.dart',
  'src/hash_map_key.dart',
};
const _requiredCoverageFiles = <String>{
  'src/errors.dart',
  'src/hash_algorithm.dart',
  'src/hash_map.dart',
  'src/hash_map_key.dart',
};

class CoverageSummary {
  const CoverageSummary({
    required this.linesHit,
    required this.linesFound,
    required this.branchesHit,
    required this.branchesFound,
  });

  final int linesHit;
  final int linesFound;
  final int branchesHit;
  final int branchesFound;

  double get linePercent => linesFound == 0 ? 0 : linesHit * 100 / linesFound;
  double get branchPercent =>
      branchesFound == 0 ? 0 : branchesHit * 100 / branchesFound;
  bool get exceedsThreshold =>
      linesFound > 0 &&
      branchesFound > 0 &&
      linePercent > 95 &&
      branchPercent > 95;
}

CoverageSummary summarizeLcov(String contents) {
  final seenFiles = <String>{};
  var currentFileIsProduction = false;
  var linesFound = 0;
  var linesHit = 0;
  var branchesFound = 0;
  var branchesHit = 0;

  for (final record in contents.split(RegExp(r'\r?\n'))) {
    if (record.startsWith('SF:')) {
      final source = record.substring(3).replaceAll('\\', '/');
      final relative = _productionLibraryPath(source);
      if (relative == null) {
        if (source.startsWith('lib/') || source.contains('/lib/')) {
          throw FormatException('Unexpected library source in LCOV: $source');
        }
        currentFileIsProduction = false;
      } else {
        if (!_knownLibraryFiles.contains(relative)) {
          throw FormatException(
              'Unexpected production library source: $source');
        }
        seenFiles.add(relative);
        currentFileIsProduction = true;
      }
    } else if (record == 'end_of_record') {
      currentFileIsProduction = false;
    } else if (currentFileIsProduction && record.startsWith('DA:')) {
      final fields = record.substring(3).split(',');
      if (fields.length < 2)
        throw FormatException('Malformed LCOV line record.');
      linesFound++;
      if (int.parse(fields[1]) > 0) linesHit++;
    } else if (currentFileIsProduction && record.startsWith('BRDA:')) {
      final fields = record.substring(5).split(',');
      if (fields.length != 4) {
        throw FormatException('Malformed LCOV branch record.');
      }
      branchesFound++;
      final taken = fields[3];
      if (taken != '-' && int.parse(taken) > 0) branchesHit++;
    }
  }

  final missing = _requiredCoverageFiles.difference(seenFiles);
  if (missing.isNotEmpty) {
    throw FormatException(
      'LCOV is missing production files: ${missing.toList()..sort()}',
    );
  }
  return CoverageSummary(
    linesHit: linesHit,
    linesFound: linesFound,
    branchesHit: branchesHit,
    branchesFound: branchesFound,
  );
}

String? _productionLibraryPath(String source) {
  if (source.startsWith('lib/')) return source.substring(4);
  const marker = '/code/packages/dart/hash-map/lib/';
  final packageRoot = source.indexOf(marker);
  if (packageRoot >= 0) return source.substring(packageRoot + marker.length);
  if (source.contains('/lib/')) {
    throw FormatException(
        'LCOV library path is outside dart/hash-map: $source');
  }
  return null;
}

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

  final summary = summarizeLcov(File(path).readAsStringSync());
  stdout.writeln(
    'production lib line coverage: ${summary.linesHit}/${summary.linesFound} '
    '(${summary.linePercent.toStringAsFixed(2)}%)',
  );
  stdout.writeln(
    'production lib branch coverage: '
    '${summary.branchesHit}/${summary.branchesFound} '
    '(${summary.branchPercent.toStringAsFixed(2)}%)',
  );
  if (!summary.exceedsThreshold) {
    stderr.writeln('Line and branch coverage must each exceed 95%.');
    exitCode = 1;
  }
}
