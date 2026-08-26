package com.codingadventures.statemachine;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Queue;
import java.util.Set;
import java.util.TreeSet;

/** A deterministic finite automaton with trace, validation, and rendering helpers. */
public final class Dfa {
    public static final int DEFAULT_MAX_TRACE_ENTRIES = 100_000;

    private final Set<String> states;
    private final Set<String> alphabet;
    private final Map<TransitionKey, String> transitions;
    private final String initial;
    private final Set<String> accepting;
    private final Map<TransitionKey, TransitionAction> actions;
    private final int maxTraceEntries;
    private final List<TransitionRecord> trace = new ArrayList<>();
    private String currentState;

    public Dfa(
            Set<String> states,
            Set<String> alphabet,
            Map<TransitionKey, String> transitions,
            String initial,
            Set<String> accepting) {
        this(states, alphabet, transitions, initial, accepting, Map.of());
    }

    public Dfa(
            Set<String> states,
            Set<String> alphabet,
            Map<TransitionKey, String> transitions,
            String initial,
            Set<String> accepting,
            Map<TransitionKey, TransitionAction> actions) {
        this(states, alphabet, transitions, initial, accepting, actions, DEFAULT_MAX_TRACE_ENTRIES);
    }

    public Dfa(
            Set<String> states,
            Set<String> alphabet,
            Map<TransitionKey, String> transitions,
            String initial,
            Set<String> accepting,
            Map<TransitionKey, TransitionAction> actions,
            int maxTraceEntries) {
        if (states == null || states.isEmpty()) {
            throw new IllegalArgumentException("DFA requires at least one state");
        }
        this.states = Set.copyOf(states);
        this.alphabet = Set.copyOf(alphabet);
        this.transitions = Map.copyOf(transitions);
        this.accepting = Set.copyOf(accepting);
        this.actions = Map.copyOf(actions);
        if (maxTraceEntries < 0) {
            throw new IllegalArgumentException("maximum trace entries must be non-negative");
        }
        this.maxTraceEntries = maxTraceEntries;
        this.initial = initial;
        if (!this.states.contains(initial)) {
            throw new IllegalArgumentException("initial state is not declared");
        }
        if (!this.states.containsAll(this.accepting)) {
            throw new IllegalArgumentException("accepting states must be declared");
        }
        validateTransitionDefinitions();
        currentState = initial;
    }

    public String process(String event) {
        if (!alphabet.contains(event)) {
            throw new IllegalArgumentException("event is not in the DFA alphabet");
        }
        String source = currentState;
        TransitionKey key = new TransitionKey(source, event);
        String target = transitions.get(key);
        if (target == null) {
            throw new IllegalStateException("no transition for current state and event");
        }
        if (trace.size() >= maxTraceEntries) {
            throw new IllegalStateException("DFA trace limit exceeded");
        }
        TransitionAction action = actions.get(key);
        currentState = target;
        if (action != null) {
            action.effect().apply(source, event, target);
        }
        trace.add(new TransitionRecord(source, event, target, action == null ? null : action.name()));
        return target;
    }

    public List<TransitionRecord> processSequence(List<String> events) {
        int start = trace.size();
        for (String event : events) {
            process(event);
        }
        return List.copyOf(trace.subList(start, trace.size()));
    }

    public boolean accepts(List<String> events) {
        String state = initial;
        for (String event : events) {
            if (!alphabet.contains(event)) {
                return false;
            }
            state = transitions.get(new TransitionKey(state, event));
            if (state == null) {
                return false;
            }
        }
        return accepting.contains(state);
    }

    public void reset() {
        currentState = initial;
        trace.clear();
    }

    public boolean isAccepting() {
        return accepting.contains(currentState);
    }

    public boolean isComplete() {
        for (String state : states) {
            for (String event : alphabet) {
                if (!transitions.containsKey(new TransitionKey(state, event))) {
                    return false;
                }
            }
        }
        return true;
    }

    public Set<String> reachableStates() {
        Set<String> reached = new LinkedHashSet<>();
        Queue<String> queue = new ArrayDeque<>();
        reached.add(initial);
        queue.add(initial);
        while (!queue.isEmpty()) {
            String state = queue.remove();
            for (String event : alphabet) {
                String target = transitions.get(new TransitionKey(state, event));
                if (target != null && reached.add(target)) {
                    queue.add(target);
                }
            }
        }
        return Set.copyOf(reached);
    }

    public List<String> validate() {
        List<String> problems = new ArrayList<>();
        Set<String> reachable = reachableStates();
        for (String state : sorted(states)) {
            if (!reachable.contains(state)) {
                problems.add("unreachable state: " + state);
            }
            for (String event : sorted(alphabet)) {
                if (!transitions.containsKey(new TransitionKey(state, event))) {
                    problems.add("missing transition: " + state + " / " + event);
                }
            }
        }
        return List.copyOf(problems);
    }

    public List<List<String>> toTable() {
        List<String> events = sorted(alphabet);
        List<List<String>> table = new ArrayList<>();
        List<String> header = new ArrayList<>();
        header.add("State");
        header.addAll(events);
        table.add(List.copyOf(header));
        for (String state : sorted(states)) {
            List<String> row = new ArrayList<>();
            row.add(state);
            for (String event : events) {
                row.add(transitions.getOrDefault(new TransitionKey(state, event), ""));
            }
            table.add(List.copyOf(row));
        }
        return List.copyOf(table);
    }

    public String toAscii() {
        StringBuilder builder = new StringBuilder();
        for (List<String> row : toTable()) {
            builder.append(String.join(" | ", row)).append('\n');
        }
        return builder.toString();
    }

    public String toDot() {
        StringBuilder builder = new StringBuilder("digraph DFA {\n  rankdir=LR;\n");
        builder.append("  node [shape=doublecircle];");
        for (String state : sorted(accepting)) {
            builder.append(" \"").append(escape(state)).append("\"");
        }
        builder.append(";\n  node [shape=circle];\n  __start [shape=point];\n  __start -> \"")
                .append(escape(initial)).append("\";\n");
        transitions.entrySet().stream()
                .sorted(Map.Entry.comparingByKey(Comparator.comparing(TransitionKey::state)
                        .thenComparing(TransitionKey::event)))
                .forEach(entry -> builder.append("  \"")
                        .append(escape(entry.getKey().state())).append("\" -> \"")
                        .append(escape(entry.getValue())).append("\" [label=\"")
                        .append(escape(entry.getKey().event())).append("\"];\n"));
        return builder.append("}\n").toString();
    }

    public Set<String> states() {
        return states;
    }

    public Set<String> alphabet() {
        return alphabet;
    }

    public Map<TransitionKey, String> transitions() {
        return transitions;
    }

    public String initial() {
        return initial;
    }

    public Set<String> accepting() {
        return accepting;
    }

    public String currentState() {
        return currentState;
    }

    public List<TransitionRecord> trace() {
        return List.copyOf(trace);
    }

    private void validateTransitionDefinitions() {
        for (Map.Entry<TransitionKey, String> entry : transitions.entrySet()) {
            TransitionKey key = entry.getKey();
            if (!states.contains(key.state()) || !alphabet.contains(key.event()) || !states.contains(entry.getValue())) {
                throw new IllegalArgumentException("transition references an undeclared state or event");
            }
        }
        if (!transitions.keySet().containsAll(actions.keySet())) {
            throw new IllegalArgumentException("actions require matching transitions");
        }
    }

    private static List<String> sorted(Set<String> values) {
        return List.copyOf(new TreeSet<>(values));
    }

    private static String escape(String value) {
        return value.replace("\\", "\\\\").replace("\"", "\\\"");
    }
}
