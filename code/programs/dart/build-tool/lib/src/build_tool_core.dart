import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_sha256/coding_adventures_sha256.dart';

import 'tracked_artifact_unicode17.dart';

enum SourceMode { packagePrefix, strictGlobs }

enum UnknownPathPolicy { all, error }

final class Edge {
  const Edge(this.prerequisite, this.dependent);
  final String prerequisite;
  final String dependent;

  @override
  bool operator ==(Object other) =>
      other is Edge &&
      prerequisite == other.prerequisite &&
      dependent == other.dependent;
  @override
  int get hashCode => Object.hash(prerequisite, dependent);
  @override
  String toString() => 'Edge($prerequisite, $dependent)';
}

final class GraphInput {
  const GraphInput(this.packages, this.edges);
  final List<String> packages;
  final List<Edge> edges;
}

final class GraphResult {
  const GraphResult(this.edges, this.levels, this.errorCode);
  final List<Edge> edges;
  final List<List<String>> levels;
  final String errorCode;

  @override
  bool operator ==(Object other) =>
      other is GraphResult &&
      _listEquals(edges, other.edges) &&
      _nestedListEquals(levels, other.levels) &&
      errorCode == other.errorCode;
  @override
  int get hashCode => Object.hash(Object.hashAll(edges),
      Object.hashAll(levels.map(Object.hashAll)), errorCode);
}

final class PackageSpec {
  const PackageSpec(this.name, this.relPath, this.sourceMode, this.sourceGlobs);
  final String name;
  final String relPath;
  final SourceMode sourceMode;
  final List<String> sourceGlobs;
}

final class AppliesTo {
  const AppliesTo(this.exactRoots, this.descendantRoots, this.excludedRoots);
  final List<String> exactRoots;
  final List<String> descendantRoots;
  final List<String> excludedRoots;
}

final class BoundaryInput {
  const BoundaryInput(this.path, this.role, this.generatedComponent);
  final String path;
  final String role;
  final String generatedComponent;
}

final class BoundaryRule {
  const BoundaryRule(this.id, this.inputOrigin, this.appliesTo, this.inputs,
      this.reason, this.owner);
  final String id;
  final String inputOrigin;
  final AppliesTo appliesTo;
  final List<BoundaryInput> inputs;
  final String reason;
  final String owner;
}

final class RepositoryBoundary {
  const RepositoryBoundary(this.schemaVersion,
      this.languageSourceInputRegistrySha256, this.boundaries);
  final int schemaVersion;
  final String languageSourceInputRegistrySha256;
  final List<BoundaryRule> boundaries;

  String digest() {
    final encoded = utf8.encode(_canonicalBoundary(this));
    final hasher = Sha256Hasher()
      ..update(_boundaryDomain)
      ..update(_u64BigEndian(encoded.length))
      ..update(encoded);
    return hasher.hexDigest();
  }
}

final class DiffSelectionInput {
  const DiffSelectionInput({
    required this.packages,
    required this.edges,
    required this.forcedPackages,
    required this.unknownPathPolicy,
    required this.changedPaths,
    this.boundarySha256 = '',
    this.boundary,
  });
  final List<PackageSpec> packages;
  final List<Edge> edges;
  final List<String> forcedPackages;
  final UnknownPathPolicy unknownPathPolicy;
  final List<String> changedPaths;
  final String boundarySha256;
  final RepositoryBoundary? boundary;
}

final class DiffSelectionResult {
  const DiffSelectionResult(this.changedPackages, this.affectedPackages,
      this.prerequisitePackages, this.errorCode);
  final List<String> changedPackages;
  final List<String> affectedPackages;
  final List<String> prerequisitePackages;
  final String errorCode;
}

abstract final class BuildToolCore {
  static GraphResult evaluateGraph(GraphInput input) {
    final graph = _validateGraph(input.packages, input.edges);
    final indegree = {for (final name in graph.names) name: 0};
    for (final edge in graph.edges) {
      indegree[edge.dependent] = indegree[edge.dependent]! + 1;
    }
    var ready = indegree.entries
        .where((entry) => entry.value == 0)
        .map((entry) => entry.key)
        .toList()
      ..sort(_compareUnicode);
    final levels = <List<String>>[];
    var visited = 0;
    while (ready.isNotEmpty) {
      final level = List<String>.unmodifiable(ready);
      levels.add(level);
      final next = <String>[];
      for (final name in level) {
        visited++;
        for (final dependent in graph.dependents[name]!) {
          final degree = indegree[dependent]! - 1;
          indegree[dependent] = degree;
          if (degree == 0) next.add(dependent);
        }
      }
      next.sort(_compareUnicode);
      ready = next;
    }
    if (visited != graph.names.length)
      return const GraphResult([], [], 'GRAPH_CYCLE');
    return GraphResult(graph.edges, levels, '');
  }

  static DiffSelectionResult evaluateDiffSelection(DiffSelectionInput input) {
    final graph = _validateGraph(
        input.packages.map((spec) => spec.name).toList(), input.edges);
    _requireCode(!_isCyclic(graph), 'DIFF_EDGE_CYCLE');
    final packages = <String, PackageSpec>{};
    final normalizedRoots = <String>[];
    for (final spec in input.packages) {
      _requireCode(_isPackageName(spec.name), 'DIFF_PACKAGE_INVALID');
      _requireCode(_isPortablePath(spec.relPath), 'DIFF_PATH_INVALID');
      final rootIdentity = TrackedArtifactUnicode17.casefold(
        TrackedArtifactUnicode17.nfc(spec.relPath),
      );
      _requireCode(
          !normalizedRoots.any((root) =>
              rootIdentity == root ||
              rootIdentity.startsWith('$root/') ||
              root.startsWith('$rootIdentity/')),
          'DIFF_PATH_INVALID');
      normalizedRoots.add(rootIdentity);
      _requireCode(
          spec.sourceGlobs.length <= 256 &&
              spec.sourceGlobs.toSet().length == spec.sourceGlobs.length,
          'DIFF_GLOB_INVALID');
      for (final glob in spec.sourceGlobs) {
        _requireCode(_isPortableGlob(glob), 'DIFF_GLOB_INVALID');
      }
      _requireCode(
          spec.sourceMode == SourceMode.strictGlobs || spec.sourceGlobs.isEmpty,
          'DIFF_GLOB_INVALID');
      _requireCode(!packages.containsKey(spec.name), 'DIFF_PACKAGE_DUPLICATE');
      packages[spec.name] = spec;
    }
    _requireCode(
        input.forcedPackages.length <= _maxPackages &&
            input.forcedPackages.toSet().length == input.forcedPackages.length,
        'DIFF_FORCED_PACKAGE_INVALID');
    _requireCode(
        input.changedPaths.length <= _maxPackages &&
            input.changedPaths.toSet().length == input.changedPaths.length,
        'DIFF_PATH_INVALID');
    for (final path in input.changedPaths) {
      _requireCode(_isPortablePath(path), 'DIFF_PATH_INVALID');
    }
    for (final name in input.forcedPackages) {
      _requireCode(packages.containsKey(name), 'DIFF_FORCED_PACKAGE_UNKNOWN');
    }

    final boundaryConsumers = <String, Set<String>>{};
    if (input.boundarySha256.isNotEmpty) {
      _requireCode(_digest.hasMatch(input.boundarySha256),
          'DIFF_BOUNDARY_DIGEST_MISMATCH');
      final boundary = input.boundary;
      _requireCode(
          boundary != null && input.boundarySha256 == boundary.digest(),
          'DIFF_BOUNDARY_DIGEST_MISMATCH');
      for (final spec in input.packages) {
        for (final rule in boundary!.boundaries) {
          if (_applies(rule.appliesTo, spec.relPath)) {
            for (final boundaryInput in rule.inputs) {
              boundaryConsumers
                  .putIfAbsent(boundaryInput.path, () => <String>{})
                  .add(spec.name);
            }
          }
        }
      }
    } else {
      _requireCode(input.boundary == null, 'DIFF_BOUNDARY_DIGEST_MISMATCH');
    }

    var remaining = _maxMatchWork;
    for (final spec in input.packages
        .where((spec) => spec.sourceMode == SourceMode.strictGlobs)) {
      final patternFactor = spec.sourceGlobs
          .fold<int>(0, (sum, glob) => sum + glob.runes.length + 1);
      for (final path
          in input.changedPaths.where((path) => _inside(path, spec.relPath))) {
        final relative = _relative(path, spec.relPath);
        if (!_buildNames.contains(_basename(relative))) {
          final pathFactor = relative.runes.length + 1;
          if (patternFactor != 0 && pathFactor > remaining ~/ patternFactor)
            return _diffError('DIFF_MATCH_LIMIT_EXCEEDED');
          remaining -= patternFactor * pathFactor;
        }
      }
    }

    final changed = input.forcedPackages.toSet();
    var unknown = false;
    for (final path in input.changedPaths) {
      final consumers = boundaryConsumers[path] ?? const <String>{};
      changed.addAll(consumers);
      var known = consumers.isNotEmpty;
      for (final spec in input.packages) {
        if (_inside(path, spec.relPath)) {
          known = true;
          final relative = _relative(path, spec.relPath);
          if (spec.sourceMode == SourceMode.packagePrefix ||
              _buildNames.contains(_basename(relative)) ||
              spec.sourceGlobs.any((glob) => _globMatches(glob, relative))) {
            changed.add(spec.name);
          }
        }
      }
      if (!known) unknown = true;
    }
    if (unknown) {
      if (input.unknownPathPolicy == UnknownPathPolicy.error)
        return _diffError('DIFF_UNKNOWN_PATH');
      changed.addAll(packages.keys);
    }
    final affected = _closure(changed, graph.dependents);
    final prerequisites = _closure(affected, graph.prerequisites)
      ..removeAll(affected);
    return DiffSelectionResult(
      changed.toList()..sort(_compareUnicode),
      affected.toList()..sort(_compareUnicode),
      prerequisites.toList()..sort(_compareUnicode),
      '',
    );
  }
}

const _maxPackages = 4096;
const _maxEdges = 16384;
const _maxMatchWork = 50000000;
final _boundaryDomain = utf8.encode(
    'coding-adventures/build-tool-repository-source-input-boundary/v1\u0000');
const _buildNames = {
  'BUILD',
  'BUILD_windows',
  'BUILD_mac',
  'BUILD_linux',
  'BUILD_mac_and_linux'
};
final _packageName = RegExp(r'^[a-z0-9][a-z0-9._-]*(/[a-z0-9][a-z0-9._-]*)+$');
final _digest = RegExp(r'^[0-9a-f]{64}$');
final _drivePrefix = RegExp(r'^[A-Za-z]:');
const _windowsReserved = {
  'CON',
  'PRN',
  'AUX',
  'NUL',
  r'CONIN$',
  r'CONOUT$',
  r'CLOCK$',
  'COM1',
  'COM2',
  'COM3',
  'COM4',
  'COM5',
  'COM6',
  'COM7',
  'COM8',
  'COM9',
  'LPT1',
  'LPT2',
  'LPT3',
  'LPT4',
  'LPT5',
  'LPT6',
  'LPT7',
  'LPT8',
  'LPT9',
  'COM¹',
  'COM²',
  'COM³',
  'LPT¹',
  'LPT²',
  'LPT³',
};

final class _ValidatedGraph {
  const _ValidatedGraph(
      this.names, this.edges, this.dependents, this.prerequisites);
  final Set<String> names;
  final List<Edge> edges;
  final Map<String, List<String>> dependents;
  final Map<String, List<String>> prerequisites;
}

_ValidatedGraph _validateGraph(List<String> packages, List<Edge> edges) {
  _requireCode(packages.length <= _maxPackages, 'GRAPH_PACKAGE_LIMIT_EXCEEDED');
  _requireCode(edges.length <= _maxEdges, 'GRAPH_EDGE_LIMIT_EXCEEDED');
  final names = <String>{};
  for (final name in packages) {
    _requireCode(_isPackageName(name), 'GRAPH_PACKAGE_INVALID');
    _requireCode(names.add(name), 'GRAPH_PACKAGE_DUPLICATE');
  }
  final uniqueEdges = <Edge>{};
  final dependents = {for (final name in names) name: <String>[]};
  final prerequisites = {for (final name in names) name: <String>[]};
  for (final edge in edges) {
    _requireCode(
        names.contains(edge.prerequisite) && names.contains(edge.dependent),
        'GRAPH_EDGE_UNKNOWN');
    _requireCode(edge.prerequisite != edge.dependent, 'GRAPH_EDGE_SELF');
    _requireCode(uniqueEdges.add(edge), 'GRAPH_EDGE_DUPLICATE');
    dependents[edge.prerequisite]!.add(edge.dependent);
    prerequisites[edge.dependent]!.add(edge.prerequisite);
  }
  for (final values in dependents.values) values.sort(_compareUnicode);
  for (final values in prerequisites.values) values.sort(_compareUnicode);
  final sortedEdges = List<Edge>.of(edges)
    ..sort((a, b) {
      final first = _compareUnicode(a.prerequisite, b.prerequisite);
      return first != 0 ? first : _compareUnicode(a.dependent, b.dependent);
    });
  return _ValidatedGraph(names, sortedEdges, dependents, prerequisites);
}

Set<String> _closure(Set<String> seeds, Map<String, List<String>> adjacency) {
  final result = Set<String>.of(seeds);
  final pending = List<String>.of(seeds);
  for (var index = 0; index < pending.length; index++) {
    for (final next in adjacency[pending[index]]!) {
      if (result.add(next)) pending.add(next);
    }
  }
  return result;
}

bool _isCyclic(_ValidatedGraph graph) {
  final indegree = {for (final name in graph.names) name: 0};
  for (final edge in graph.edges)
    indegree[edge.dependent] = indegree[edge.dependent]! + 1;
  final ready = indegree.entries
      .where((entry) => entry.value == 0)
      .map((entry) => entry.key)
      .toList();
  var visited = 0;
  for (var index = 0; index < ready.length; index++) {
    visited++;
    for (final dependent in graph.dependents[ready[index]]!) {
      final degree = indegree[dependent]! - 1;
      indegree[dependent] = degree;
      if (degree == 0) ready.add(dependent);
    }
  }
  return visited != graph.names.length;
}

DiffSelectionResult _diffError(String code) =>
    DiffSelectionResult(const [], const [], const [], code);

void _requireCode(bool condition, String code) {
  if (!condition) throw ArgumentError(code);
}

bool _applies(AppliesTo appliesTo, String root) =>
    appliesTo.exactRoots.contains(root) ||
    (!appliesTo.excludedRoots.contains(root) &&
        appliesTo.descendantRoots
            .any((ancestor) => root.startsWith('$ancestor/')));
bool _inside(String path, String root) =>
    path == root || path.startsWith('$root/');
String _relative(String path, String root) =>
    path == root ? '' : path.substring(root.length + 1);
String _basename(String path) => path.substring(path.lastIndexOf('/') + 1);
bool _isPackageName(String value) =>
    value.runes.length <= 240 && _packageName.hasMatch(value);

bool _isPortablePath(String value) {
  if (value.runes.isEmpty ||
      value.runes.length > 512 ||
      TrackedArtifactUnicode17.nfc(value) != value ||
      value.startsWith('/') ||
      _drivePrefix.hasMatch(value) ||
      value.contains(r'\') ||
      value.contains('//') ||
      _hasForbidden(value, true)) return false;
  return value.split('/').every((segment) =>
      segment.isNotEmpty &&
      segment != '.' &&
      segment != '..' &&
      !segment.endsWith(' ') &&
      !segment.endsWith('.') &&
      !_reserved(segment));
}

bool _isPortableGlob(String value) {
  if (value.runes.isEmpty ||
      value.runes.length > 512 ||
      TrackedArtifactUnicode17.nfc(value) != value ||
      value.startsWith('/') ||
      _drivePrefix.hasMatch(value) ||
      value.contains(r'\') ||
      value.contains('//') ||
      _hasForbidden(value, false)) return false;
  return value.split('/').every((segment) =>
      segment.isNotEmpty &&
      segment != '.' &&
      segment != '..' &&
      !segment.endsWith(' ') &&
      !segment.endsWith('.') &&
      (segment.contains(RegExp(r'[*\[\]{}]')) || !_reserved(segment)) &&
      !_hasAmbiguousCharacterClass(segment));
}

bool _hasAmbiguousCharacterClass(String segment) {
  final chars = segment.runes.toList();
  var index = 0;
  while (index < chars.length) {
    if (chars[index] != 0x5b) {
      index++;
      continue;
    }
    var cursor = index + 1;
    if (cursor < chars.length && chars[cursor] == 0x21) cursor++;
    var closing = cursor;
    if (closing < chars.length && chars[closing] == 0x5d) closing++;
    while (closing < chars.length && chars[closing] != 0x5d) closing++;
    if (closing == chars.length) {
      index++;
      continue;
    }
    for (var member = cursor; member < closing - 1; member++) {
      final a = chars[member], b = chars[member + 1];
      if ((a == 0x2d && b == 0x2d) ||
          (a == 0x26 && b == 0x26) ||
          (a == 0x7e && b == 0x7e) ||
          (a == 0x7c && b == 0x7c)) return true;
    }
    var member = cursor;
    while (member + 2 < closing) {
      if (chars[member + 1] == 0x2d) {
        if (chars[member] > chars[member + 2]) return true;
        member += 3;
      } else {
        member++;
      }
    }
    index = closing + 1;
  }
  return false;
}

bool _hasForbidden(String value, bool path) {
  final forbidden = (path ? '<>:"|?*' : '<>:"|?').runes.toSet();
  return value.runes.any((rune) => rune < 32 || forbidden.contains(rune));
}

bool _reserved(String segment) => _windowsReserved
    .contains(TrackedArtifactUnicode17.fullUppercase(segment.split('.').first));

int _compareUnicode(String left, String right) {
  final a = left.runes.toList(), b = right.runes.toList();
  for (var index = 0;
      index < (a.length < b.length ? a.length : b.length);
      index++) {
    final compared = a[index].compareTo(b[index]);
    if (compared != 0) return compared;
  }
  return a.length.compareTo(b.length);
}

bool _globMatches(String pattern, String path) {
  final patterns = pattern.split('/'), paths = path.split('/');
  final memo = <(int, int), bool>{};
  bool matches(int pi, int vi) => memo.putIfAbsent((pi, vi), () {
        if (pi == patterns.length) return vi == paths.length;
        if (patterns[pi] == '**')
          return matches(pi + 1, vi) ||
              (vi < paths.length && matches(pi, vi + 1));
        return vi < paths.length &&
            _segmentMatches(patterns[pi], paths[vi]) &&
            matches(pi + 1, vi + 1);
      });
  return matches(0, 0);
}

bool _segmentMatches(String pattern, String value) {
  final p = pattern.runes.toList(), v = value.runes.toList();
  final memo = <(int, int), bool>{};
  bool matches(int pi, int vi) => memo.putIfAbsent((pi, vi), () {
        if (pi == p.length) return vi == v.length;
        if (p[pi] == 0x2a)
          return matches(pi + 1, vi) || (vi < v.length && matches(pi, vi + 1));
        if (p[pi] == 0x5b) {
          var search = pi + 1;
          if (search < p.length && p[search] == 0x21) search++;
          if (search < p.length && p[search] == 0x5d) search++;
          var closing = -1;
          for (var i = search; i < p.length; i++)
            if (p[i] == 0x5d) {
              closing = i;
              break;
            }
          if (closing < 0)
            return vi < v.length && p[pi] == v[vi] && matches(pi + 1, vi + 1);
          return vi < v.length &&
              _classMatches(p, pi + 1, closing, v[vi]) &&
              matches(closing + 1, vi + 1);
        }
        if (p[pi] == 0x3f) return vi < v.length && matches(pi + 1, vi + 1);
        return vi < v.length && p[pi] == v[vi] && matches(pi + 1, vi + 1);
      });
  return matches(0, 0);
}

bool _classMatches(List<int> pattern, int start, int end, int value) {
  final negated = start < end && pattern[start] == 0x21;
  var index = negated ? start + 1 : start, matched = false;
  while (index < end) {
    if (index + 2 < end && pattern[index + 1] == 0x2d) {
      matched =
          matched || (value >= pattern[index] && value <= pattern[index + 2]);
      index += 3;
    } else {
      matched = matched || value == pattern[index];
      index++;
    }
  }
  return negated ? !matched : matched;
}

String _canonicalBoundary(RepositoryBoundary boundary) {
  final rules = boundary.boundaries.map((rule) {
    final inputs = rule.inputs.map((input) {
      final result = <String, Object?>{};
      if (input.generatedComponent.isNotEmpty)
        result['generated_component'] = input.generatedComponent;
      result['path'] = input.path;
      result['role'] = input.role;
      return result;
    }).toList();
    return <String, Object?>{
      'applies_to': <String, Object?>{
        'descendant_roots': rule.appliesTo.descendantRoots,
        'exact_roots': rule.appliesTo.exactRoots,
        'excluded_roots': rule.appliesTo.excludedRoots,
      },
      'id': rule.id,
      'input_origin': rule.inputOrigin,
      'inputs': inputs,
      'owner': rule.owner,
      'reason': rule.reason,
    };
  }).toList();
  return jsonEncode(<String, Object?>{
    'boundaries': rules,
    'language_source_input_registry_sha256':
        boundary.languageSourceInputRegistrySha256,
    'schema_version': boundary.schemaVersion,
  });
}

Uint8List _u64BigEndian(int value) {
  final data = ByteData(8)..setUint64(0, value, Endian.big);
  return data.buffer.asUint8List();
}

bool _listEquals<T>(List<T> a, List<T> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) if (a[i] != b[i]) return false;
  return true;
}

bool _nestedListEquals<T>(List<List<T>> a, List<List<T>> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) if (!_listEquals(a[i], b[i])) return false;
  return true;
}
