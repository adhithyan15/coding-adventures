import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_build_tool/build_tool.dart';
import 'package:test/test.dart';

const expectedCases = <String>{
  'diff-selection/forced-package',
  'diff-selection/exact-build-fronts',
  'diff-selection/known-unmatched-near-build',
  'diff-selection/match-work-at-limit',
  'diff-selection/match-work-over-limit',
  'diff-selection/package-prefix',
  'diff-selection/repository-boundary-reverse-index',
  'diff-selection/strict-glob-character-classes',
  'diff-selection/transitive-package-change',
  'diff-selection/unknown-path-all',
  'diff-selection/unknown-path-error',
  'graph/canonical-edge-order',
  'graph/chain',
  'graph/cycle',
  'graph/diamond',
  'graph/empty',
  'graph/isolated',
  'graph/multiple-components',
  'graph/partial-cycle-no-output',
};

void main() {
  test('consumes every shared graph and diff fixture', () {
    final fixtureRoot = _repositoryRoot()
        .uri
        .resolve('code/specs/fixtures/build-tool-v1/')
        .toFilePath();
    final seen = <String>{};
    final files = Directory('$fixtureRoot${Platform.pathSeparator}cases')
        .listSync()
        .whereType<File>()
        .where((file) => file.path.endsWith('.json'))
        .toList()
      ..sort((left, right) => left.path.compareTo(right.path));

    for (final file in files) {
      final fixture =
          jsonDecode(file.readAsStringSync()) as Map<String, Object?>;
      final domain = fixture['domain'];
      if (domain != 'graph' && domain != 'diff_selection') continue;
      seen.add(fixture['id']! as String);
      if (domain == 'graph') {
        _assertGraphFixture(fixture);
      } else {
        _assertDiffFixture(fixture, fixtureRoot);
      }
    }
    expect(seen, equals(expectedCases));
  });

  test('rejects malformed inputs before evaluation', () {
    expect(
      () => BuildToolCore.evaluateGraph(
        const GraphInput(['fixture/a'], [Edge('fixture/a', 'fixture/missing')]),
      ),
      throwsArgumentError,
    );
    expect(
      () => BuildToolCore.evaluateGraph(
          const GraphInput(['fixture/a', 'fixture/a'], [])),
      throwsArgumentError,
    );
    expect(
      () => BuildToolCore.evaluateGraph(
        const GraphInput(['fixture/a'], [Edge('fixture/a', 'fixture/a')]),
      ),
      throwsArgumentError,
    );
    expect(
      () => BuildToolCore.evaluateGraph(
        const GraphInput(
          ['fixture/a', 'fixture/b'],
          [Edge('fixture/a', 'fixture/b'), Edge('fixture/a', 'fixture/b')],
        ),
      ),
      throwsArgumentError,
    );

    final boundary = RepositoryBoundary(1, 'registry', const []);
    final mismatched = DiffSelectionInput(
      packages: const [
        PackageSpec('fixture/a', 'a', SourceMode.packagePrefix, []),
      ],
      edges: const [],
      forcedPackages: const [],
      unknownPathPolicy: UnknownPathPolicy.error,
      changedPaths: const ['a/file'],
      boundarySha256: List.filled(64, '0').join(),
      boundary: boundary,
    );
    expect(
      () => BuildToolCore.evaluateDiffSelection(mismatched),
      throwsA(isA<ArgumentError>().having((error) => error.message, 'message',
          'DIFF_BOUNDARY_DIGEST_MISMATCH')),
    );

    final oversized = DiffSelectionInput(
      packages: const [
        PackageSpec('fixture/a', 'a', SourceMode.strictGlobs, ['*']),
      ],
      edges: const [],
      forcedPackages: const [],
      unknownPathPolicy: UnknownPathPolicy.error,
      changedPaths: ['a/${List.filled(100000, 'x').join()}'],
    );
    expect(
      () => BuildToolCore.evaluateDiffSelection(oversized),
      throwsA(isA<ArgumentError>()
          .having((error) => error.message, 'message', 'DIFF_PATH_INVALID')),
    );

    final cycle = DiffSelectionInput(
      packages: const [
        PackageSpec('fixture/a', 'a', SourceMode.packagePrefix, []),
        PackageSpec('fixture/b', 'b', SourceMode.packagePrefix, []),
      ],
      edges: const [
        Edge('fixture/a', 'fixture/b'),
        Edge('fixture/b', 'fixture/a')
      ],
      forcedPackages: const [],
      unknownPathPolicy: UnknownPathPolicy.error,
      changedPaths: const ['a/file'],
    );
    expect(
      () => BuildToolCore.evaluateDiffSelection(cycle),
      throwsA(isA<ArgumentError>()
          .having((error) => error.message, 'message', 'DIFF_EDGE_CYCLE')),
    );

    final collidingRoots = DiffSelectionInput(
      packages: const [
        PackageSpec('fixture/a', 'straße', SourceMode.packagePrefix, []),
        PackageSpec('fixture/b', 'strasse', SourceMode.packagePrefix, []),
      ],
      edges: const [],
      forcedPackages: const [],
      unknownPathPolicy: UnknownPathPolicy.error,
      changedPaths: const ['strasse/file'],
    );
    expect(
      () => BuildToolCore.evaluateDiffSelection(collidingRoots),
      throwsA(isA<ArgumentError>()
          .having((error) => error.message, 'message', 'DIFF_PATH_INVALID')),
    );

    final descendingGlob = DiffSelectionInput(
      packages: const [
        PackageSpec('fixture/a', 'a', SourceMode.strictGlobs, ['[z-a]']),
      ],
      edges: const [],
      forcedPackages: const [],
      unknownPathPolicy: UnknownPathPolicy.error,
      changedPaths: const ['a/z'],
    );
    expect(
      () => BuildToolCore.evaluateDiffSelection(descendingGlob),
      throwsA(isA<ArgumentError>()
          .having((error) => error.message, 'message', 'DIFF_GLOB_INVALID')),
    );
  });

  test('covers empty, partial-cycle, glob, Unicode, and boundary edges', () {
    expect(
      BuildToolCore.evaluateGraph(const GraphInput([], [])),
      equals(const GraphResult([], [], '')),
    );
    final cycle = BuildToolCore.evaluateGraph(
      const GraphInput(
        ['fixture/free', 'fixture/a', 'fixture/b'],
        [Edge('fixture/a', 'fixture/b'), Edge('fixture/b', 'fixture/a')],
      ),
    );
    expect(cycle.errorCode, 'GRAPH_CYCLE');
    expect(cycle.edges, isEmpty);
    expect(cycle.levels, isEmpty);

    const spec = PackageSpec(
      'fixture/p',
      'p',
      SourceMode.strictGlobs,
      ['src/**/[a-c]*.txt', 'literal[', 'emoji/[😀].txt'],
    );
    expect(
      BuildToolCore.evaluateDiffSelection(
        _diffInput(
            spec, const ['p/src/deep/bee.txt', 'p/BUILD_debug', 'p/BUILD']),
      ).changedPackages,
      ['fixture/p'],
    );
    expect(
      BuildToolCore.evaluateDiffSelection(
          _diffInput(spec, const ['p/src/deep/x.bin'])).changedPackages,
      isEmpty,
    );
    expect(
      BuildToolCore.evaluateDiffSelection(
          _diffInput(spec, const ['p/literal['])).changedPackages,
      ['fixture/p'],
    );
    expect(
      BuildToolCore.evaluateDiffSelection(
          _diffInput(spec, const ['p/emoji/😀.txt'])).changedPackages,
      ['fixture/p'],
    );

    final escaped = RepositoryBoundary(
      1,
      'line\nquote"slash\\tab\t',
      [
        BoundaryRule(
          'id',
          'origin',
          const AppliesTo(['p'], [], []),
          const [BoundaryInput('path', 'role', 'generated')],
          'back\bform\freturn\r',
          'owner',
        ),
      ],
    );
    expect(escaped.digest(), hasLength(64));
  });
}

DiffSelectionInput _diffInput(PackageSpec spec, List<String> changedPaths) =>
    DiffSelectionInput(
      packages: [spec],
      edges: const [],
      forcedPackages: const [],
      unknownPathPolicy: UnknownPathPolicy.error,
      changedPaths: changedPaths,
    );

void _assertGraphFixture(Map<String, Object?> fixture) {
  final input = fixture['input']! as Map<String, Object?>;
  final options = input['options']! as Map<String, Object?>;
  final actual = BuildToolCore.evaluateGraph(
    GraphInput(_strings(options['packages']), _edges(options['edges'])),
  );
  final expected = fixture['expected']! as Map<String, Object?>;
  if (expected['outcome'] == 'error') {
    final diagnostics = expected['diagnostics']! as List<Object?>;
    expect(
        actual.errorCode, (diagnostics.first! as Map<String, Object?>)['code']);
    expect(actual.edges, isEmpty);
    expect(actual.levels, isEmpty);
  } else {
    final result = expected['result']! as Map<String, Object?>;
    expect(actual.errorCode, isEmpty);
    expect(actual.edges, equals(_edges(result['edges'])));
    expect(actual.levels, equals(_nestedStrings(result['levels'])));
  }
}

void _assertDiffFixture(Map<String, Object?> fixture, String fixtureRoot) {
  final input = fixture['input']! as Map<String, Object?>;
  final options = input['options']! as Map<String, Object?>;
  final digest = options['boundary_sha256'] as String? ?? '';
  final boundary = digest.isEmpty
      ? null
      : _boundary(
          jsonDecode(
            File('$fixtureRoot${Platform.pathSeparator}repository-source-input-boundary.json')
                .readAsStringSync(),
          ) as Map<String, Object?>,
        );
  final actual = BuildToolCore.evaluateDiffSelection(
    DiffSelectionInput(
      packages: _packages(options['packages']),
      edges: _edges(options['edges']),
      forcedPackages: _strings(options['forced_packages']),
      unknownPathPolicy: options['unknown_path_policy'] == 'all'
          ? UnknownPathPolicy.all
          : UnknownPathPolicy.error,
      changedPaths: _strings(input['changed_paths']),
      boundarySha256: digest,
      boundary: boundary,
    ),
  );
  final expected = fixture['expected']! as Map<String, Object?>;
  if (expected['outcome'] == 'error') {
    final diagnostics = expected['diagnostics']! as List<Object?>;
    expect(
        actual.errorCode, (diagnostics.first! as Map<String, Object?>)['code']);
    expect(actual.changedPackages, isEmpty);
    expect(actual.affectedPackages, isEmpty);
    expect(actual.prerequisitePackages, isEmpty);
  } else {
    final result = expected['result']! as Map<String, Object?>;
    expect(actual.errorCode, isEmpty);
    expect(
        actual.changedPackages, equals(_strings(result['changed_packages'])));
    expect(
        actual.affectedPackages, equals(_strings(result['affected_packages'])));
    expect(actual.prerequisitePackages,
        equals(_strings(result['prerequisite_packages'])));
  }
}

List<PackageSpec> _packages(Object? value) =>
    (value! as List<Object?>).map((item) {
      final node = item! as Map<String, Object?>;
      return PackageSpec(
        node['name']! as String,
        node['rel_path']! as String,
        node['source_mode'] == 'package_prefix'
            ? SourceMode.packagePrefix
            : SourceMode.strictGlobs,
        _strings(node['source_globs']),
      );
    }).toList();

RepositoryBoundary _boundary(Map<String, Object?> node) => RepositoryBoundary(
      node['schema_version']! as int,
      node['language_source_input_registry_sha256']! as String,
      (node['boundaries']! as List<Object?>).map((item) {
        final rule = item! as Map<String, Object?>;
        final applies = rule['applies_to']! as Map<String, Object?>;
        return BoundaryRule(
          rule['id']! as String,
          rule['input_origin']! as String,
          AppliesTo(
            _strings(applies['exact_roots']),
            _strings(applies['descendant_roots']),
            _strings(applies['excluded_roots']),
          ),
          (rule['inputs']! as List<Object?>).map((input) {
            final value = input! as Map<String, Object?>;
            return BoundaryInput(
              value['path']! as String,
              value['role']! as String,
              value['generated_component'] as String? ?? '',
            );
          }).toList(),
          rule['reason']! as String,
          rule['owner']! as String,
        );
      }).toList(),
    );

List<String> _strings(Object? value) => value == null
    ? <String>[]
    : (value as List<Object?>).cast<String>().toList();

List<List<String>> _nestedStrings(Object? value) =>
    (value! as List<Object?>).map(_strings).toList();

List<Edge> _edges(Object? value) => (value! as List<Object?>).map((edge) {
      final values = edge! as List<Object?>;
      return Edge(values[0]! as String, values[1]! as String);
    }).toList();

Directory _repositoryRoot() {
  var current = Directory.current.absolute;
  while (true) {
    if (Directory(
            '${current.path}${Platform.pathSeparator}code${Platform.pathSeparator}specs${Platform.pathSeparator}fixtures${Platform.pathSeparator}build-tool-v1')
        .existsSync()) {
      return current;
    }
    final parent = current.parent;
    if (parent.path == current.path) {
      throw StateError('repository root not found');
    }
    current = parent;
  }
}
