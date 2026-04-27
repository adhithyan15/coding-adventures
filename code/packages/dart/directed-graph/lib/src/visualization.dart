import 'graph.dart';
import 'labeled_graph.dart';

class DotOptions {
  const DotOptions({
    this.name = 'G',
    this.nodeAttrs = const <String, Map<String, String>>{},
    this.initial,
    this.rankdir = 'LR',
  });

  final String name;
  final Map<String, Map<String, String>> nodeAttrs;
  final String? initial;
  final String rankdir;
}

class MermaidOptions {
  const MermaidOptions({
    this.direction = 'LR',
    this.initial,
  });

  final String direction;
  final String? initial;
}

String toDot(
  Object graph, {
  DotOptions options = const DotOptions(),
}) {
  final lines = <String>[];
  lines.add('digraph ${options.name} {');
  lines.add('    rankdir=${options.rankdir};');

  if (options.initial != null) {
    lines.add('    "" [shape=none];');
    lines.add('    "" -> ${options.initial};');
  }

  final nodes = _nodesOf(graph).toList()..sort();
  for (final node in nodes) {
    final attrs = options.nodeAttrs[node];
    if (attrs == null || attrs.isEmpty) {
      lines.add('    $node;');
    } else {
      lines.add('    $node ${_formatDotAttrs(attrs)};');
    }
  }

  if (graph is LabeledDirectedGraph) {
    final labels = <String, List<String>>{};
    for (final edge in graph.edges()) {
      labels.putIfAbsent('${edge.from}\u0000${edge.to}', () => <String>[]).add(edge.label);
    }
    final keys = labels.keys.toList()..sort();
    for (final key in keys) {
      final parts = key.split('\u0000');
      final labelValues = labels[key]!.toList()..sort();
      final label = labelValues.join(', ');
      lines.add('    ${parts[0]} -> ${parts[1]} [label="$label"];');
    }
  } else if (graph is Graph) {
    for (final edge in graph.edges()) {
      lines.add('    ${edge.from} -> ${edge.to};');
    }
  } else {
    throw ArgumentError('Unsupported graph type: ${graph.runtimeType}');
  }

  lines.add('}');
  return lines.join('\n');
}

String toMermaid(
  Object graph, {
  MermaidOptions options = const MermaidOptions(),
}) {
  final lines = <String>['flowchart ${options.direction}'];

  if (options.initial != null) {
    lines.add('    __start__(("")) --> ${options.initial}');
  }

  if (graph is LabeledDirectedGraph) {
    final grouped = <String, List<String>>{};
    for (final edge in graph.edges()) {
      grouped.putIfAbsent('${edge.from}\u0000${edge.to}', () => <String>[]).add(edge.label);
    }
    final keys = grouped.keys.toList()..sort();
    for (final key in keys) {
      final parts = key.split('\u0000');
      final labelValues = grouped[key]!.toList()..sort();
      final label = labelValues.join(', ');
      lines.add('    ${parts[0]} -->|$label| ${parts[1]}');
    }
  } else if (graph is Graph) {
    for (final edge in graph.edges()) {
      lines.add('    ${edge.from} --> ${edge.to}');
    }
  } else {
    throw ArgumentError('Unsupported graph type: ${graph.runtimeType}');
  }

  return lines.join('\n');
}

String toAsciiTable(Object graph) {
  if (graph is LabeledDirectedGraph) {
    final rows = <String>['STATE | SUCCESSOR | LABELS'];
    for (final edge in graph.edges()) {
      rows.add('${edge.from} | ${edge.to} | ${edge.label}');
    }
    return rows.join('\n');
  }

  if (graph is Graph) {
    final rows = <String>['NODE | SUCCESSORS'];
    for (final node in graph.nodes()) {
      rows.add('$node | ${graph.successors(node).join(", ")}');
    }
    return rows.join('\n');
  }

  throw ArgumentError('Unsupported graph type: ${graph.runtimeType}');
}

List<String> _nodesOf(Object graph) {
  if (graph is Graph) {
    return graph.nodes();
  }
  if (graph is LabeledDirectedGraph) {
    return graph.nodes();
  }
  throw ArgumentError('Unsupported graph type: ${graph.runtimeType}');
}

String _formatDotAttrs(Map<String, String> attrs) {
  final entries = attrs.entries.toList()..sort((left, right) => left.key.compareTo(right.key));
  final parts = entries.map((entry) => '${entry.key}=${entry.value}').join(', ');
  return '[$parts]';
}
