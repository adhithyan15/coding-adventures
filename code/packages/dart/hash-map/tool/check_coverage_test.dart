import 'check_coverage.dart';

void main() {
  final lines = <String>[];
  for (final source in <String>[
    'lib/coding_adventures_hash_map.dart',
    'lib/src/errors.dart',
    'lib/src/hash_algorithm.dart',
    'lib/src/hash_map.dart',
    'lib/src/hash_map_key.dart',
  ]) {
    lines
      ..add('SF:$source')
      ..add('DA:1,1');
    for (var branch = 0; branch < 4; branch++) {
      lines.add('BRDA:1,0,$branch,1');
    }
    lines.add('end_of_record');
  }
  lines[3] = 'BRDA:1,0,2,-';

  final oneUncovered = summarizeLcov(lines.join('\n'));
  if (oneUncovered.branchesFound != 20 || oneUncovered.branchesHit != 19) {
    throw StateError(
      'Uncovered BRDA must remain in the denominator: '
      '${oneUncovered.branchesHit}/${oneUncovered.branchesFound}',
    );
  }
  if (oneUncovered.exceedsThreshold) {
    throw StateError('95% branch coverage must not satisfy a >95% gate.');
  }

  lines[3] = 'BRDA:1,0,2,1';
  final complete = summarizeLcov(lines.join('\n'));
  if (!complete.exceedsThreshold) {
    throw StateError('A complete 100% synthetic report should pass.');
  }
  print('Coverage checker fixture passed: BRDA `-` is an uncovered branch.');
}
