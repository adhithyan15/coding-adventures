import 'package:coding_adventures_directed_graph/directed_graph.dart';

import 'dfa.dart';
import 'types.dart';

const String epsilon = '';
const String EPSILON = epsilon;

typedef NfaTraceEntry = ({
  Set<String> beforeStates,
  String event,
  Set<String> afterStates,
});

class NFA {
  NFA(
    Set<String> states,
    Set<String> alphabet,
    Map<String, Set<String>> transitions,
    String initial,
    Set<String> accepting,
  )   : _states = Set<String>.unmodifiable(states),
        _alphabet = Set<String>.unmodifiable(alphabet),
        _transitions = Map<String, Set<String>>.unmodifiable(
          transitions.map(
            (key, value) => MapEntry<String, Set<String>>(
              key,
              Set<String>.unmodifiable(value),
            ),
          ),
        ),
        _initial = initial,
        _accepting = Set<String>.unmodifiable(accepting),
        _graph = LabeledDirectedGraph() {
    if (states.isEmpty) {
      throw ArgumentError('States set must be non-empty');
    }
    if (alphabet.contains(EPSILON)) {
      throw ArgumentError(
        'Alphabet must not contain the empty string (reserved for epsilon)',
      );
    }
    if (!states.contains(initial)) {
      throw ArgumentError("Initial state '$initial' is not in the states set");
    }
    for (final state in accepting) {
      if (!states.contains(state)) {
        throw ArgumentError("Accepting state '$state' is not in the states set");
      }
    }
    for (final entry in transitions.entries) {
      final sep = entry.key.indexOf('\u0000');
      final source = entry.key.substring(0, sep);
      final event = entry.key.substring(sep + 1);
      if (!states.contains(source)) {
        throw ArgumentError("Transition source '$source' is not in the states set");
      }
      if (event != EPSILON && !alphabet.contains(event)) {
        throw ArgumentError(
          "Transition event '$event' is not in the alphabet and is not epsilon",
        );
      }
      for (final target in entry.value) {
        if (!states.contains(target)) {
          throw ArgumentError(
            "Transition target '$target' (from ($source, '$event')) is not in the states set",
          );
        }
      }
    }

    for (final state in states) {
      _graph.addNode(state);
    }
    for (final entry in transitions.entries) {
      final sep = entry.key.indexOf('\u0000');
      final source = entry.key.substring(0, sep);
      final event = entry.key.substring(sep + 1);
      for (final target in entry.value) {
        _graph.addEdge(source, target, event);
      }
    }

    _currentStates = epsilonClosure(<String>{initial});
  }

  final Set<String> _states;
  final Set<String> _alphabet;
  final Map<String, Set<String>> _transitions;
  final String _initial;
  final Set<String> _accepting;
  final LabeledDirectedGraph _graph;

  late Set<String> _currentStates;

  Set<String> get states => Set<String>.unmodifiable(_states);
  Set<String> get alphabet => Set<String>.unmodifiable(_alphabet);
  String get initial => _initial;
  Set<String> get accepting => Set<String>.unmodifiable(_accepting);
  Set<String> get currentStates => Set<String>.unmodifiable(_currentStates);

  Set<String> epsilonClosure(Set<String> states) {
    final closure = <String>{...states};
    final worklist = <String>[...states];
    while (worklist.isNotEmpty) {
      final state = worklist.removeLast();
      final targets = _transitions[transitionKey(state, EPSILON)] ?? const <String>{};
      for (final target in targets) {
        if (closure.add(target)) {
          worklist.add(target);
        }
      }
    }
    return Set<String>.unmodifiable(closure);
  }

  Set<String> process(String event) {
    if (!_alphabet.contains(event)) {
      throw ArgumentError(
        "Event '$event' is not in the alphabet [${_sorted(_alphabet)}]",
      );
    }

    final nextStates = <String>{};
    for (final state in _currentStates) {
      final targets = _transitions[transitionKey(state, event)] ?? const <String>{};
      nextStates.addAll(targets);
    }
    _currentStates = epsilonClosure(nextStates).toSet();
    return Set<String>.unmodifiable(_currentStates);
  }

  List<NfaTraceEntry> processSequence(List<String> events) {
    final trace = <NfaTraceEntry>[];
    for (final event in events) {
      final before = Set<String>.unmodifiable(_currentStates.toSet());
      final after = process(event);
      trace.add(
        (
          beforeStates: before,
          event: event,
          afterStates: Set<String>.unmodifiable(after.toSet()),
        ),
      );
    }
    return List<NfaTraceEntry>.unmodifiable(trace);
  }

  bool accepts(List<String> events) {
    var current = epsilonClosure(<String>{_initial}).toSet();
    for (final event in events) {
      if (!_alphabet.contains(event)) {
        throw ArgumentError(
          "Event '$event' is not in the alphabet [${_sorted(_alphabet)}]",
        );
      }
      final nextStates = <String>{};
      for (final state in current) {
        final targets = _transitions[transitionKey(state, event)] ?? const <String>{};
        nextStates.addAll(targets);
      }
      current = epsilonClosure(nextStates).toSet();
      if (current.isEmpty) {
        return false;
      }
    }
    return current.any(_accepting.contains);
  }

  void reset() {
    _currentStates = epsilonClosure(<String>{_initial}).toSet();
  }

  DFA toDfa() {
    final startClosure = epsilonClosure(<String>{_initial});
    final dfaStart = stateSetName(startClosure);
    final dfaStates = <String>{dfaStart};
    final dfaTransitions = <String, String>{};
    final dfaAccepting = <String>{};
    final stateMap = <String, Set<String>>{
      dfaStart: Set<String>.from(startClosure),
    };
    if (startClosure.any(_accepting.contains)) {
      dfaAccepting.add(dfaStart);
    }

    final worklist = <String>[dfaStart];
    final sortedAlphabet = _alphabet.toList()..sort();
    while (worklist.isNotEmpty) {
      final currentName = worklist.removeLast();
      final currentNfaStates = stateMap[currentName]!;

      for (final event in sortedAlphabet) {
        final nextNfa = <String>{};
        for (final state in currentNfaStates) {
          nextNfa.addAll(_transitions[transitionKey(state, event)] ?? const <String>{});
        }

        final nextClosure = epsilonClosure(nextNfa);
        if (nextClosure.isEmpty) {
          continue;
        }

        final nextName = stateSetName(nextClosure);
        dfaTransitions[transitionKey(currentName, event)] = nextName;
        if (dfaStates.add(nextName)) {
          stateMap[nextName] = Set<String>.from(nextClosure);
          worklist.add(nextName);
          if (nextClosure.any(_accepting.contains)) {
            dfaAccepting.add(nextName);
          }
        }
      }
    }

    return DFA(
      dfaStates,
      _alphabet,
      dfaTransitions,
      dfaStart,
      dfaAccepting,
    );
  }

  String toDot() {
    final lines = <String>[
      'digraph NFA {',
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
      final label = event == EPSILON ? 'ε' : event;
      final targets = _transitions[key]!.toList()..sort();
      for (final target in targets) {
        edgeLabels.putIfAbsent('$source\u0000$target', () => <String>[]).add(label);
      }
    }

    final edges = edgeLabels.keys.toList()..sort();
    for (final edge in edges) {
      final sep = edge.indexOf('\u0000');
      final source = edge.substring(0, sep);
      final target = edge.substring(sep + 1);
      lines.add(
        '    "${_escape(source)}" -> "${_escape(target)}" '
        '[label="${edgeLabels[edge]!.join(", ")}"];',
      );
    }

    lines.add('}');
    return lines.join('\n');
  }
}

String stateSetName(Set<String> states) {
  final members = states.toList()..sort();
  return '{${members.join(",")}}';
}

String _sorted(Set<String> values) {
  final result = values.toList()..sort();
  return result.join(', ');
}

String _escape(String value) => value.replaceAll(r'"', r'\"');
