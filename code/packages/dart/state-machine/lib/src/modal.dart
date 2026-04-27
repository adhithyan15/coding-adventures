import 'package:coding_adventures_directed_graph/directed_graph.dart';

import 'dfa.dart';
import 'types.dart';

class ModeTransitionRecord {
  const ModeTransitionRecord({
    required this.fromMode,
    required this.trigger,
    required this.toMode,
  });

  final String fromMode;
  final String trigger;
  final String toMode;

  @override
  bool operator ==(Object other) {
    return other is ModeTransitionRecord &&
        other.fromMode == fromMode &&
        other.trigger == trigger &&
        other.toMode == toMode;
  }

  @override
  int get hashCode => Object.hash(fromMode, trigger, toMode);
}

class ModalStateMachine {
  ModalStateMachine(
    Map<String, DFA> modes,
    Map<String, String> modeTransitions,
    String initialMode,
  )   : _modes = Map<String, DFA>.unmodifiable(modes),
        _modeTransitions = Map<String, String>.unmodifiable(modeTransitions),
        _initialMode = initialMode,
        _modeGraph = LabeledDirectedGraph() {
    if (modes.isEmpty) {
      throw ArgumentError('At least one mode must be provided');
    }
    if (!modes.containsKey(initialMode)) {
      throw ArgumentError("Initial mode '$initialMode' is not in the modes map");
    }
    for (final entry in modeTransitions.entries) {
      final sep = entry.key.indexOf('\u0000');
      final source = entry.key.substring(0, sep);
      if (!modes.containsKey(source)) {
        throw ArgumentError(
          "Mode transition source '$source' is not a valid mode",
        );
      }
      if (!modes.containsKey(entry.value)) {
        throw ArgumentError(
          "Mode transition target '${entry.value}' is not a valid mode",
        );
      }
    }

    for (final mode in modes.keys) {
      _modeGraph.addNode(mode);
    }
    for (final entry in modeTransitions.entries) {
      final sep = entry.key.indexOf('\u0000');
      final source = entry.key.substring(0, sep);
      final trigger = entry.key.substring(sep + 1);
      _modeGraph.addEdge(source, entry.value, trigger);
    }

    _currentMode = initialMode;
  }

  final Map<String, DFA> _modes;
  final Map<String, String> _modeTransitions;
  final String _initialMode;
  final LabeledDirectedGraph _modeGraph;

  late String _currentMode;
  final List<ModeTransitionRecord> _modeTrace = <ModeTransitionRecord>[];

  String get currentMode => _currentMode;
  DFA get activeMachine => _modes[_currentMode]!;
  Map<String, DFA> get modes => Map<String, DFA>.from(_modes);
  List<ModeTransitionRecord> get modeTrace =>
      List<ModeTransitionRecord>.unmodifiable(_modeTrace);

  String process(String event) => activeMachine.process(event);

  String switchMode(String trigger) {
    final newMode = _modeTransitions[transitionKey(_currentMode, trigger)];
    if (newMode == null) {
      throw StateError(
        "No mode transition for (mode='$_currentMode', trigger='$trigger')",
      );
    }

    final oldMode = _currentMode;
    _modes[newMode]!.reset();
    _modeTrace.add(
      ModeTransitionRecord(
        fromMode: oldMode,
        trigger: trigger,
        toMode: newMode,
      ),
    );
    _currentMode = newMode;
    return newMode;
  }

  void reset() {
    _currentMode = _initialMode;
    _modeTrace.clear();
    for (final machine in _modes.values) {
      machine.reset();
    }
  }
}

