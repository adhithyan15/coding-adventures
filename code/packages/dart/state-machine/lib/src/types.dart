typedef Action = void Function(String source, String event, String target);

class TransitionRecord {
  const TransitionRecord({
    required this.source,
    required this.event,
    required this.target,
    required this.actionName,
  });

  final String source;
  final String? event;
  final String target;
  final String? actionName;

  @override
  bool operator ==(Object other) {
    return other is TransitionRecord &&
        other.source == source &&
        other.event == event &&
        other.target == target &&
        other.actionName == actionName;
  }

  @override
  int get hashCode => Object.hash(source, event, target, actionName);

  @override
  String toString() {
    return 'TransitionRecord(source: $source, event: $event, '
        'target: $target, actionName: $actionName)';
  }
}

String transitionKey(String state, String event) => '$state\u0000$event';
