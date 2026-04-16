import 'dfa.dart';
import 'types.dart';

DFA minimize(DFA dfa) {
  final reachable = dfa.reachableStates();
  final accepting = dfa.accepting.where(reachable.contains).toSet();
  final filteredTransitions = <String, String>{};

  for (final entry in dfa.transitions.entries) {
    final sep = entry.key.indexOf('\u0000');
    final source = entry.key.substring(0, sep);
    if (reachable.contains(source) && reachable.contains(entry.value)) {
      filteredTransitions[entry.key] = entry.value;
    }
  }

  final nonAccepting = reachable.where((state) => !accepting.contains(state)).toSet();
  var partitions = <Set<String>>[];
  if (accepting.isNotEmpty) {
    partitions.add(accepting);
  }
  if (nonAccepting.isNotEmpty) {
    partitions.add(nonAccepting);
  }
  if (partitions.isEmpty) {
    return dfa;
  }

  final alphabet = dfa.alphabet.toList()..sort();
  var changed = true;
  while (changed) {
    changed = false;
    final nextPartitions = <Set<String>>[];
    for (final group in partitions) {
      final split = _splitGroup(group, alphabet, filteredTransitions, partitions);
      if (split.length > 1) {
        changed = true;
      }
      nextPartitions.addAll(split);
    }
    partitions = nextPartitions;
  }

  final stateToPartition = <String, Set<String>>{};
  for (final partition in partitions) {
    for (final state in partition) {
      stateToPartition[state] = partition;
    }
  }

  final newStates = <String>{};
  final newTransitions = <String, String>{};
  final newAccepting = <String>{};
  for (final partition in partitions) {
    final name = _partitionName(partition);
    newStates.add(name);
    if (partition.any(accepting.contains)) {
      newAccepting.add(name);
    }
    final representative = partition.toList()..sort();
    for (final event in alphabet) {
      final target = filteredTransitions[transitionKey(representative.first, event)];
      if (target != null) {
        newTransitions[transitionKey(name, event)] =
            _partitionName(stateToPartition[target]!);
      }
    }
  }

  final newInitial = _partitionName(stateToPartition[dfa.initial]!);
  return DFA(
    newStates,
    dfa.alphabet,
    newTransitions,
    newInitial,
    newAccepting,
  );
}

List<Set<String>> _splitGroup(
  Set<String> group,
  List<String> alphabet,
  Map<String, String> transitions,
  List<Set<String>> partitions,
) {
  if (group.length <= 1) {
    return <Set<String>>[group];
  }

  final stateToPartition = <String, int>{};
  for (var index = 0; index < partitions.length; index++) {
    for (final state in partitions[index]) {
      stateToPartition[state] = index;
    }
  }

  for (final event in alphabet) {
    final signatures = <int?, Set<String>>{};
    for (final state in group) {
      final target = transitions[transitionKey(state, event)];
      final signature = target == null ? null : stateToPartition[target];
      signatures.putIfAbsent(signature, () => <String>{}).add(state);
    }
    if (signatures.length > 1) {
      return signatures.values.toList();
    }
  }

  return <Set<String>>[group];
}

String _partitionName(Set<String> partition) {
  final members = partition.toList()..sort();
  if (members.length == 1) {
    return members.first;
  }
  return '{${members.join(",")}}';
}

