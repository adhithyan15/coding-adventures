package com.codingadventures.statemachine;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Queue;
import java.util.Set;
import java.util.TreeSet;

/** A nondeterministic finite automaton with epsilon closure and subset conversion. */
public final class Nfa {
    public static final String EPSILON = "";
    public static final int DEFAULT_MAX_DFA_STATES = 4_096;
    public static final int DEFAULT_MAX_TRACE_ENTRIES = 100_000;
    public static final long DEFAULT_MAX_TRACE_STATE_CELLS = 1_000_000;

    private final Set<String> states;
    private final Set<String> alphabet;
    private final Map<TransitionKey, Set<String>> transitions;
    private final String initial;
    private final Set<String> accepting;
    private final int maxDfaStates;
    private final int maxTraceEntries;
    private final long maxTraceStateCells;
    private final List<NfaTraceEntry> trace = new ArrayList<>();
    private Set<String> currentStates;
    private long traceStateCells;

    public Nfa(
            Set<String> states,
            Set<String> alphabet,
            Map<TransitionKey, Set<String>> transitions,
            String initial,
            Set<String> accepting) {
        this(
                states,
                alphabet,
                transitions,
                initial,
                accepting,
                DEFAULT_MAX_DFA_STATES,
                DEFAULT_MAX_TRACE_ENTRIES,
                DEFAULT_MAX_TRACE_STATE_CELLS);
    }

    public Nfa(
            Set<String> states,
            Set<String> alphabet,
            Map<TransitionKey, Set<String>> transitions,
            String initial,
            Set<String> accepting,
            int maxDfaStates,
            int maxTraceEntries) {
        this(
                states,
                alphabet,
                transitions,
                initial,
                accepting,
                maxDfaStates,
                maxTraceEntries,
                DEFAULT_MAX_TRACE_STATE_CELLS);
    }

    public Nfa(
            Set<String> states,
            Set<String> alphabet,
            Map<TransitionKey, Set<String>> transitions,
            String initial,
            Set<String> accepting,
            int maxDfaStates,
            int maxTraceEntries,
            long maxTraceStateCells) {
        if (states == null || states.isEmpty()) {
            throw new IllegalArgumentException("NFA requires at least one state");
        }
        if (alphabet.contains(EPSILON)) {
            throw new IllegalArgumentException("epsilon must not be part of the input alphabet");
        }
        this.states = Set.copyOf(states);
        this.alphabet = Set.copyOf(alphabet);
        Map<TransitionKey, Set<String>> copied = new HashMap<>();
        transitions.forEach((key, targets) -> copied.put(key, Set.copyOf(targets)));
        this.transitions = Map.copyOf(copied);
        this.initial = initial;
        this.accepting = Set.copyOf(accepting);
        if (maxDfaStates < 1 || maxTraceEntries < 0 || maxTraceStateCells < 0) {
            throw new IllegalArgumentException("NFA state and trace limits are invalid");
        }
        this.maxDfaStates = maxDfaStates;
        this.maxTraceEntries = maxTraceEntries;
        this.maxTraceStateCells = maxTraceStateCells;
        if (!this.states.contains(initial) || !this.states.containsAll(this.accepting)) {
            throw new IllegalArgumentException("initial and accepting states must be declared");
        }
        validateDefinitions();
        currentStates = epsilonClosure(Set.of(initial));
    }

    public Set<String> epsilonClosure(Set<String> seeds) {
        if (!states.containsAll(seeds)) {
            throw new IllegalArgumentException("epsilon-closure seed is not declared");
        }
        Set<String> closure = new LinkedHashSet<>(seeds);
        Queue<String> queue = new ArrayDeque<>(seeds);
        while (!queue.isEmpty()) {
            String state = queue.remove();
            for (String target : transitions.getOrDefault(new TransitionKey(state, EPSILON), Set.of())) {
                if (closure.add(target)) {
                    queue.add(target);
                }
            }
        }
        return Set.copyOf(closure);
    }

    public Set<String> process(String event) {
        if (!alphabet.contains(event)) {
            throw new IllegalArgumentException("event is not in the NFA alphabet");
        }
        Set<String> source = currentStates;
        if (trace.size() >= maxTraceEntries) {
            throw new IllegalStateException("NFA trace limit exceeded");
        }
        Set<String> moved = new LinkedHashSet<>();
        for (String state : source) {
            moved.addAll(transitions.getOrDefault(new TransitionKey(state, event), Set.of()));
        }
        Set<String> target = epsilonClosure(moved);
        long stateCells = (long) source.size() + target.size();
        if (stateCells > maxTraceStateCells - traceStateCells) {
            throw new IllegalStateException("NFA trace state-cell limit exceeded");
        }
        currentStates = target;
        trace.add(new NfaTraceEntry(source, event, target));
        traceStateCells += stateCells;
        return currentStates;
    }

    public List<NfaTraceEntry> processSequence(List<String> events) {
        int start = trace.size();
        for (String event : events) {
            process(event);
        }
        return List.copyOf(trace.subList(start, trace.size()));
    }

    public boolean accepts(List<String> events) {
        Set<String> active = epsilonClosure(Set.of(initial));
        for (String event : events) {
            if (!alphabet.contains(event)) {
                return false;
            }
            Set<String> moved = new LinkedHashSet<>();
            for (String state : active) {
                moved.addAll(transitions.getOrDefault(new TransitionKey(state, event), Set.of()));
            }
            active = epsilonClosure(moved);
        }
        return active.stream().anyMatch(accepting::contains);
    }

    public void reset() {
        currentStates = epsilonClosure(Set.of(initial));
        trace.clear();
        traceStateCells = 0;
    }

    public boolean isAccepting() {
        return currentStates.stream().anyMatch(accepting::contains);
    }

    public Dfa toDfa() {
        Set<String> startSet = epsilonClosure(Set.of(initial));
        Map<Set<String>, String> names = new HashMap<>();
        names.put(startSet, "S0");
        Queue<Set<String>> queue = new ArrayDeque<>();
        queue.add(startSet);
        Set<String> dfaStates = new LinkedHashSet<>();
        Set<String> dfaAccepting = new LinkedHashSet<>();
        Map<TransitionKey, String> dfaTransitions = new HashMap<>();
        while (!queue.isEmpty()) {
            Set<String> active = queue.remove();
            String sourceName = names.get(active);
            dfaStates.add(sourceName);
            if (active.stream().anyMatch(accepting::contains)) {
                dfaAccepting.add(sourceName);
            }
            for (String event : alphabet) {
                Set<String> moved = new LinkedHashSet<>();
                for (String state : active) {
                    moved.addAll(transitions.getOrDefault(new TransitionKey(state, event), Set.of()));
                }
                Set<String> target = epsilonClosure(moved);
                String targetName = names.get(target);
                if (targetName == null) {
                    if (names.size() >= maxDfaStates) {
                        throw new IllegalStateException("NFA subset construction exceeds configured limit");
                    }
                    targetName = "S" + names.size();
                    names.put(target, targetName);
                    queue.add(target);
                }
                dfaTransitions.put(new TransitionKey(sourceName, event), targetName);
            }
        }
        return new Dfa(dfaStates, alphabet, dfaTransitions, names.get(startSet), dfaAccepting);
    }

    public String toDot() {
        StringBuilder builder = new StringBuilder("digraph NFA {\n  rankdir=LR;\n");
        builder.append("  node [shape=doublecircle];");
        for (String state : new TreeSet<>(accepting)) {
            builder.append(" \"").append(escape(state)).append("\"");
        }
        builder.append(";\n  node [shape=circle];\n  __start [shape=point];\n  __start -> \"")
                .append(escape(initial)).append("\";\n");
        transitions.entrySet().stream()
                .sorted(Map.Entry.comparingByKey(Comparator.comparing(TransitionKey::state)
                        .thenComparing(TransitionKey::event)))
                .forEach(entry -> {
                    String label = entry.getKey().event().isEmpty() ? "ε" : entry.getKey().event();
                    for (String target : new TreeSet<>(entry.getValue())) {
                        builder.append("  \"").append(escape(entry.getKey().state()))
                                .append("\" -> \"").append(escape(target))
                                .append("\" [label=\"").append(escape(label)).append("\"];\n");
                    }
                });
        return builder.append("}\n").toString();
    }

    public Set<String> currentStates() {
        return currentStates;
    }

    public List<NfaTraceEntry> trace() {
        return List.copyOf(trace);
    }

    private void validateDefinitions() {
        for (Map.Entry<TransitionKey, Set<String>> entry : transitions.entrySet()) {
            String event = entry.getKey().event();
            if (!states.contains(entry.getKey().state())
                    || (!event.equals(EPSILON) && !alphabet.contains(event))
                    || !states.containsAll(entry.getValue())) {
                throw new IllegalArgumentException("transition references an undeclared state or event");
            }
        }
    }

    private static String escape(String value) {
        return value.replace("\\", "\\\\").replace("\"", "\\\"");
    }
}
