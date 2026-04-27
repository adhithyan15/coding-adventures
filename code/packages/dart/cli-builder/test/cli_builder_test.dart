import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_cli_builder/cli_builder.dart';
import 'package:test/test.dart';

void main() {
  Map<String, dynamic> baseSpec() {
    return <String, dynamic>{
      'cli_builder_spec_version': '1.0',
      'name': 'paint',
      'description': 'Paint things',
      'version': '1.2.3',
      'parsing_mode': 'gnu',
      'builtin_flags': <String, dynamic>{'help': true, 'version': true},
      'global_flags': <Map<String, dynamic>>[
        <String, dynamic>{
          'id': 'verbose',
          'short': 'v',
          'long': 'verbose',
          'description': 'Verbose output',
          'type': 'count',
        },
      ],
      'flags': <Map<String, dynamic>>[
        <String, dynamic>{
          'id': 'output',
          'short': 'o',
          'long': 'output',
          'description': 'Output path',
          'type': 'string',
        },
        <String, dynamic>{
          'id': 'color',
          'long': 'color',
          'description': 'Color mode',
          'type': 'enum',
          'enum_values': <String>['always', 'never'],
          'default_when_present': 'always',
        },
        <String, dynamic>{
          'id': 'profile',
          'long': 'profile',
          'description': 'Enable profiling',
          'type': 'boolean',
          'requires': <String>['config'],
        },
        <String, dynamic>{
          'id': 'config',
          'long': 'config',
          'description': 'Config file',
          'type': 'string',
        },
      ],
      'arguments': <Map<String, dynamic>>[
        <String, dynamic>{
          'id': 'input',
          'display_name': 'INPUT',
          'description': 'Input source',
          'type': 'string',
          'required': true,
        },
      ],
      'commands': <Map<String, dynamic>>[
        <String, dynamic>{
          'id': 'serve',
          'name': 'serve',
          'aliases': <String>['srv'],
          'description': 'Serve content',
          'inherit_global_flags': true,
          'flags': <Map<String, dynamic>>[
            <String, dynamic>{
              'id': 'port',
              'long': 'port',
              'description': 'Port',
              'type': 'integer',
            },
          ],
          'arguments': <Map<String, dynamic>>[],
          'commands': <Map<String, dynamic>>[],
          'mutually_exclusive_groups': <Map<String, dynamic>>[],
        },
      ],
      'mutually_exclusive_groups': <Map<String, dynamic>>[],
    };
  }

  group('validation', () {
    test('rejects duplicate flag ids', () {
      final spec = baseSpec();
      (spec['flags'] as List).add(
        <String, dynamic>{
          'id': 'output',
          'long': 'other-output',
          'description': 'Duplicate',
          'type': 'string',
        },
      );

      final validation = validateSpecObject(spec);
      expect(validation.isValid, isFalse);
      expect(validation.errors.join('\n'), contains('duplicate flag id'));
    });
  });

  group('token classifier', () {
    test('classifies stacked and single-dash-long tokens', () {
      final classifier = TokenClassifier(<FlagDef>[
        const FlagDef(
          id: 'verbose',
          shortName: 'v',
          longName: 'verbose',
          singleDashLong: null,
          description: 'Verbose',
          type: ValueType.count,
          required: false,
          defaultValue: 0,
          valueName: null,
          enumValues: <String>[],
          defaultWhenPresent: null,
          conflictsWith: <String>[],
          requires: <String>[],
          requiredUnless: <String>[],
          repeatable: false,
        ),
        const FlagDef(
          id: 'classpath',
          shortName: null,
          longName: null,
          singleDashLong: 'classpath',
          description: 'Classpath',
          type: ValueType.string,
          required: false,
          defaultValue: null,
          valueName: null,
          enumValues: <String>[],
          defaultWhenPresent: null,
          conflictsWith: <String>[],
          requires: <String>[],
          requiredUnless: <String>[],
          repeatable: false,
        ),
      ]);

      expect(classifier.classify('-vvv').kind, TokenKind.stackedFlags);
      expect(classifier.classify('-classpath').kind, TokenKind.singleDashLong);
      expect(classifier.classify('--verbose').kind, TokenKind.longFlag);
    });
  });

  group('parser', () {
    late Directory tempDir;
    late File specFile;

    setUp(() {
      tempDir = Directory.systemTemp.createTempSync('dart-cli-builder-');
      specFile = File('${tempDir.path}\\spec.json');
      specFile.writeAsStringSync(jsonEncode(baseSpec()));
    });

    tearDown(() {
      if (tempDir.existsSync()) {
        tempDir.deleteSync(recursive: true);
      }
    });

    test('parses root command flags and arguments', () {
      final result = Parser.fromPath(
        specFile.path,
        <String>['paint', '--output', 'out.png', '-vv', 'scene.cad'],
      ).parse() as ParseResult;

      expect(result.commandPath, ['paint']);
      expect(result.flags['output'], 'out.png');
      expect(result.flags['verbose'], 2);
      expect(result.arguments['input'], 'scene.cad');
      expect(result.explicitFlags, ['output', 'verbose', 'verbose']);
    });

    test('supports enum default_when_present and subcommands', () {
      final result = Parser.fromPath(
        specFile.path,
        <String>['paint', '--color', 'scene.cad'],
      ).parse() as ParseResult;
      expect(result.flags['color'], 'always');

      final subcommand = Parser.fromPath(
        specFile.path,
        <String>['paint', 'serve', '--port', '8080', '-v'],
      ).parse() as ParseResult;
      expect(subcommand.commandPath, ['paint', 'serve']);
      expect(subcommand.flags['port'], 8080);
      expect(subcommand.flags['verbose'], 1);
    });

    test('returns help and version results', () {
      final help = Parser.fromPath(
        specFile.path,
        <String>['paint', 'serve', '--help'],
      ).parse();
      expect(help, isA<HelpResult>());
      expect((help as HelpResult).text, contains('GLOBAL OPTIONS'));

      final version = Parser.fromPath(
        specFile.path,
        <String>['paint', '--version'],
      ).parse();
      expect(version, isA<VersionResult>());
      expect((version as VersionResult).version, '1.2.3');
    });

    test('reports missing required dependency flags', () {
      expect(
        () => Parser.fromPath(
          specFile.path,
          <String>['paint', '--profile', 'scene.cad'],
        ).parse(),
        throwsA(isA<ParseErrors>()),
      );
    });
  });
}
