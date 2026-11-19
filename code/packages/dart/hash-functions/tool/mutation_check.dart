import 'dart:io';

const _publicApiTest = 'test/public_api_conformance_test.dart';

final _mutations = <_Mutation>[
  _Mutation(
    name: 'fnv32-prime',
    source: 'lib/src/algorithms.dart',
    before: "BigInt.parse('01000193', radix: 16)",
    after: "BigInt.parse('01000192', radix: 16)",
    testName: 'public hash APIs match all 526344 exhaustive outputs',
  ),
  _Mutation(
    name: 'parameterized-polynomial-dispatch',
    source: 'lib/src/algorithms.dart',
    before: 'hash = (hash * base + BigInt.from(byte)) % modulus;',
    after: 'hash = (hash * _defaultPolynomialBase + BigInt.from(byte)) '
        '% _defaultPolynomialModulus;',
    testName: 'public hash APIs match all 526344 exhaustive outputs',
  ),
  _Mutation(
    name: 'siphash-initialization',
    source: 'lib/src/algorithms.dart',
    before: "BigInt.parse('736f6d6570736575', radix: 16) ^ k0",
    after: "BigInt.parse('736f6d6570736574', radix: 16) ^ k0",
    testName: 'public hash APIs match all 526344 exhaustive outputs',
  ),
  _Mutation(
    name: 'unicode-surrogate-validation',
    source: 'lib/src/algorithms.dart',
    before: 'for (var index = 0; index < value.length; index++) {',
    after: 'for (var index = value.length; index < value.length; index++) {',
    testName: 'ArgumentError: string-unpaired-surrogate',
  ),
  _Mutation(
    name: 'analysis-callback-width-validation',
    source: 'lib/src/analysis.dart',
    before: 'if (value < BigInt.zero || value >= upperBound) {',
    after: 'if (false) {',
    testName: 'ArgumentError: avalanche-negative-callback',
  ),
  _Mutation(
    name: 'siphash-wrapper-key-snapshot',
    source: 'lib/src/hash_function.dart',
    before:
        'SipHash24(List<int> key) : _key = List<int>.unmodifiable(_copyKey(key));',
    after: 'SipHash24(List<int> key) : _key = key;',
    testName: 'SipHash wrappers snapshot caller key ownership',
  ),
  _Mutation(
    name: 'node-unsafe-number-bucket-modulo',
    source: 'lib/src/analysis.dart',
    before: 'final bucket = value % bucketCount;',
    after: 'final bucket = BigInt.from(value.toInt() % numBuckets);',
    testName:
        'distributionTest uses sparse exact buckets through the portable int limit',
    nodeOnly: true,
  ),
];

Future<void> main() async {
  final packageRoot = Directory.current.absolute;
  final sourcePaths = <String>{
    for (final mutation in _mutations) mutation.source,
  };
  final sourceBytes = <String, List<int>>{
    for (final path in sourcePaths)
      path: File('${packageRoot.path}/$path').readAsBytesSync(),
  };
  final scratch = await Directory.systemTemp.createTemp('dt17_hash_mutation_');
  try {
    await _copyPackage(packageRoot, scratch);
    final setup = await _run(
      'dart',
      const ['pub', 'get', '--offline'],
      scratch,
    );
    _requireSuccess('scratch package setup', setup);

    var killed = 0;
    for (final mutation in _mutations) {
      for (final entry in sourceBytes.entries) {
        File('${scratch.path}/${entry.key}').writeAsBytesSync(entry.value);
      }
      final mutantFile = File('${scratch.path}/${mutation.source}');
      final original = String.fromCharCodes(sourceBytes[mutation.source]!);
      final matches = _occurrences(original, mutation.before);
      if (matches != 1) {
        throw StateError(
          '${mutation.name}: mutation anchor matched $matches times, expected once',
        );
      }
      final mutant = original.replaceFirst(mutation.before, mutation.after);
      if (mutant == original) {
        throw StateError('${mutation.name}: mutation changed no source bytes');
      }
      mutantFile.writeAsStringSync(mutant, flush: true);

      if (mutation.nodeOnly) {
        final vmResult = await _runTest(mutation, 'vm', scratch);
        _requireSuccess('${mutation.name} VM control', vmResult);
      }
      final targetPlatform = mutation.nodeOnly ? 'node' : 'vm';
      final result = await _runTest(mutation, targetPlatform, scratch);
      _requireAssertionFailure(mutation, targetPlatform, result);
      stdout.writeln(
        'KILLED ${mutation.name} on $targetPlatform by '
        '${mutation.testName}; exit=${result.exitCode}; '
        'production-source-bytes=${mutant.length}',
      );
      killed++;
    }

    if (killed != _mutations.length) {
      throw StateError('Killed $killed/${_mutations.length} source mutants');
    }
    stdout.writeln('Mutation controls: $killed/${_mutations.length} killed.');
  } finally {
    await scratch.delete(recursive: true);
  }
}

Future<void> _copyPackage(Directory source, Directory destination) async {
  for (final name in ['pubspec.yaml', 'pubspec.lock']) {
    File('${source.path}/$name').copySync('${destination.path}/$name');
  }
  for (final directory in ['lib', 'test', 'tool']) {
    await _copyDirectory(
      Directory('${source.path}/$directory'),
      Directory('${destination.path}/$directory'),
    );
  }
  final generatedSnapshot =
      File('${source.path}/.dart_tool/hash_functions_fixture_snapshot.dart');
  final scratchTool = Directory('${destination.path}/.dart_tool')
    ..createSync(recursive: true);
  generatedSnapshot.copySync(
    '${scratchTool.path}/hash_functions_fixture_snapshot.dart',
  );
}

Future<void> _copyDirectory(Directory source, Directory destination) async {
  destination.createSync(recursive: true);
  for (final entity in source.listSync(followLinks: false)) {
    final segments = entity.uri.pathSegments.where((part) => part.isNotEmpty);
    final targetPath = '${destination.path}/${segments.last}';
    if (entity is Directory) {
      await _copyDirectory(entity, Directory(targetPath));
    } else if (entity is File) {
      entity.copySync(targetPath);
    } else {
      throw StateError('Unexpected package entry: ${entity.path}');
    }
  }
}

Future<ProcessResult> _runTest(
  _Mutation mutation,
  String platform,
  Directory scratch,
) {
  final escapedName = RegExp.escape(mutation.testName);
  return _run(
    'dart',
    [
      'test',
      '-p',
      platform,
      _publicApiTest,
      '--name',
      '^${escapedName}\$',
      '--reporter',
      'expanded',
    ],
    scratch,
  );
}

Future<ProcessResult> _run(
  String executable,
  List<String> arguments,
  Directory workingDirectory,
) =>
    Process.run(
      executable == 'dart' ? Platform.resolvedExecutable : executable,
      arguments,
      workingDirectory: workingDirectory.path,
    );

void _requireSuccess(String label, ProcessResult result) {
  if (result.exitCode != 0) {
    throw StateError(
      '$label failed during setup/runtime (exit ${result.exitCode}):\n'
      '${result.stdout}\n${result.stderr}',
    );
  }
}

void _requireAssertionFailure(
  _Mutation mutation,
  String platform,
  ProcessResult result,
) {
  final output = '${result.stdout}\n${result.stderr}';
  if (result.exitCode == 0) {
    throw StateError('${mutation.name} survived its ${platform} test');
  }
  if (!output.contains(mutation.testName) ||
      !(output.contains('Expected:') || output.contains('Expected<'))) {
    throw StateError(
      '${mutation.name} failed before the intended assertion; setup failures '
      'do not count as kills:\n$output',
    );
  }
}

int _occurrences(String source, String needle) {
  var count = 0;
  var offset = 0;
  while (true) {
    final found = source.indexOf(needle, offset);
    if (found < 0) return count;
    count++;
    offset = found + needle.length;
  }
}

class _Mutation {
  const _Mutation({
    required this.name,
    required this.source,
    required this.before,
    required this.after,
    required this.testName,
    this.nodeOnly = false,
  });

  final String name;
  final String source;
  final String before;
  final String after;
  final String testName;
  final bool nodeOnly;
}
