package com.codingadventures.statemachine;

/** Side effect invoked after a deterministic transition is selected. */
@FunctionalInterface
public interface TransitionEffect {
    void apply(String source, String event, String target);
}
