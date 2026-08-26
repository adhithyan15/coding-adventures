package com.codingadventures.statemachine;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/** A set of DFAs connected by named mode-switch transitions. */
public final class ModalStateMachine {
    public static final int DEFAULT_MAX_TRACE_ENTRIES = 100_000;

    private final Map<String, Dfa> modes;
    private final Map<ModeTransitionKey, String> modeTransitions;
    private final String initialMode;
    private final int maxTraceEntries;
    private final List<ModeTransitionRecord> modeTrace = new ArrayList<>();
    private String currentMode;

    public ModalStateMachine(
            Map<String, Dfa> modes,
            Map<ModeTransitionKey, String> modeTransitions,
            String initialMode) {
        this(modes, modeTransitions, initialMode, DEFAULT_MAX_TRACE_ENTRIES);
    }

    public ModalStateMachine(
            Map<String, Dfa> modes,
            Map<ModeTransitionKey, String> modeTransitions,
            String initialMode,
            int maxTraceEntries) {
        if (modes == null || modes.isEmpty()) {
            throw new IllegalArgumentException("modal machine requires at least one mode");
        }
        this.modes = Map.copyOf(modes);
        this.modeTransitions = Map.copyOf(modeTransitions);
        this.initialMode = initialMode;
        if (maxTraceEntries < 0) {
            throw new IllegalArgumentException("maximum trace entries must be non-negative");
        }
        this.maxTraceEntries = maxTraceEntries;
        if (!this.modes.containsKey(initialMode)) {
            throw new IllegalArgumentException("initial mode is not declared");
        }
        for (Map.Entry<ModeTransitionKey, String> entry : this.modeTransitions.entrySet()) {
            if (!this.modes.containsKey(entry.getKey().mode()) || !this.modes.containsKey(entry.getValue())) {
                throw new IllegalArgumentException("mode transition references an undeclared mode");
            }
        }
        reset();
    }

    public String process(String event) {
        return modes.get(currentMode).process(event);
    }

    public String switchMode(String trigger) {
        String source = currentMode;
        String target = modeTransitions.get(new ModeTransitionKey(source, trigger));
        if (target == null) {
            throw new IllegalArgumentException("unknown mode trigger for current mode");
        }
        if (modeTrace.size() >= maxTraceEntries) {
            throw new IllegalStateException("modal trace limit exceeded");
        }
        modes.get(target).reset();
        currentMode = target;
        modeTrace.add(new ModeTransitionRecord(source, trigger, target));
        return target;
    }

    public void reset() {
        modes.values().forEach(Dfa::reset);
        currentMode = initialMode;
        modeTrace.clear();
    }

    public String currentMode() {
        return currentMode;
    }

    public Dfa activeMachine() {
        return modes.get(currentMode);
    }

    public List<ModeTransitionRecord> modeTrace() {
        return List.copyOf(modeTrace);
    }
}
