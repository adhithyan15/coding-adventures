package com.codingadventures.statemachine;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;

/** Reachability pruning and partition-refinement minimization for DFAs. */
public final class DfaMinimizer {
    private DfaMinimizer() {}

    public static Dfa minimize(Dfa machine) {
        Set<String> reachable = machine.reachableStates();
        Set<String> reachableAccepting = new LinkedHashSet<>(machine.accepting());
        reachableAccepting.retainAll(reachable);
        Set<String> reachableRejecting = new LinkedHashSet<>(reachable);
        reachableRejecting.removeAll(reachableAccepting);
        List<Set<String>> partitions = new ArrayList<>();
        if (!reachableAccepting.isEmpty()) {
            partitions.add(Set.copyOf(reachableAccepting));
        }
        if (!reachableRejecting.isEmpty()) {
            partitions.add(Set.copyOf(reachableRejecting));
        }

        boolean changed;
        do {
            changed = false;
            Map<String, Integer> partitionByState = indexPartitions(partitions);
            List<Set<String>> refined = new ArrayList<>();
            for (Set<String> partition : partitions) {
                Map<List<Integer>, Set<String>> groups = new HashMap<>();
                for (String state : partition) {
                    List<Integer> signature = new ArrayList<>();
                    for (String event : new TreeSet<>(machine.alphabet())) {
                        String target = machine.transitions().get(new TransitionKey(state, event));
                        signature.add(target == null ? -1 : partitionByState.get(target));
                    }
                    groups.computeIfAbsent(List.copyOf(signature), ignored -> new LinkedHashSet<>()).add(state);
                }
                refined.addAll(groups.values().stream().map(Set::copyOf).toList());
                changed |= groups.size() > 1;
            }
            partitions = refined;
        } while (changed);

        Map<String, String> minimizedName = new HashMap<>();
        Set<String> minimizedStates = new LinkedHashSet<>();
        Set<String> minimizedAccepting = new LinkedHashSet<>();
        int partitionIndex = 0;
        for (Set<String> partition : partitions) {
            String name = "M" + partitionIndex++;
            minimizedStates.add(name);
            for (String state : partition) {
                minimizedName.put(state, name);
            }
            if (partition.stream().anyMatch(machine.accepting()::contains)) {
                minimizedAccepting.add(name);
            }
        }
        Map<TransitionKey, String> minimizedTransitions = new HashMap<>();
        for (Set<String> partition : partitions) {
            String representative = partition.iterator().next();
            String source = minimizedName.get(representative);
            for (String event : machine.alphabet()) {
                String target = machine.transitions().get(new TransitionKey(representative, event));
                if (target != null) {
                    minimizedTransitions.put(new TransitionKey(source, event), minimizedName.get(target));
                }
            }
        }
        return new Dfa(
                minimizedStates,
                machine.alphabet(),
                minimizedTransitions,
                minimizedName.get(machine.initial()),
                minimizedAccepting);
    }

    private static Map<String, Integer> indexPartitions(List<Set<String>> partitions) {
        Map<String, Integer> result = new HashMap<>();
        for (int index = 0; index < partitions.size(); index++) {
            for (String state : partitions.get(index)) {
                result.put(state, index);
            }
        }
        return result;
    }
}
