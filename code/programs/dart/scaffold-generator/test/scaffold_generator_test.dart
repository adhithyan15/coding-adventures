import 'dart:io';

import 'package:scaffold_generator/src/scaffold_generator.dart';
import 'package:test/test.dart';

void writeFile(String repoRoot, String relativePath, String content) {
  final file = File('$repoRoot/$relativePath');
  file.parent.createSync(recursive: true);
  file.writeAsStringSync(content);
}

String readCapabilityFixture(String name) => File(
      '../../../specs/fixtures/scaffold-generator/$name',
    ).readAsStringSync();

ProcessResult runDart(List<String> arguments, String workingDirectory) {
  final result = Process.runSync(
    Platform.resolvedExecutable,
    arguments,
    workingDirectory: workingDirectory,
  );
  expect(
    result.exitCode,
    0,
    reason:
        'dart ${arguments.join(' ')} failed in $workingDirectory\n${result.stdout}\n${result.stderr}',
  );
  return result;
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

  group('capability manifests', () {
    test('renders the schema-v1 pure-library golden document', () {
      expect(
        capabilityManifestContents(PackageType.library, 'my-pkg'),
        readCapabilityFixture('dart_library_required_capabilities.json'),
      );
    });

    test('renders the schema-v1 stdout program golden document', () {
      expect(
        capabilityManifestContents(PackageType.program, 'build-helper'),
        readCapabilityFixture('dart_program_required_capabilities.json'),
      );
    });
  });

  group('repository root', () {
    late Directory tempDir;

    setUp(() {
      tempDir = Directory.systemTemp.createTempSync('dart-scaffold-root-');
      Directory('${tempDir.path}/code').createSync();
      File('${tempDir.path}/lessons.md').writeAsStringSync('# Lessons\n');
    });

    tearDown(() {
      tempDir.deleteSync(recursive: true);
    });

    test('accepts only the explicit repository root', () {
      expect(findRepoRoot(tempDir.path), tempDir.absolute.path);
      expect(
        () => findRepoRoot('${tempDir.path}/code'),
        throwsA(isA<ArgumentError>()),
      );
    });

    test('derives the default root from the generator package', () {
      final script = Uri.file(
        '${tempDir.path}/code/programs/dart/scaffold-generator/bin/scaffold_generator.dart',
      );
      expect(defaultRepoRoot(script), tempDir.path);
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

    test('rejects untrusted transitive names before path resolution', () {
      writeFile(
        repoRoot,
        'code/packages/dart/poisoned/pubspec.yaml',
        '''
name: coding_adventures_poisoned
dependencies:
  ../../../../outside:
    path: ../../../../outside
''',
      );

      expect(
        () => transitiveClosure(<String>['poisoned'], repoRoot),
        throwsA(
          isA<ArgumentError>().having(
            (error) => error.message,
            'message',
            contains('not valid kebab-case'),
          ),
        ),
      );
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
      expect(
        File(
          '${targetDir.path}/required_capabilities.json',
        ).readAsStringSync(),
        capabilityManifestContents(PackageType.library, 'nib-parser'),
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
      expect(
        File(
          '${targetDir.path}/required_capabilities.json',
        ).readAsStringSync(),
        capabilityManifestContents(PackageType.program, 'nib-demo'),
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
      expect(preview, contains('required_capabilities.json'));
      expect(preview, contains('Transitive Dart dependencies'));
    });

    test(
      'generated library and program pass the real Dart toolchain',
      () {
        final libraryPlan = scaffoldPlan(
          repoRoot: repoRoot,
          options: const CliOptions(
            packageName: 'generated-library',
            packageType: PackageType.library,
            languages: <String>['dart'],
            directDependencies: <String>[],
            layer: 1,
            description: 'Generated downstream library.',
            dryRun: false,
          ),
        );
        final programPlan = scaffoldPlan(
          repoRoot: repoRoot,
          options: const CliOptions(
            packageName: 'generated-program',
            packageType: PackageType.program,
            languages: <String>['dart'],
            directDependencies: <String>[],
            layer: null,
            description: 'Generated downstream program.',
            dryRun: false,
          ),
        );
        writePlan(libraryPlan);
        writePlan(programPlan);

        for (final target in <String>[
          libraryPlan.targetDir,
          programPlan.targetDir,
        ]) {
          runDart(<String>['pub', 'get', '--offline'], target);
          runDart(<String>['analyze'], target);
          runDart(<String>['test'], target);
        }

        final execution = runDart(
          <String>['run', 'bin/generated_program.dart'],
          programPlan.targetDir,
        );
        expect(
            execution.stdout, contains('TODO: implement generated-program.'));
      },
      timeout: const Timeout(Duration(minutes: 2)),
    );
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
      final exitCode = runWithOverrides(
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
      final exitCode = runWithOverrides(
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
