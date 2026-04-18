import 'dart:io';

import 'package:scaffold_generator/scaffold_generator.dart';
import 'package:test/test.dart';

void writeFile(String repoRoot, String relativePath, String content) {
  final file = File('$repoRoot/$relativePath');
  file.parent.createSync(recursive: true);
  file.writeAsStringSync(content);
}

void writeDartPackage(
  String repoRoot,
  String packageName, {
  List<String> dependencies = const <String>[],
}) {
  final snake = toSnakeCase(packageName);
  final dependencyBlock = dependencies.isEmpty
      ? 'dependencies: {}\n'
      : [
          'dependencies:',
          ...dependencies.expand(
            (dependency) => <String>[
              '  coding_adventures_${toSnakeCase(dependency)}:',
              '    path: ../$dependency',
            ],
          ),
        ].join('\n');

  writeFile(
    repoRoot,
    'code/packages/dart/$packageName/pubspec.yaml',
    [
      'name: coding_adventures_$snake',
      wrapDescription('Fixture package for $packageName.'),
      'version: 0.1.0',
      'publish_to: none',
      '',
      'environment:',
      '  sdk: ^3.0.0',
      '',
      dependencyBlock.trimRight(),
      '',
      'dev_dependencies:',
      '  test: ^1.25.0',
      '',
    ].join('\n'),
  );
}

void main() {
  group('name helpers', () {
    test('snake and title case conversions match repo conventions', () {
      expect(toSnakeCase('nib-parser'), 'nib_parser');
      expect(toTitleCase('nib-parser'), 'Nib Parser');
    });

    test('formats ISO dates', () {
      expect(todayIso(DateTime(2026, 4, 18)), '2026-04-18');
    });

    test('escapes strings for generated Dart code', () {
      expect(
        dartStringLiteral("it's \"fine\"\nnext"),
        '"it\'s \\"fine\\"\\nnext"',
      );
    });
  });

  group('Dart dependency parsing', () {
    late Directory tempDir;
    late String repoRoot;

    setUp(() {
      tempDir = Directory.systemTemp.createTempSync('dart-scaffold-generator-');
      repoRoot = tempDir.path;
      writeFile(repoRoot, 'lessons.md', '# Lessons\n');
      writeDartPackage(repoRoot, 'graph');
      writeDartPackage(repoRoot, 'lexer', dependencies: <String>['graph']);
      writeDartPackage(repoRoot, 'parser', dependencies: <String>['lexer']);
    });

    tearDown(() {
      tempDir.deleteSync(recursive: true);
    });

    test('reads dependency keys from pubspec blocks', () {
      expect(
        readDartDependencies('$repoRoot/code/packages/dart/parser'),
        <String>['lexer'],
      );
    });

    test('computes transitive closure and topological order', () {
      final closure = transitiveClosure(<String>['parser'], repoRoot);
      expect(closure, <String>['graph', 'lexer', 'parser']);
      expect(topologicalSort(closure, repoRoot), <String>[
        'graph',
        'lexer',
        'parser',
      ]);
    });
  });

  group('scaffolding', () {
    late Directory tempDir;
    late String repoRoot;

    setUp(() {
      tempDir = Directory.systemTemp.createTempSync('dart-scaffold-generator-');
      repoRoot = tempDir.path;
      writeFile(repoRoot, 'lessons.md', '# Lessons\n');
      writeDartPackage(repoRoot, 'grammar-tools');
      writeDartPackage(
        repoRoot,
        'lexer',
        dependencies: <String>['grammar-tools'],
      );
    });

    tearDown(() {
      tempDir.deleteSync(recursive: true);
    });

    test('creates a Dart library scaffold', () {
      final plan = scaffoldPlan(
        repoRoot: repoRoot,
        options: const CliOptions(
          packageName: 'nib-parser',
          packageType: PackageType.library,
          languages: <String>['dart'],
          directDependencies: <String>['lexer'],
          layer: 3,
          description: 'Nib parser for Dart.',
          dryRun: false,
        ),
      );

      writePlan(plan);

      final targetDir = Directory('$repoRoot/code/packages/dart/nib-parser');
      expect(targetDir.existsSync(), isTrue);
      expect(
        File('${targetDir.path}/pubspec.yaml').readAsStringSync(),
        contains('name: coding_adventures_nib_parser'),
      );
      expect(
        File('${targetDir.path}/pubspec.yaml').readAsStringSync(),
        contains('path: ../lexer'),
      );
      expect(
        File('${targetDir.path}/test/nib_parser_test.dart').readAsStringSync(),
        contains('describePackage()'),
      );
    });

    test('escapes quotes inside generated descriptions', () {
      final plan = scaffoldPlan(
        repoRoot: repoRoot,
        options: const CliOptions(
          packageName: 'quoted-package',
          packageType: PackageType.library,
          languages: <String>['dart'],
          directDependencies: <String>['lexer'],
          layer: 3,
          description: 'Parser for "quoted" input and it\'s safe.',
          dryRun: false,
        ),
      );

      writePlan(plan);

      final source = File(
        '$repoRoot/code/packages/dart/quoted-package/lib/src/quoted_package.dart',
      ).readAsStringSync();
      expect(
        source,
        contains("Parser for \\\"quoted\\\" input and it's safe."),
      );
    });

    test('creates a Dart program scaffold', () {
      final plan = scaffoldPlan(
        repoRoot: repoRoot,
        options: const CliOptions(
          packageName: 'nib-demo',
          packageType: PackageType.program,
          languages: <String>['dart'],
          directDependencies: <String>['lexer'],
          layer: null,
          description: 'Nib demo program for Dart.',
          dryRun: false,
        ),
      );

      writePlan(plan);

      final targetDir = Directory('$repoRoot/code/programs/dart/nib-demo');
      expect(targetDir.existsSync(), isTrue);
      expect(
        File('${targetDir.path}/bin/nib_demo.dart').readAsStringSync(),
        contains("print(renderMessage())"),
      );
      expect(
        File('${targetDir.path}/BUILD').readAsStringSync(),
        contains('dart run bin/nib_demo.dart'),
      );
      expect(
        File('${targetDir.path}/pubspec.yaml').readAsStringSync(),
        contains('path: ../../../packages/dart/lexer'),
      );
    });

    test('renders dry-run output for a scaffold plan', () {
      final plan = scaffoldPlan(
        repoRoot: repoRoot,
        options: const CliOptions(
          packageName: 'nib-lexer',
          packageType: PackageType.library,
          languages: <String>['dart'],
          directDependencies: <String>['lexer'],
          layer: 2,
          description: 'Nib lexer for Dart.',
          dryRun: true,
        ),
      );

      final preview = renderDryRun(plan);
      expect(preview, contains('Would create'));
      expect(preview, contains('pubspec.yaml'));
      expect(preview, contains('Transitive Dart dependencies'));
    });
  });

  group('CLI entrypoints', () {
    late Directory tempDir;
    late String repoRoot;
    late String specPath;

    setUp(() {
      tempDir = Directory.systemTemp.createTempSync('dart-scaffold-generator-');
      repoRoot = tempDir.path;
      writeFile(repoRoot, 'lessons.md', '# Lessons\n');
      writeDartPackage(repoRoot, 'lexer');
      specPath =
          Directory.current.uri.resolve('scaffold-generator.json').toFilePath();
    });

    tearDown(() {
      tempDir.deleteSync(recursive: true);
    });

    test('dry-run uses the CLI spec and leaves the tree untouched', () {
      final stdoutBuffer = StringBuffer();
      final stderrBuffer = StringBuffer();
      final exitCode = run(
        <String>[
          'nib-lexer',
          '--depends-on',
          'lexer',
          '--description',
          'Nib lexer for Dart.',
          '--dry-run',
        ],
        repoRoot: repoRoot,
        out: stdoutBuffer,
        err: stderrBuffer,
        specPath: specPath,
      );

      expect(exitCode, 0);
      expect(stdoutBuffer.toString(), contains('Would create'));
      expect(stdoutBuffer.toString(), contains('pubspec.yaml'));
      expect(
        Directory('$repoRoot/code/packages/dart/nib-lexer').existsSync(),
        isFalse,
      );
      expect(stderrBuffer.toString(), isEmpty);
    });

    test('reports invalid kebab-case names', () {
      final stderrBuffer = StringBuffer();
      final exitCode = run(
        <String>['NibLexer'],
        repoRoot: repoRoot,
        err: stderrBuffer,
        specPath: specPath,
      );

      expect(exitCode, 1);
      expect(stderrBuffer.toString(), contains('kebab-case'));
    });
  });
}
