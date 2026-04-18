import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_cli_builder/cli_builder.dart';

const List<String> validLanguages = <String>['dart'];
final RegExp kebabCasePattern = RegExp(r'^[a-z][a-z0-9]*(-[a-z0-9]+)*$');

enum PackageType { library, program }

enum DartTargetKind { package, program }

class CliOptions {
  const CliOptions({
    required this.packageName,
    required this.packageType,
    required this.languages,
    required this.directDependencies,
    required this.layer,
    required this.description,
    required this.dryRun,
  });

  final String packageName;
  final PackageType packageType;
  final List<String> languages;
  final List<String> directDependencies;
  final int? layer;
  final String description;
  final bool dryRun;
}

class DependencyRef {
  const DependencyRef({
    required this.name,
    required this.kind,
    required this.absolutePath,
  });

  final String name;
  final DartTargetKind kind;
  final String absolutePath;
}

class ScaffoldFile {
  const ScaffoldFile({required this.relativePath, required this.content});

  final String relativePath;
  final String content;
}

class ScaffoldPlan {
  const ScaffoldPlan({
    required this.targetDir,
    required this.files,
    required this.transitiveDependencies,
  });

  final String targetDir;
  final List<ScaffoldFile> files;
  final List<String> transitiveDependencies;
}

String toSnakeCase(String kebab) => kebab.replaceAll('-', '_');

String toTitleCase(String kebab) {
  return kebab
      .split('-')
      .where((segment) => segment.isNotEmpty)
      .map((segment) => '${segment[0].toUpperCase()}${segment.substring(1)}')
      .join(' ');
}

String todayIso([DateTime? now]) {
  final value = now ?? DateTime.now();
  final year = value.year.toString().padLeft(4, '0');
  final month = value.month.toString().padLeft(2, '0');
  final day = value.day.toString().padLeft(2, '0');
  return '$year-$month-$day';
}

String defaultDescription(String packageName, PackageType packageType) {
  if (packageType == PackageType.program) {
    return 'A Dart program scaffold for $packageName.';
  }
  return 'A Dart package scaffold for $packageName.';
}

String buildLayerContext(int? layer, PackageType packageType) {
  final subject = packageType == PackageType.program ? 'program' : 'package';
  if (layer == null) {
    return 'This $subject is part of the coding-adventures Dart lane.';
  }
  return 'This $subject sits at layer $layer of the coding-adventures stack.';
}

String wrapDescription(String description) {
  final lines = description
      .split('\n')
      .map((line) => line.trim())
      .where((line) => line.isNotEmpty)
      .toList();
  if (lines.isEmpty) {
    return 'description: >\n  TODO: add a package description.';
  }
  return 'description: >\n  ${lines.join('\n  ')}';
}

String dartStringLiteral(String value) => jsonEncode(value);

String findRepoRoot([String? startPath]) {
  var current = Directory(startPath ?? Directory.current.path).absolute;
  while (true) {
    final codeDir = Directory('${current.path}/code');
    final lessons = File('${current.path}/lessons.md');
    if (codeDir.existsSync() && lessons.existsSync()) {
      return current.path;
    }

    final parent = current.parent;
    if (parent.path == current.path) {
      throw ArgumentError(
        'Could not find the coding-adventures repo root from ${current.path}.',
      );
    }
    current = parent;
  }
}

String defaultSpecPath([Uri? script]) {
  final scriptFile = File.fromUri(script ?? Platform.script);
  return '${scriptFile.parent.parent.path}/scaffold-generator.json';
}

List<String> parseLanguages(String rawValue) {
  final raw = rawValue.trim();
  if (raw.isEmpty || raw == 'all') {
    return validLanguages;
  }

  final values = raw
      .split(',')
      .map((value) => value.trim())
      .where((value) => value.isNotEmpty)
      .toSet()
      .toList()
    ..sort();

  for (final value in values) {
    if (value != 'dart') {
      throw ArgumentError(
        'Unsupported language "$value". The Dart bootstrap implementation accepts only dart or all.',
      );
    }
  }
  return values;
}

List<String> parseDependencyList(String rawValue) {
  final raw = rawValue.trim();
  if (raw.isEmpty) {
    return const <String>[];
  }

  final values = raw
      .split(',')
      .map((value) => value.trim())
      .where((value) => value.isNotEmpty)
      .toSet()
      .toList()
    ..sort();

  for (final value in values) {
    if (!kebabCasePattern.hasMatch(value)) {
      throw ArgumentError('Dependency "$value" is not valid kebab-case.');
    }
  }
  return values;
}

CliOptions optionsFromParseResult(ParseResult result) {
  final packageName = (result.arguments['package-name'] ??
      result.arguments['PACKAGE_NAME']) as String;
  final typeFlag = (result.flags['type'] as String?) ?? 'library';
  final packageType =
      typeFlag == 'program' ? PackageType.program : PackageType.library;
  final descriptionFlag =
      ((result.flags['description'] as String?) ?? '').trim();
  return CliOptions(
    packageName: packageName,
    packageType: packageType,
    languages: parseLanguages((result.flags['language'] as String?) ?? 'dart'),
    directDependencies: parseDependencyList(
      (result.flags['depends-on'] as String?) ?? '',
    ),
    layer: result.flags['layer'] as int?,
    description: descriptionFlag.isEmpty
        ? defaultDescription(packageName, packageType)
        : descriptionFlag,
    dryRun: (result.flags['dry-run'] as bool?) ?? false,
  );
}

DependencyRef resolveDependency(String repoRoot, String dependencyName) {
  final packagePath = '$repoRoot/code/packages/dart/$dependencyName';
  if (Directory(packagePath).existsSync()) {
    return DependencyRef(
      name: dependencyName,
      kind: DartTargetKind.package,
      absolutePath: packagePath,
    );
  }

  final programPath = '$repoRoot/code/programs/dart/$dependencyName';
  if (Directory(programPath).existsSync()) {
    return DependencyRef(
      name: dependencyName,
      kind: DartTargetKind.program,
      absolutePath: programPath,
    );
  }

  throw ArgumentError(
    'Dependency "$dependencyName" does not exist under code/packages/dart/ or code/programs/dart/.',
  );
}

String normalizeDependencyKey(String rawKey) {
  final trimmed = rawKey.trim().toLowerCase();
  if (trimmed.startsWith('coding_adventures_')) {
    return trimmed.substring('coding_adventures_'.length).replaceAll('_', '-');
  }
  return trimmed.replaceAll('_', '-');
}

List<String> readDartDependencies(String packageDir) {
  final pubspecFile = File('$packageDir/pubspec.yaml');
  if (!pubspecFile.existsSync()) {
    return const <String>[];
  }

  final dependencies = <String>{};
  String? activeBlock;
  String? pendingDependency;
  for (final line in pubspecFile.readAsLinesSync()) {
    final trimmed = line.trim();
    if (trimmed.isEmpty || trimmed.startsWith('#')) {
      continue;
    }

    final indent = line.length - line.trimLeft().length;
    if (indent == 0 && trimmed.endsWith(':')) {
      final blockName = trimmed.substring(0, trimmed.length - 1);
      if (blockName == 'dependencies' || blockName == 'dev_dependencies') {
        activeBlock = blockName;
        pendingDependency = null;
      } else {
        activeBlock = null;
        pendingDependency = null;
      }
      continue;
    }

    if (activeBlock == null) {
      continue;
    }

    if (indent == 2 && trimmed.contains(':')) {
      final key = trimmed.split(':').first.trim();
      final value = trimmed.split(':').skip(1).join(':').trim();
      if (key == 'sdk') {
        pendingDependency = null;
        continue;
      }
      if (trimmed.endsWith(':') && value.isEmpty) {
        pendingDependency = key;
      } else {
        pendingDependency = null;
      }
      continue;
    }

    if (indent >= 4 &&
        pendingDependency != null &&
        trimmed.startsWith('path:')) {
      dependencies.add(normalizeDependencyKey(pendingDependency));
      pendingDependency = null;
      continue;
    }

    if (indent <= 2) {
      pendingDependency = null;
    }
  }

  return dependencies.toList()..sort();
}

List<String> transitiveClosure(
  List<String> directDependencies,
  String repoRoot,
) {
  final visited = <String>{};
  final queue = List<String>.from(directDependencies);

  while (queue.isNotEmpty) {
    final dependency = queue.removeAt(0);
    if (!visited.add(dependency)) {
      continue;
    }

    final resolved = resolveDependency(repoRoot, dependency);
    for (final nextDependency in readDartDependencies(resolved.absolutePath)) {
      if (!visited.contains(nextDependency)) {
        queue.add(nextDependency);
      }
    }
  }

  return visited.toList()..sort();
}

List<String> topologicalSort(List<String> allDependencies, String repoRoot) {
  final remaining = allDependencies.toSet();
  final graph = <String, Set<String>>{};
  final inDegree = <String, int>{};

  for (final dependency in allDependencies) {
    final resolved = resolveDependency(repoRoot, dependency);
    final localDependencies = readDartDependencies(
      resolved.absolutePath,
    ).where(remaining.contains).toSet();
    graph[dependency] = localDependencies;
    inDegree[dependency] = localDependencies.length;
  }

  final queue = inDegree.entries
      .where((entry) => entry.value == 0)
      .map((entry) => entry.key)
      .toList()
    ..sort();

  final ordered = <String>[];
  while (queue.isNotEmpty) {
    final current = queue.removeAt(0);
    ordered.add(current);
    for (final entry in graph.entries) {
      if (entry.value.remove(current)) {
        final nextDegree = (inDegree[entry.key] ?? 0) - 1;
        inDegree[entry.key] = nextDegree;
        if (nextDegree == 0) {
          queue.add(entry.key);
          queue.sort();
        }
      }
    }
  }

  if (ordered.length != allDependencies.length) {
    throw StateError(
      'Circular Dart dependency detected in ${allDependencies.join(', ')}.',
    );
  }

  return ordered;
}

String dependencyPathForTarget(
  PackageType packageType,
  DependencyRef dependency,
) {
  if (packageType == PackageType.library) {
    return dependency.kind == DartTargetKind.package
        ? '../${dependency.name}'
        : '../../programs/dart/${dependency.name}';
  }
  return dependency.kind == DartTargetKind.package
      ? '../../../packages/dart/${dependency.name}'
      : '../${dependency.name}';
}

String renderDependencyBlock(
  PackageType packageType,
  String repoRoot,
  List<String> directDependencies,
) {
  if (directDependencies.isEmpty) {
    return 'dependencies: {}';
  }

  final buffer = StringBuffer('dependencies:\n');
  for (final dependencyName in directDependencies) {
    final dependency = resolveDependency(repoRoot, dependencyName);
    final packageKey = dependency.kind == DartTargetKind.package
        ? 'coding_adventures_${toSnakeCase(dependency.name)}'
        : toSnakeCase(dependency.name);
    buffer.writeln('  $packageKey:');
    buffer.writeln(
      '    path: ${dependencyPathForTarget(packageType, dependency)}',
    );
  }
  return buffer.toString().trimRight();
}

List<ScaffoldFile> generateCommonFiles({
  required String packageName,
  required PackageType packageType,
  required String description,
  required int? layer,
  required List<String> directDependencies,
}) {
  final snake = toSnakeCase(packageName);
  final title = packageType == PackageType.library
      ? 'coding_adventures_$snake'
      : toTitleCase(packageName);
  final subject = packageType == PackageType.library ? 'package' : 'program';
  final usage = packageType == PackageType.library
      ? "Import `package:coding_adventures_$snake/$snake.dart` and replace the starter API with real functionality."
      : 'Run `dart run bin/$snake.dart` after filling in the starter logic.';

  final readme = StringBuffer()
    ..writeln('# $title')
    ..writeln()
    ..writeln(description)
    ..writeln()
    ..writeln('## What it does')
    ..writeln()
    ..writeln(
      'This scaffold creates a Dart $subject with the standard coding-adventures layout: metadata, tests, documentation, and source files from day one.',
    )
    ..writeln()
    ..writeln('## Usage')
    ..writeln()
    ..writeln(usage)
    ..writeln()
    ..writeln('## How it fits in the stack')
    ..writeln()
    ..writeln(buildLayerContext(layer, packageType));

  if (directDependencies.isNotEmpty) {
    readme
      ..writeln()
      ..writeln('## Direct dependencies')
      ..writeln()
      ..writeln(directDependencies.join(', '));
  }

  final changelog = StringBuffer()
    ..writeln('# Changelog')
    ..writeln()
    ..writeln('## [0.1.0] - ${todayIso()}')
    ..writeln()
    ..writeln('### Added')
    ..writeln()
    ..writeln('- Initial Dart $subject scaffold for $packageName.');

  return <ScaffoldFile>[
    ScaffoldFile(
      relativePath: '.gitignore',
      content: '.dart_tool/\npubspec.lock\n',
    ),
    ScaffoldFile(
      relativePath: 'README.md',
      content: '${readme.toString().trimRight()}\n',
    ),
    ScaffoldFile(
      relativePath: 'CHANGELOG.md',
      content: '${changelog.toString().trimRight()}\n',
    ),
  ];
}

List<ScaffoldFile> generateDartLibrary({
  required String packageName,
  required String description,
  required int? layer,
  required String repoRoot,
  required List<String> directDependencies,
}) {
  final snake = toSnakeCase(packageName);
  final layerContext = buildLayerContext(layer, PackageType.library);
  final pubspec = StringBuffer()
    ..writeln('name: coding_adventures_$snake')
    ..writeln(wrapDescription(description))
    ..writeln('version: 0.1.0')
    ..writeln('repository: https://github.com/adhithyan15/coding-adventures')
    ..writeln('publish_to: none')
    ..writeln()
    ..writeln('environment:')
    ..writeln('  sdk: ^3.0.0')
    ..writeln()
    ..writeln(
      renderDependencyBlock(PackageType.library, repoRoot, directDependencies),
    )
    ..writeln()
    ..writeln('dev_dependencies:')
    ..writeln('  test: ^1.25.0');

  final libraryFile = 'library;\n\nexport \'src/$snake.dart\';\n';

  final sourceFile = '''
/// Public starter API for $packageName.
///
/// This file exists so a new package starts with one tiny, testable surface
/// area instead of an empty directory. Teams can grow the implementation from
/// here without having to backfill the standard Dart layout first.
const String packageName = 'coding_adventures_$snake';
const String packageVersion = '0.1.0';

/// Describe the scaffold in one sentence for smoke tests and REPL sessions.
String describePackage() =>
    ${dartStringLiteral('$description $layerContext')};
''';

  final testFile = '''
import 'package:coding_adventures_$snake/$snake.dart';
import 'package:test/test.dart';

void main() {
  test('exposes starter metadata', () {
    expect(packageName, 'coding_adventures_$snake');
    expect(packageVersion, '0.1.0');
    expect(describePackage(), contains(${dartStringLiteral(description)}));
  });
}
''';

  return <ScaffoldFile>[
    ...generateCommonFiles(
      packageName: packageName,
      packageType: PackageType.library,
      description: description,
      layer: layer,
      directDependencies: directDependencies,
    ),
    ScaffoldFile(
      relativePath: 'pubspec.yaml',
      content: '${pubspec.toString().trimRight()}\n',
    ),
    ScaffoldFile(relativePath: 'BUILD', content: 'dart pub get\ndart test\n'),
    ScaffoldFile(relativePath: 'lib/$snake.dart', content: libraryFile),
    ScaffoldFile(relativePath: 'lib/src/$snake.dart', content: sourceFile),
    ScaffoldFile(relativePath: 'test/${snake}_test.dart', content: testFile),
  ];
}

List<ScaffoldFile> generateDartProgram({
  required String packageName,
  required String description,
  required int? layer,
  required String repoRoot,
  required List<String> directDependencies,
}) {
  final snake = toSnakeCase(packageName);
  final pubspec = StringBuffer()
    ..writeln('name: $snake')
    ..writeln(wrapDescription(description))
    ..writeln('version: 0.1.0')
    ..writeln('publish_to: none')
    ..writeln()
    ..writeln('environment:')
    ..writeln('  sdk: ^3.0.0')
    ..writeln()
    ..writeln(
      renderDependencyBlock(PackageType.program, repoRoot, directDependencies),
    )
    ..writeln()
    ..writeln('dev_dependencies:')
    ..writeln('  test: ^1.25.0');

  final libraryFile = 'library;\n\nexport \'src/$snake.dart\';\n';

  final sourceFile = '''
/// Core behavior for the $packageName program.
///
/// The side effect stays in `bin/`, while this file carries the logic we can
/// test from day one.
String renderMessage() => 'TODO: implement $packageName.';
''';

  final binFile = '''
import 'package:$snake/$snake.dart';

void main() {
  print(renderMessage());
}
''';

  final testFile = '''
import 'package:$snake/$snake.dart';
import 'package:test/test.dart';

void main() {
  test('renders a starter message', () {
    expect(renderMessage(), contains('$packageName'));
  });
}
''';

  return <ScaffoldFile>[
    ...generateCommonFiles(
      packageName: packageName,
      packageType: PackageType.program,
      description: description,
      layer: layer,
      directDependencies: directDependencies,
    ),
    ScaffoldFile(
      relativePath: 'pubspec.yaml',
      content: '${pubspec.toString().trimRight()}\n',
    ),
    ScaffoldFile(
      relativePath: 'BUILD',
      content: 'dart pub get\ndart test\ndart run bin/$snake.dart\n',
    ),
    ScaffoldFile(relativePath: 'lib/$snake.dart', content: libraryFile),
    ScaffoldFile(relativePath: 'lib/src/$snake.dart', content: sourceFile),
    ScaffoldFile(relativePath: 'bin/$snake.dart', content: binFile),
    ScaffoldFile(relativePath: 'test/${snake}_test.dart', content: testFile),
  ];
}

ScaffoldPlan scaffoldPlan({
  required String repoRoot,
  required CliOptions options,
}) {
  if (!kebabCasePattern.hasMatch(options.packageName)) {
    throw ArgumentError(
      'Package name "${options.packageName}" is not valid kebab-case.',
    );
  }

  final typeDir =
      options.packageType == PackageType.library ? 'packages' : 'programs';
  final targetDir = '$repoRoot/code/$typeDir/dart/${options.packageName}';
  if (Directory(targetDir).existsSync()) {
    throw ArgumentError('Target directory already exists: $targetDir');
  }

  for (final language in options.languages) {
    if (!validLanguages.contains(language)) {
      throw ArgumentError(
        'Unsupported language "$language". This implementation only scaffolds Dart.',
      );
    }
  }

  for (final dependency in options.directDependencies) {
    resolveDependency(repoRoot, dependency);
  }

  final closure = transitiveClosure(options.directDependencies, repoRoot);
  final orderedDependencies = topologicalSort(closure, repoRoot);
  final files = options.packageType == PackageType.library
      ? generateDartLibrary(
          packageName: options.packageName,
          description: options.description,
          layer: options.layer,
          repoRoot: repoRoot,
          directDependencies: options.directDependencies,
        )
      : generateDartProgram(
          packageName: options.packageName,
          description: options.description,
          layer: options.layer,
          repoRoot: repoRoot,
          directDependencies: options.directDependencies,
        );

  return ScaffoldPlan(
    targetDir: targetDir,
    files: files,
    transitiveDependencies: orderedDependencies,
  );
}

void writePlan(ScaffoldPlan plan) {
  final targetDir = Directory(plan.targetDir);
  targetDir.createSync(recursive: true);
  for (final file in plan.files) {
    final output = File('${plan.targetDir}/${file.relativePath}');
    output.parent.createSync(recursive: true);
    output.writeAsStringSync(file.content);
  }
}

String renderDryRun(ScaffoldPlan plan) {
  final buffer = StringBuffer()
    ..writeln('Would create ${plan.targetDir}')
    ..writeln();

  if (plan.transitiveDependencies.isEmpty) {
    buffer.writeln('Transitive Dart dependencies: (none)');
  } else {
    buffer.writeln(
      'Transitive Dart dependencies: ${plan.transitiveDependencies.join(', ')}',
    );
  }
  buffer.writeln();

  for (final file in plan.files) {
    buffer.writeln('=== ${file.relativePath} ===');
    buffer.writeln(file.content.trimRight());
    buffer.writeln();
  }
  return buffer.toString();
}

int run(
  List<String> args, {
  String? repoRoot,
  StringSink? out,
  StringSink? err,
  String? specPath,
}) {
  final stdoutSink = out ?? stdout;
  final stderrSink = err ?? stderr;

  try {
    final result = Parser.fromPath(specPath ?? defaultSpecPath(), <String>[
      'scaffold-generator',
      ...args,
    ]).parse();

    if (result is HelpResult) {
      stdoutSink.writeln(result.text);
      return 0;
    }
    if (result is VersionResult) {
      stdoutSink.writeln(result.version);
      return 0;
    }
    if (result is! ParseResult) {
      stderrSink.writeln('Unexpected CLI Builder result: $result');
      return 1;
    }

    final options = optionsFromParseResult(result);
    final root = repoRoot ?? findRepoRoot();
    final plan = scaffoldPlan(repoRoot: root, options: options);
    if (options.dryRun) {
      stdoutSink.write(renderDryRun(plan));
      return 0;
    }

    writePlan(plan);
    stdoutSink.writeln('Created ${plan.targetDir}');
    return 0;
  } on ParseErrors catch (error) {
    for (final parseError in error.errors) {
      stderrSink.writeln(parseError.message);
      if (parseError.suggestion != null) {
        stderrSink.writeln(parseError.suggestion);
      }
    }
    return 1;
  } on ArgumentError catch (error) {
    stderrSink.writeln(error.message);
    return 1;
  } on StateError catch (error) {
    stderrSink.writeln(error.message);
    return 1;
  }
}
