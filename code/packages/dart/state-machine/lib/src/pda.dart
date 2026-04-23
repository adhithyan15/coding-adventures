class PDATransition {
  const PDATransition({
    required this.source,
    required this.event,
    required this.stackRead,
    required this.target,
    required this.stackPush,
  });

  final String source;
  final String? event;
  final String stackRead;
  final String target;
  final List<String> stackPush;

  @override
  bool operator ==(Object other) {
    return other is PDATransition &&
        other.source == source &&
        other.event == event &&
        other.stackRead == stackRead &&
        other.target == target &&
        _listEquals(other.stackPush, stackPush);
  }

  @override
  int get hashCode => Object.hash(source, event, stackRead, target, stackPush.join('\u0000'));
}

class PDATraceEntry {
  const PDATraceEntry({
    required this.source,
    required this.event,
    required this.stackRead,
    required this.target,
    required this.stackPush,
    required this.stackAfter,
  });

  final String source;
  final String? event;
  final String stackRead;
  final String target;
  final List<String> stackPush;
  final List<String> stackAfter;

  @override
  bool operator ==(Object other) {
    return other is PDATraceEntry &&
        other.source == source &&
        other.event == event &&
        other.stackRead == stackRead &&
        other.target == target &&
        _listEquals(other.stackPush, stackPush) &&
        _listEquals(other.stackAfter, stackAfter);
  }

  @override
  int get hashCode => Object.hash(
        source,
        event,
        stackRead,
        target,
        stackPush.join('\u0000'),
        stackAfter.join('\u0000'),
      );
}

class PushdownAutomaton {
  PushdownAutomaton(
    Set<String> states,
    Set<String> inputAlphabet,
    Set<String> stackAlphabet,
    List<PDATransition> transitions,
    String initial,
    String initialStackSymbol,
    Set<String> accepting,
  )   : _states = Set<String>.unmodifiable(states),
        _inputAlphabet = Set<String>.unmodifiable(inputAlphabet),
        _stackAlphabet = Set<String>.unmodifiable(stackAlphabet),
        _transitions = List<PDATransition>.unmodifiable(transitions),
        _initial = initial,
        _initialStackSymbol = initialStackSymbol,
        _accepting = Set<String>.unmodifiable(accepting),
        _transitionIndex = <String, PDATransition>{} {
    if (states.isEmpty) {
      throw ArgumentError('States set must be non-empty');
    }
    if (!states.contains(initial)) {
      throw ArgumentError("Initial state '$initial' is not in the states set");
    }
    if (!stackAlphabet.contains(initialStackSymbol)) {
      throw ArgumentError(
        "Initial stack symbol '$initialStackSymbol' is not in the stack alphabet",
      );
    }
    for (final state in accepting) {
      if (!states.contains(state)) {
        throw ArgumentError("Accepting state '$state' is not in the states set");
      }
    }
    for (final transition in transitions) {
      final key = _transitionKey(
        transition.source,
        transition.event,
        transition.stackRead,
      );
      if (_transitionIndex.containsKey(key)) {
        throw ArgumentError(
          'Duplicate transition for (${transition.source}, ${transition.event}, '
          '${transition.stackRead}) — this PDA must be deterministic',
        );
      }
      _transitionIndex[key] = transition;
    }

    _currentState = initial;
    _stack = <String>[initialStackSymbol];
  }

  final Set<String> _states;
  final Set<String> _inputAlphabet;
  final Set<String> _stackAlphabet;
  final List<PDATransition> _transitions;
  final String _initial;
  final String _initialStackSymbol;
  final Set<String> _accepting;
  final Map<String, PDATransition> _transitionIndex;

  late String _currentState;
  late List<String> _stack;
  final List<PDATraceEntry> _trace = <PDATraceEntry>[];

  Set<String> get states => Set<String>.unmodifiable(_states);
  Set<String> get inputAlphabet => Set<String>.unmodifiable(_inputAlphabet);
  Set<String> get stackAlphabet => Set<String>.unmodifiable(_stackAlphabet);
  String get currentState => _currentState;
  List<String> get stack => List<String>.unmodifiable(_stack);
  String? get stackTop => _stack.isEmpty ? null : _stack.last;
  List<PDATraceEntry> get trace => List<PDATraceEntry>.unmodifiable(_trace);

  PDATransition? _findTransition(String? event) {
    if (_stack.isEmpty) {
      return null;
    }
    return _transitionIndex[_transitionKey(_currentState, event, _stack.last)];
  }

  void _applyTransition(PDATransition transition) {
    _stack.removeLast();
    for (final symbol in transition.stackPush) {
      _stack.add(symbol);
    }
    _trace.add(
      PDATraceEntry(
        source: transition.source,
        event: transition.event,
        stackRead: transition.stackRead,
        target: transition.target,
        stackPush: List<String>.unmodifiable(List<String>.from(transition.stackPush)),
        stackAfter: List<String>.unmodifiable(List<String>.from(_stack)),
      ),
    );
    _currentState = transition.target;
  }

  bool _tryEpsilon() {
    final transition = _findTransition(null);
    if (transition == null) {
      return false;
    }
    _applyTransition(transition);
    return true;
  }

  String process(String event) {
    final transition = _findTransition(event);
    if (transition == null) {
      throw StateError(
        "No transition for (state='$_currentState', event='$event', stackTop='${stackTop ?? 'null'}')",
      );
    }
    _applyTransition(transition);
    return _currentState;
  }

  List<PDATraceEntry> processSequence(List<String> events) {
    final start = _trace.length;
    for (final event in events) {
      process(event);
    }
    while (_tryEpsilon()) {}
    return List<PDATraceEntry>.unmodifiable(_trace.sublist(start));
  }

  bool accepts(List<String> events) {
    var state = _initial;
    final stack = <String>[_initialStackSymbol];
    for (final event in events) {
      if (stack.isEmpty) {
        return false;
      }
      final transition = _transitionIndex[_transitionKey(state, event, stack.last)];
      if (transition == null) {
        return false;
      }
      stack.removeLast();
      stack.addAll(transition.stackPush);
      state = transition.target;
    }

    final maxEpsilon = _transitions.length + 1;
    for (var index = 0; index < maxEpsilon; index++) {
      if (stack.isEmpty) {
        break;
      }
      final transition = _transitionIndex[_transitionKey(state, null, stack.last)];
      if (transition == null) {
        break;
      }
      stack.removeLast();
      stack.addAll(transition.stackPush);
      state = transition.target;
    }

    return _accepting.contains(state);
  }

  void reset() {
    _currentState = _initial;
    _stack = <String>[_initialStackSymbol];
    _trace.clear();
  }
}

String _transitionKey(String state, String? event, String stackTop) {
  final eventString = event ?? '\u0000null';
  return '$state\u0000$eventString\u0000$stackTop';
}

bool _listEquals(List<String> left, List<String> right) {
  if (identical(left, right)) {
    return true;
  }
  if (left.length != right.length) {
    return false;
  }
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) {
      return false;
    }
  }
  return true;
}
