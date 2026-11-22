import 'dart:convert';
import 'dart:io';

const _mutations = <_Mutation>[
  _Mutation(
    name: 'key-type-tag-identity',
    source: 'lib/src/hash_map_key.dart',
    before: 'if (other is! HashMapKey || _tag != other._tag) return false;',
    after: 'if (other is! HashMapKey) return false;',
    testFile: 'test/hash_map_shared_fixture_test.dart',
    testName:
        'byte and string key tags remain distinct under a forced collision',
  ),
  _Mutation(
    name: 'strict-utf16-validation',
    source: 'lib/src/hash_map_key.dart',
    before: 'for (var index = 0; index < units.length; index++) {',
    after: 'for (var index = units.length; index < units.length; index++) {',
    testFile: 'test/hash_map_shared_fixture_test.dart',
    testName:
        'Dart-only malformed arguments reach their real public operations',
  ),
  _Mutation(
    name: 'siphash-key-snapshot',
    source: 'lib/src/hash_algorithm.dart',
    before:
        ': key = List<int>.unmodifiable(List<int>.of(key)),\n        super._();',
    after: ': key = key,\n        super._();',
    testFile: 'test/hash_map_shared_fixture_test.dart',
    testName: 'key and map ownership cases use detached public snapshots',
  ),
  _Mutation(
    name: 'persistent-update-structural-copy',
    source: 'lib/src/hash_map.dart',
    before: 'buckets: table.buckets == null ? null : List.of(table.buckets!),',
    after: 'buckets: table.buckets,',
    testFile: 'test/hash_map_shared_fixture_test.dart',
    testName: 'key and map ownership cases use detached public snapshots',
  ),
  _Mutation(
    name: 'chaining-growth-threshold',
    source: 'lib/src/hash_map.dart',
    before: '? count == capacity',
    after: '? count == capacity - 1',
    testFile: 'test/hash_map_shared_fixture_test.dart',
    testName: 'public maps match every directed operation and physical order',
  ),
  _Mutation(
    name: 'probing-full-cycle-tombstone-fallback',
    source: 'lib/src/hash_map.dart',
    before:
        'if (slot.state == _SlotState.tombstone && firstTombstone == null) {',
    after: 'if (false) {',
    testFile: 'test/hash_map_shared_fixture_test.dart',
    testName: 'public maps match every directed operation and physical order',
  ),
  _Mutation(
    name: 'probing-delete-live-count',
    source: 'lib/src/hash_map.dart',
    before:
        'slots![index] = _Slot<V>.tombstone();\n    count--;\n    return true;',
    after:
        'slots![index] = _Slot<V>.tombstone();\n    count++;\n    return true;',
    testFile: 'test/hash_map_shared_fixture_test.dart',
    testName: 'public maps match every directed operation and physical order',
  ),
  _Mutation(
    name: 'byte-domain-validation',
    source: 'lib/src/hash_map_key.dart',
    before: 'if (value < 0 || value > 255) throw InvalidByte(index, value);',
    after: 'if (false) throw InvalidByte(index, value);',
    testFile: 'test/hash_map_shared_fixture_test.dart',
    testName:
        'Dart-only malformed arguments reach their real public operations',
  ),
  _Mutation(
    name: 'node-unsafe-hash-to-index-conversion',
    source: 'lib/src/hash_map.dart',
    before:
        'int _bucketIndex(HashMapKey key, int capacity, HashAlgorithm algorithm) =>\n    (algorithm._hashValidated(key.encodedBytes) % BigInt.from(capacity))\n        .toInt();',
    after:
        'int _bucketIndex(HashMapKey key, int capacity, HashAlgorithm algorithm) {\n  final hash = algorithm._hashValidated(key.encodedBytes);\n  if (const bool.fromEnvironment(\'dart.library.js_interop\')) {\n    return hash.toInt() % capacity;\n  }\n  return (hash % BigInt.from(capacity)).toInt();\n}',
    testFile: 'test/hash_map_shared_fixture_test.dart',
    testName:
        'public placement preserves exact wide hash reduction on non-power capacity',
    nodeOnly: true,
  ),
];

Future<void> main(List<String> arguments) async {
  final selected = arguments.isEmpty
      ? _mutations
      : _mutations
          .where((mutation) => arguments.contains(mutation.name))
          .toList();
  final unknown = arguments
      .where((name) => !_mutations.any((mutation) => mutation.name == name))
      .toList();
  if (unknown.isNotEmpty || selected.isEmpty) {
    stderr
        .writeln('Unknown or empty mutation selection: ${unknown.join(', ')}');
    exitCode = 64;
    return;
  }

  final sourcePackage = Directory.current.absolute;
  final siblingHashFunctions =
      Directory('${sourcePackage.parent.path}/hash-functions');
  if (!siblingHashFunctions.existsSync()) {
    throw StateError(
        'Expected path dependency at ${siblingHashFunctions.path}');
  }
  final originalBytes = <String, List<int>>{};
  for (final mutation in selected) {
    originalBytes.putIfAbsent(
      mutation.source,
      () => File('${sourcePackage.path}/${mutation.source}').readAsBytesSync(),
    );
  }

  final scratchRoot =
      await Directory.systemTemp.createTemp('dt18_hash_map_mutation_');
  final scratchPath = scratchRoot.absolute.path;
  final systemTemp = Directory.systemTemp.absolute.path;
  if (!scratchPath.startsWith('$systemTemp${Platform.pathSeparator}')) {
    throw StateError('Refusing cleanup outside system temp: $scratchPath');
  }
  var killed = 0;
  try {
    for (final mutation in selected) {
      final packageCopy = Directory('${scratchRoot.path}/hash-map');
      final hashFunctionsCopy = Directory('${scratchRoot.path}/hash-functions');
      await _copyPackage(sourcePackage, packageCopy);
      await _copyHashFunctions(siblingHashFunctions, hashFunctionsCopy);

      final setup = await _run(
        ['pub', 'get', '--offline'],
        packageCopy,
      );
      _requireSuccess('${mutation.name} scratch setup', setup);

      final baselinePlatforms =
          mutation.nodeOnly ? const ['vm', 'node'] : const ['vm'];
      for (final platform in baselinePlatforms) {
        final baseline = await _runTest(mutation, platform, packageCopy);
        _requireBaseline(mutation, platform, baseline);
      }

      final source = String.fromCharCodes(originalBytes[mutation.source]!);
      final matches = _occurrences(source, mutation.before);
      if (matches != 1) {
        throw StateError(
          '${mutation.name}: anchor matched $matches times; expected exactly once',
        );
      }
      final mutated = source.replaceFirst(mutation.before, mutation.after);
      if (mutated == source) {
        throw StateError(
            '${mutation.name}: replacement changed no source bytes');
      }
      File('${packageCopy.path}/${mutation.source}')
          .writeAsStringSync(mutated, flush: true);

      if (mutation.nodeOnly) {
        final vmControl = await _runTest(mutation, 'vm', packageCopy);
        _requireSuccess('${mutation.name} VM precision control', vmControl);
      }
      final platform = mutation.nodeOnly ? 'node' : 'vm';
      final result = await _runTest(mutation, platform, packageCopy);
      _requireAssertionFailure(mutation, platform, result);
      stdout.writeln(
        'KILLED ${mutation.name} on $platform by "${mutation.testName}"; '
        'exit=${result.exitCode}; mutated-file=${mutation.source}; '
        'scratch-source-bytes=${mutated.length}.',
      );
      killed++;

      await packageCopy.delete(recursive: true);
      await hashFunctionsCopy.delete(recursive: true);
    }
    expectCount(killed, selected.length);
    stdout.writeln(
        'DT18 source mutation controls: $killed/${selected.length} killed.');
  } finally {
    await scratchRoot.delete(recursive: true);
  }
}

Future<void> _copyPackage(Directory source, Directory destination) async {
  destination.createSync(recursive: true);
  for (final name in ['pubspec.yaml', 'pubspec.lock']) {
    final file = File('${source.path}/$name');
    if (file.existsSync()) file.copySync('${destination.path}/$name');
  }
  for (final directory in ['lib', 'test', 'tool']) {
    await _copyDirectory(
      Directory('${source.path}/$directory'),
      Directory('${destination.path}/$directory'),
    );
  }
  final snapshot =
      File('${source.path}/.dart_tool/hash_map_fixture_snapshot.dart');
  if (!snapshot.existsSync()) {
    throw StateError('Fixture snapshot is missing: ${snapshot.path}');
  }
  final scratchTools = Directory('${destination.path}/.dart_tool')
    ..createSync(recursive: true);
  snapshot.copySync('${scratchTools.path}/${snapshot.uri.pathSegments.last}');
}

Future<void> _copyHashFunctions(Directory source, Directory destination) async {
  destination.createSync(recursive: true);
  for (final name in ['pubspec.yaml', 'pubspec.lock']) {
    final file = File('${source.path}/$name');
    if (file.existsSync()) file.copySync('${destination.path}/$name');
  }
  await _copyDirectory(
    Directory('${source.path}/lib'),
    Directory('${destination.path}/lib'),
  );
}

Future<void> _copyDirectory(Directory source, Directory destination) async {
  destination.createSync(recursive: true);
  for (final entity in source.listSync(followLinks: false)) {
    final name =
        entity.uri.pathSegments.where((segment) => segment.isNotEmpty).last;
    final target = '${destination.path}/$name';
    if (entity is Directory) {
      await _copyDirectory(entity, Directory(target));
    } else if (entity is File) {
      entity.copySync(target);
    } else {
      throw StateError('Unexpected package entry: ${entity.path}');
    }
  }
}

Future<ProcessResult> _run(
        List<String> arguments, Directory workingDirectory) =>
    Process.run(
      Platform.resolvedExecutable,
      arguments,
      workingDirectory: workingDirectory.path,
    );

Future<ProcessResult> _runTest(
  _Mutation mutation,
  String platform,
  Directory workingDirectory,
) =>
    _run(
      [
        'test',
        '-p',
        platform,
        mutation.testFile,
        '--name',
        '^${RegExp.escape(mutation.testName)}\$',
        '--reporter',
        'json',
      ],
      workingDirectory,
    );

void _requireSuccess(String label, ProcessResult result) {
  if (result.exitCode != 0) {
    throw StateError(
      '$label failed before mutation assertions (exit ${result.exitCode}):\n'
      '${result.stdout}\n${result.stderr}',
    );
  }
}

void _requireBaseline(
  _Mutation mutation,
  String platform,
  ProcessResult result,
) {
  final events = _jsonEvents(result.stdout, mutation.name, 'baseline');
  final starts = events.where(
    (event) =>
        event['type'] == 'testStart' &&
        (event['test'] as Map<String, dynamic>)['name'] == mutation.testName,
  );
  if (result.exitCode != 0 || starts.length != 1) {
    throw StateError(
      '${mutation.name} $platform baseline did not select exactly one named '
      'test (exit ${result.exitCode}):\n${result.stdout}\n${result.stderr}',
    );
  }
  final testId = (starts.single['test'] as Map<String, dynamic>)['id'];
  final completions = events.where(
    (event) => event['type'] == 'testDone' && event['testID'] == testId,
  );
  if (completions.length != 1 ||
      completions.single['result'] != 'success' ||
      completions.single['skipped'] == true) {
    throw StateError(
      '${mutation.name} $platform unmutated baseline failed/skipped its '
      'named test:\n${result.stdout}\n${result.stderr}',
    );
  }
}

void _requireAssertionFailure(
  _Mutation mutation,
  String platform,
  ProcessResult result,
) {
  final events = _jsonEvents(result.stdout, mutation.name, 'mutated');
  final starts = events.where(
    (event) =>
        event['type'] == 'testStart' &&
        (event['test'] as Map<String, dynamic>)['name'] == mutation.testName,
  );
  if (starts.length != 1) {
    throw StateError(
      '${mutation.name} $platform mutant did not select exactly one named '
      'test:\n${result.stdout}\n${result.stderr}',
    );
  }
  final testId = (starts.single['test'] as Map<String, dynamic>)['id'];
  final completions = events.where(
    (event) => event['type'] == 'testDone' && event['testID'] == testId,
  );
  if (result.exitCode == 0) {
    throw StateError(
      '${mutation.name} survived its $platform assertion:\n'
      '${result.stdout}\n${result.stderr}',
    );
  }
  final failures = events.where(
    (event) =>
        event['type'] == 'error' &&
        event['testID'] == testId &&
        event['isFailure'] == true,
  );
  final assertionText =
      failures.map((event) => event['error']?.toString() ?? '').join('\n');
  if (completions.length != 1 ||
      completions.single['result'] == 'success' ||
      failures.isEmpty ||
      !assertionText.contains('Expected')) {
    throw StateError(
      '${mutation.name} failed before its named behavioral assertion; '
      'setup, compilation, and uncaught errors do not count as kills:\n'
      '${result.stdout}\n${result.stderr}',
    );
  }
}

List<Map<String, dynamic>> _jsonEvents(
  String stdout,
  String mutationName,
  String phase,
) {
  final events = <Map<String, dynamic>>[];
  for (final line in const LineSplitter().convert(stdout)) {
    try {
      final event = jsonDecode(line);
      if (event is Map<String, dynamic>) events.add(event);
    } on FormatException catch (error) {
      throw StateError(
        '$mutationName $phase emitted invalid JSON reporter output: $error\n$line',
      );
    }
  }
  if (events.isEmpty || !events.any((event) => event['type'] == 'done')) {
    throw StateError(
        '$mutationName $phase did not emit a terminal JSON event.');
  }
  return events;
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

void expectCount(int actual, int expected) {
  if (actual != expected) {
    throw StateError('Killed $actual/${expected} selected source mutants.');
  }
}

class _Mutation {
  const _Mutation({
    required this.name,
    required this.source,
    required this.before,
    required this.after,
    required this.testFile,
    required this.testName,
    this.nodeOnly = false,
  });

  final String name;
  final String source;
  final String before;
  final String after;
  final String testFile;
  final String testName;
  final bool nodeOnly;
}
