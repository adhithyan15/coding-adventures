import 'package:coding_adventures_directed_graph/directed_graph.dart';

import 'types.dart';

class DFA {
  DFA(
    Set<String> states,
    Set<String> alphabet,
    Map<String, String> transitions,
    String initial,
    Set<String> accepting, [
    Map<String, Action>? actions,
  ])  : _states = Set<String>.unmodifiable(states),
        _alphabet = Set<String>.unmodifiable(alphabet),
        _transitions = Map<String, String>.unmodifiable(transitions),
        _initial = initial,
        _accepting = Set<String>.unmodifiable(accepting),
        _actions = Map<String, Action>.unmodifiable(actions ?? const {}),
        _graph = LabeledDirectedGraph() {
    if (states.isEmpty) {
      throw ArgumentError('States set must be non-empty');
    }
    if (!states.contains(initial)) {
      throw ArgumentError(
        "Initial state '$initial' is not in the states set [${states.toList()..sort()}]",
      );
    }
    for (final state in accepting) {
      if (!states.contains(state)) {
        throw ArgumentError(
          "Accepting state '$state' is not in the states set [${states.toList()..sort()}]",
        );
      }
    }
    for (final entry in transitions.entries) {
      final sep = entry.key.indexOf('\u0000');
      final source = entry.key.substring(0, sep);
      final event = entry.key.substring(sep + 1);
      final target = entry.value;
      if (!states.contains(source)) {
        throw ArgumentError("Transition source '$source' is not in the states set");
      }
      if (!alphabet.contains(event)) {
        throw ArgumentError(
          "Transition event '$event' is not in the alphabet [${alphabet.toList()..sort()}]",
        );
      }
      if (!states.contains(target)) {
        throw ArgumentError(
          "Transition target '$target' (from ($source, $event)) is not in the states set",
        );
      }
    }
    for (final key in _actions.keys) {
      if (!transitions.containsKey(key)) {
        final sep = key.indexOf('\u0000');
        final source = key.substring(0, sep);
        final event = key.substring(sep + 1);
        throw ArgumentError(
          'Action defined for ($source, $event) but no transition exists for that pair',
        );
      }
    }

    for (final state in states) {
      _graph.addNode(state);
    }
    for (final entry in transitions.entries) {
      final sep = entry.key.indexOf('\u0000');
      final source = entry.key.substring(0, sep);
      final event = entry.key.substring(sep + 1);
      _graph.addEdge(source, entry.value, event);
    }

    _currentState = initial;
  }

  final Set<String> _states;
  final Set<String> _alphabet;
  final Map<String, String> _transitions;
  final String _initial;
  final Set<String> _accepting;
  final Map<String, Action> _actions;
  final LabeledDirectedGraph _graph;

  late String _currentState;
  final List<TransitionRecord> _trace = <TransitionRecord>[];

  Set<String> get states => Set<String>.unmodifiable(_states);
  Set<String> get alphabet => Set<String>.unmodifiable(_alphabet);
  Map<String, String> get transitions => Map<String, String>.from(_transitions);
  String get initial => _initial;
  Set<String> get accepting => Set<String>.unmodifiable(_accepting);
  String get currentState => _currentState;
  List<TransitionRecord> get trace => List<TransitionRecord>.unmodifiable(_trace);

  String process(String event) {
    if (!_alphabet.contains(event)) {
      throw ArgumentError(
        "Event '$event' is not in the alphabet [${_sorted(_alphabet)}]",
      );
    }

    final key = transitionKey(_currentState, event);
    final target = _transitions[key];
    if (target == null) {
      throw StateError(
        "No transition defined for (state='$_currentState', event='$event')",
      );
    }

    String? actionName;
    final action = _actions[key];
    if (action != null) {
      action(_currentState, event, target);
      actionName = _nameOfAction(action);
    }

    _trace.add(
      TransitionRecord(
        source: _currentState,
        event: event,
        target: target,
        actionName: actionName,
      ),
    );
    _currentState = target;
    return target;
  }

  List<TransitionRecord> processSequence(List<String> events) {
    final start = _trace.length;
    for (final event in events) {
      process(event);
    }
    return List<TransitionRecord>.unmodifiable(_trace.sublist(start));
  }

  bool accepts(List<String> events) {
    var state = _initial;
    for (final event in events) {
      if (!_alphabet.contains(event)) {
        throw ArgumentError(
          "Event '$event' is not in the alphabet [${_sorted(_alphabet)}]",
        );
      }
      final target = _transitions[transitionKey(state, event)];
      if (target == null) {
        return false;
      }
      state = target;
    }
    return _accepting.contains(state);
  }

  void reset() {
    _currentState = _initial;
    _trace.clear();
  }

  Set<String> reachableStates() {
    final reachable = _graph.transitiveClosure(_initial).toSet();
    reachable.add(_initial);
    return Set<String>.unmodifiable(reachable);
  }

  bool isComplete() {
    for (final state in _states) {
      for (final event in _alphabet) {
        if (!_transitions.containsKey(transitionKey(state, event))) {
          return false;
        }
      }
    }
    return true;
  }

  List<String> validate() {
    final warnings = <String>[];
    final reachable = reachableStates();
    final unreachable = _states.where((state) => !reachable.contains(state)).toList()
      ..sort();
    if (unreachable.isNotEmpty) {
      warnings.add('Unreachable states: [${unreachable.join(", ")}]');
    }

    final unreachableAccepting = _accepting
        .where((state) => !reachable.contains(state))
        .toList()
      ..sort();
    if (unreachableAccepting.isNotEmpty) {
      warnings.add(
        'Unreachable accepting states: [${unreachableAccepting.join(", ")}]',
      );
    }

    final missing = <String>[];
    final sortedStates = _states.toList()..sort();
    final sortedAlphabet = _alphabet.toList()..sort();
    for (final state in sortedStates) {
      for (final event in sortedAlphabet) {
        if (!_transitions.containsKey(transitionKey(state, event))) {
          missing.add('($state, $event)');
        }
      }
    }
    if (missing.isNotEmpty) {
      warnings.add('Missing transitions: ${missing.join(", ")}');
    }

    return List<String>.unmodifiable(warnings);
  }

  String toDot() {
    final lines = <String>[
      'digraph DFA {',
      '    rankdir=LR;',
      '',
      '    __start [shape=point, width=0.2];',
      '    __start -> "${_escape(_initial)}";',
      '',
    ];

    final sortedStates = _states.toList()..sort();
    for (final state in sortedStates) {
      final shape = _accepting.contains(state) ? 'doublecircle' : 'circle';
      lines.add('    "${_escape(state)}" [shape=$shape];');
    }
    lines.add('');

    final edgeLabels = <String, List<String>>{};
    final keys = _transitions.keys.toList()..sort();
    for (final key in keys) {
      final sep = key.indexOf('\u0000');
      final source = key.substring(0, sep);
      final event = key.substring(sep + 1);
      final target = _transitions[key]!;
      edgeLabels.putIfAbsent('$source\u0000$target', () => <String>[]).add(event);
    }
    final edges = edgeLabels.keys.toList()..sort();
    for (final edge in edges) {
      final sep = edge.indexOf('\u0000');
      final source = edge.substring(0, sep);
      final target = edge.substring(sep + 1);
      final labels = edgeLabels[edge]!..sort();
      lines.add(
        '    "${_escape(source)}" -> "${_escape(target)}" '
        '[label="${labels.map(_escape).join(", ")}"];',
      );
    }

    lines.add('}');
    return lines.join('\n');
  }

  String toAscii() {
    final sortedEvents = _alphabet.toList()..sort();
    final sortedStates = _states.toList()..sort();

    final stateWidth = sortedStates
        .map((state) => state.length + 4)
        .fold<int>(5, (left, right) => left > right ? left : right);
    var eventWidth = 5;
    for (final event in sortedEvents) {
      if (event.length > eventWidth) {
        eventWidth = event.length;
      }
    }
    for (final state in sortedStates) {
      for (final event in sortedEvents) {
        final target = _transitions[transitionKey(state, event)] ?? '—';
        if (target.length > eventWidth) {
          eventWidth = target.length;
        }
      }
    }

    final lines = <String>[];
    final header = StringBuffer('${' ' * stateWidth}│');
    for (final event in sortedEvents) {
      header.write(' ${event.padRight(eventWidth)} │');
    }
    lines.add(header.toString());

    final separator = StringBuffer('${'─' * stateWidth}┼');
    for (var index = 0; index < sortedEvents.length; index++) {
      separator.write('─' * (eventWidth + 2));
      if (index < sortedEvents.length - 1) {
        separator.write('┼');
      }
    }
    lines.add(separator.toString());

    for (final state in sortedStates) {
      final markers = StringBuffer();
      if (state == _initial) {
        markers.write('>');
      }
      if (_accepting.contains(state)) {
        markers.write('*');
      }
      final label = markers.isEmpty ? '  $state' : '${markers.toString()} $state';
      final row = StringBuffer('${label.padRight(stateWidth)}│');
      for (final event in sortedEvents) {
        final target = _transitions[transitionKey(state, event)] ?? '—';
        row.write(' ${target.padRight(eventWidth)} │');
      }
      lines.add(row.toString());
    }

    return lines.join('\n');
  }

  List<List<String>> toTable() {
    final sortedEvents = _alphabet.toList()..sort();
    final sortedStates = _states.toList()..sort();
    final rows = <List<String>>[
      <String>['State', ...sortedEvents],
    ];
    for (final state in sortedStates) {
      final row = <String>[state];
      for (final event in sortedEvents) {
        row.add(_transitions[transitionKey(state, event)] ?? '—');
      }
      rows.add(List<String>.unmodifiable(row));
    }
    return List<List<String>>.unmodifiable(rows);
  }
}

String _sorted(Set<String> values) {
  final result = values.toList()..sort();
  return result.join(', ');
}

String _nameOfAction(Action action) {
  final description = action.toString();
  final match = RegExp("Function '([^']+)'").firstMatch(description);
  return match?.group(1) ?? description;
}

String _escape(String value) => value.replaceAll(r'"', r'\"');

