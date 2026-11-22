// Compile and execute concrete broken copies of the production source. Each
// mutant runs the same oracle/fixture tests. An analyzer/compile failure is an
// invalid control, and only a runtime assertion failure counts as detection.
import 'dart:io';

void main() {
  final original = Directory.current;
  final fixture = File(Platform.environment['FENWICK_FIXTURE'] ??
          '../../../specs/fixtures/fenwick-tree-v1/cases.json')
      .absolute
      .path;
  final source = File('lib/src/fenwick_tree.dart').readAsStringSync();
  final mutations = <String, List<String>>{
    'omit parent propagation': [
      'if (parent <= size) _tree[parent] += _tree[index];',
      'if (parent < 0) _tree[parent] += _tree[index];',
    ],
    'omit update propagation': [
      '_tree[cursor] += delta;',
      '_tree[cursor] += 0;',
    ],
    'prefix excludes endpoint': [
      'var cursor = index; cursor > 0;',
      'var cursor = index - 1; cursor > 0;',
    ],
    'range includes preceding point': [
      'prefixSum(right) - prefixSum(left - 1)',
      'prefixSum(right) - prefixSum(left)',
    ],
    'kth uses strict prefix crossing': [
      '_tree[next] < remaining',
      '_tree[next] <= remaining',
    ],
    'skip update L1 rejection': [
      'newValue.abs() > maxMagnitude - otherMagnitude',
      'false',
    ],
    'mutate points before rejected update': [
      'final otherMagnitude = _absoluteTotal - oldValue.abs();',
      '_points[index - 1] = newValue;\n    final otherMagnitude = _absoluteTotal - oldValue.abs();',
    ],
    'skip delta scalar rejection': [
      'delta < -maxMagnitude || delta > maxMagnitude',
      'false',
    ],
    'forget negative frequency restoration': [
      'if (oldValue < 0) _negativeCount--;',
      'if (oldValue < 0) _negativeCount += 0;',
    ],
  };
  var killed = 0;
  final temp = Directory.systemTemp.createTempSync('fenwick-mutations-');
  try {
    for (final entry in mutations.entries) {
      final oldText = entry.value[0];
      if (oldText.allMatches(source).length != 1) {
        throw StateError('Mutation anchor is ambiguous: ${entry.key}');
      }
      final copy = Directory('${temp.path}/mutant-$killed')..createSync();
      for (final name in ['pubspec.yaml', 'pubspec.lock']) {
        File(name).copySync('${copy.path}/$name');
      }
      for (final folder in ['lib', 'test']) {
        for (final file in Directory(folder).listSync(recursive: true)) {
          if (file is File) {
            final destination = File('${copy.path}/${file.path}');
            destination.parent.createSync(recursive: true);
            file.copySync(destination.path);
          }
        }
      }
      File('${copy.path}/lib/src/fenwick_tree.dart')
          .writeAsStringSync(source.replaceFirst(oldText, entry.value[1]));
      ProcessResult run(List<String> arguments) => Process.runSync(
            Platform.resolvedExecutable,
            arguments,
            workingDirectory: copy.path,
            environment: {'FENWICK_FIXTURE': fixture},
          );
      final restore = run(['pub', 'get', '--offline']);
      if (restore.exitCode != 0) {
        throw StateError(
            'Mutation dependency restore failed: ${restore.stderr}');
      }
      final analyze = run(['analyze', '--fatal-infos']);
      if (analyze.exitCode != 0) {
        // Dead-code warnings are expected when a rejection condition becomes
        // false. Compilation below still must succeed, then assertions fail.
        final output = '${analyze.stdout}${analyze.stderr}';
        if (output.contains('error -')) {
          throw StateError('Mutation has analyzer errors: $output');
        }
      }
      final result = run(['test', '--reporter', 'expanded']);
      final output = '${result.stdout}${result.stderr}';
      if (result.exitCode != 1 ||
          !output.contains('Some tests failed') ||
          output.contains('Failed to load')) {
        throw StateError(
            'Undetected or invalid mutation ${entry.key}: $output');
      }
      killed++;
      stdout.writeln('Detected mutation: ${entry.key}');
    }
    stdout
        .writeln('Detected $killed/${mutations.length} production mutations.');
  } finally {
    Directory.current = original;
    temp.deleteSync(recursive: true);
  }
}
