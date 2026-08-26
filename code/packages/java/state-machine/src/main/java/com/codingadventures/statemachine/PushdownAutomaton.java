package com.codingadventures.statemachine;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;

/** A deterministic pushdown automaton with bounded epsilon closure. */
public final class PushdownAutomaton {
    private static final int EPSILON_STEP_LIMIT = 10_000;
    public static final int DEFAULT_MAX_STACK_DEPTH = 4_096;
    public static final int DEFAULT_MAX_TRACE_ENTRIES = 2_048;

    private final Set<String> states;
    private final Set<String> inputAlphabet;
    private final Set<String> stackAlphabet;
    private final List<PdaTransition> transitions;
    private final Map<PdaKey, PdaTransition> transitionIndex;
    private final String initial;
    private final String initialStackSymbol;
    private final Set<String> accepting;
    private final int maxStackDepth;
    private final int maxTraceEntries;
    private final List<PdaTraceEntry> trace = new ArrayList<>();
    private final List<String> stack = new ArrayList<>();
    private String currentState;

    public PushdownAutomaton(
            Set<String> states,
            Set<String> inputAlphabet,
            Set<String> stackAlphabet,
            List<PdaTransition> transitions,
            String initial,
            String initialStackSymbol,
            Set<String> accepting) {
        this(
                states,
                inputAlphabet,
                stackAlphabet,
                transitions,
                initial,
                initialStackSymbol,
                accepting,
                DEFAULT_MAX_STACK_DEPTH,
                DEFAULT_MAX_TRACE_ENTRIES);
    }

    public PushdownAutomaton(
            Set<String> states,
            Set<String> inputAlphabet,
            Set<String> stackAlphabet,
            List<PdaTransition> transitions,
            String initial,
            String initialStackSymbol,
            Set<String> accepting,
            int maxStackDepth,
            int maxTraceEntries) {
        if (states == null || states.isEmpty()) {
            throw new IllegalArgumentException("PDA requires at least one state");
        }
        this.states = Set.copyOf(states);
        this.inputAlphabet = Set.copyOf(inputAlphabet);
        this.stackAlphabet = Set.copyOf(stackAlphabet);
        this.transitions = List.copyOf(transitions);
        this.initial = initial;
        this.initialStackSymbol = initialStackSymbol;
        this.accepting = Set.copyOf(accepting);
        if (maxStackDepth < 1 || maxTraceEntries < 0) {
            throw new IllegalArgumentException("PDA stack and trace limits are invalid");
        }
        this.maxStackDepth = maxStackDepth;
        this.maxTraceEntries = maxTraceEntries;
        if (!this.states.contains(initial)
                || !this.states.containsAll(this.accepting)
                || !this.stackAlphabet.contains(initialStackSymbol)) {
            throw new IllegalArgumentException("initial and accepting definitions must be declared");
        }
        Map<PdaKey, PdaTransition> indexed = new HashMap<>();
        for (PdaTransition transition : this.transitions) {
            validateTransition(transition);
            PdaKey key = new PdaKey(transition.source(), transition.event(), transition.stackRead());
            if (indexed.put(key, transition) != null) {
                throw new IllegalArgumentException("PDA transitions must be deterministic");
            }
        }
        transitionIndex = Map.copyOf(indexed);
        reset();
    }

    public String process(String input) {
        if (!inputAlphabet.contains(input)) {
            throw new IllegalArgumentException("input is not in the PDA alphabet");
        }
        if (stack.isEmpty()) {
            throw new IllegalStateException("PDA stack is empty");
        }
        PdaTransition transition = transitionIndex.get(new PdaKey(currentState, input, top()));
        if (transition == null) {
            throw new IllegalStateException("no PDA transition for current configuration");
        }
        apply(transition);
        return currentState;
    }

    public List<PdaTraceEntry> processSequence(List<String> inputs) {
        int start = trace.size();
        for (String input : inputs) {
            process(input);
        }
        closeEpsilon();
        return List.copyOf(trace.subList(start, trace.size()));
    }

    public boolean accepts(List<String> inputs) {
        String state = initial;
        List<String> localStack = new ArrayList<>();
        localStack.add(initialStackSymbol);
        try {
            Configuration configuration;
            for (String input : inputs) {
                if (!inputAlphabet.contains(input) || localStack.isEmpty()) {
                    return false;
                }
                PdaTransition transition = transitionIndex.get(
                        new PdaKey(state, input, localStack.get(localStack.size() - 1)));
                if (transition == null) {
                    return false;
                }
                state = applyLocal(transition, localStack);
            }
            configuration = closeEpsilon(state, localStack);
            return accepting.contains(configuration.state());
        } catch (IllegalStateException exception) {
            return false;
        }
    }

    public void reset() {
        currentState = initial;
        stack.clear();
        stack.add(initialStackSymbol);
        trace.clear();
    }

    public String currentState() {
        return currentState;
    }

    public List<String> stack() {
        return List.copyOf(stack);
    }

    public List<PdaTraceEntry> trace() {
        return List.copyOf(trace);
    }

    private void closeEpsilon() {
        int steps = 0;
        while (!stack.isEmpty()) {
            if (steps++ >= EPSILON_STEP_LIMIT) {
                throw new IllegalStateException("PDA epsilon cycle detected");
            }
            PdaTransition transition = transitionIndex.get(new PdaKey(currentState, null, top()));
            if (transition == null) {
                return;
            }
            apply(transition);
        }
    }

    private Configuration closeEpsilon(String state, List<String> localStack) {
        int steps = 0;
        while (!localStack.isEmpty()) {
            if (steps++ >= EPSILON_STEP_LIMIT) {
                throw new IllegalStateException("PDA epsilon cycle detected");
            }
            PdaTransition transition = transitionIndex.get(
                    new PdaKey(state, null, localStack.get(localStack.size() - 1)));
            if (transition == null) {
                return new Configuration(state, List.copyOf(localStack));
            }
            state = applyLocal(transition, localStack);
        }
        return new Configuration(state, List.copyOf(localStack));
    }

    private void apply(PdaTransition transition) {
        if (trace.size() >= maxTraceEntries) {
            throw new IllegalStateException("PDA trace limit exceeded");
        }
        String source = currentState;
        String popped = top();
        currentState = applyLocal(transition, stack);
        trace.add(new PdaTraceEntry(
                source,
                transition.event(),
                popped,
                currentState,
                transition.stackPush(),
                stack));
    }

    private String applyLocal(PdaTransition transition, List<String> targetStack) {
        int resultingDepth = targetStack.size() - 1 + transition.stackPush().size();
        if (resultingDepth > maxStackDepth) {
            throw new IllegalStateException("PDA stack limit exceeded");
        }
        targetStack.remove(targetStack.size() - 1);
        targetStack.addAll(transition.stackPush());
        return transition.target();
    }

    private String top() {
        return stack.get(stack.size() - 1);
    }

    private void validateTransition(PdaTransition transition) {
        if (!states.contains(transition.source())
                || !states.contains(transition.target())
                || (transition.event() != null && !inputAlphabet.contains(transition.event()))
                || !stackAlphabet.contains(transition.stackRead())
                || !stackAlphabet.containsAll(transition.stackPush())) {
            throw new IllegalArgumentException("PDA transition references an undeclared symbol or state");
        }
    }

    private record PdaKey(String state, String input, String stackTop) {}
    private record Configuration(String state, List<String> stack) {}
}
