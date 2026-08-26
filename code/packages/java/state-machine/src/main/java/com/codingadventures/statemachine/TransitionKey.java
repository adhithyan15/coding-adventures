package com.codingadventures.statemachine;

/** A state/event lookup key shared by deterministic and nondeterministic machines. */
public record TransitionKey(String state, String event) {
    public TransitionKey {
        if (state == null || event == null) {
            throw new IllegalArgumentException("state and event must be non-null");
        }
    }
}
